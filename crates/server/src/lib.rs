//! The server's shared state and health-only Axum `app()` (`/health/*` on :8080).
//! Game/auth/decks/cards live on the tonic gRPC server in [`grpc`].
//!
//! Single instance, so live-game fan-out is an in-process `tokio::broadcast` rather than Redis
//! (see ADR 0005). State lives behind a `std::sync::Mutex` — `Game::submit` is synchronous and
//! fast, and the lock is never held across an `.await`.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use axum::{Router, http::HeaderValue, routing::get};
#[cfg(test)]
use engine::PlayerId;

mod action_log;
pub mod auth;
pub mod catalog_search;
mod chrome;
pub mod db;
pub mod decks;
pub mod decks_api;
mod game_loop;
pub mod grpc;
pub mod health;
pub mod legality;
mod lobby;
pub mod precons;
mod session;
pub mod settings;
mod stream;
pub mod telemetry;
#[cfg(test)]
pub(crate) mod test_support;
use decks::Table;
use settings::Settings;

/// The registry of live tables, keyed by `table_id`. Single instance, in-process — no Redis
/// (ADR 0005). Ids are random 128-bit hex, so they're unguessable.
#[derive(Default)]
pub struct Registry {
    tables: HashMap<String, Table>,
}

impl Registry {
    /// Live games this instance holds (every table is born already seeded — see
    /// [`decks::Table::seeded`] — so this is simply how many tables are registered).
    pub fn active_table_count(&self) -> usize {
        self.tables.values().filter(|t| t.game.is_some()).count()
    }
}

/// Lock the table registry, tolerating a poisoned mutex. A panic under the lock quarantines
/// just that one table (C3, see `session::TableSession::apply`); it must not brick every other
/// table by leaving every later `lock()` panic on the poison.
pub fn lock(reg: &Mutex<Registry>) -> std::sync::MutexGuard<'_, Registry> {
    reg.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The shared state handed to every request: the in-memory live-game registry plus the
/// durable store (accounts, sessions, decks). `Db` is a cheap-to-clone pool handle, so
/// handlers clone it per request; the registry stays behind a mutex.
#[derive(Clone)]
pub struct AppState {
    pub reg: Arc<Mutex<Registry>>,
    pub db: toasty::Db,
    pub settings: Arc<Settings>,
    /// Live drain flag (startup from `settings.drain`; flipped by SIGTERM).
    pub draining: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(db: toasty::Db, settings: Arc<Settings>) -> AppState {
        let draining = Arc::new(AtomicBool::new(settings.drain));
        AppState {
            reg: Arc::new(Mutex::new(Registry::default())),
            db,
            settings,
            draining,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(db: toasty::Db) -> AppState {
        AppState::new(db, Arc::new(settings::for_test()))
    }
}

/// How often the gRPC `Game.Stream` service emits a real `Heartbeat` frame (`grpc::game_svc`).
/// Comfortably under the client's stale threshold (15s) so a couple can be missed before it
/// gives up and reconnects — a data event the stream decoder surfaces, so the client can time
/// out on the absence of *any* frame and catch a silently-dropped upstream (killed backend, no
/// FIN), not just rely on a transport-level keepalive.
pub(crate) const HEARTBEAT_SECS: u64 = 5;

/// CORS for a single configured origin with credentials. Empty origin → no layer (dev proxy).
fn cors_layer(origin: &str) -> Option<tower_http::cors::CorsLayer> {
    use axum::http::Method;
    use axum::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
    use tower_http::cors::CorsLayer;

    if origin.is_empty() {
        return None;
    }
    // `Settings::load` already validates; fail closed for hand-built Settings in tests.
    let Ok(origin) = origin.parse::<HeaderValue>() else {
        eprintln!("cors_origin {origin:?} is not a valid header value — CORS layer disabled");
        return None;
    };
    Some(
        CorsLayer::new()
            .allow_origin(origin)
            .allow_credentials(true)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([CONTENT_TYPE, ACCEPT, AUTHORIZATION]),
    )
}

/// Health-only Axum app: k8s liveness/readiness/drain probes on 8080.
/// `cors` is applied for parity with `Settings` even though health routes don't need it.
pub fn app(state: AppState) -> Router {
    let cors = cors_layer(&state.settings.cors_origin);
    let router = Router::new()
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/health/drain", get(health::drain))
        .with_state(state);
    match cors {
        Some(cors) => router.layer(cors),
        None => router,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cors_layer_is_none_for_an_empty_origin() {
        assert!(
            cors_layer("").is_none(),
            "same-origin dev needs no CORS layer at all"
        );
    }

    #[test]
    fn cors_layer_is_some_for_a_valid_origin() {
        assert!(cors_layer("https://edh.example.com").is_some());
    }

    #[test]
    fn cors_layer_fails_closed_for_an_origin_that_is_not_a_valid_header_value() {
        // `Settings::load` normally rejects this at startup (settings.rs); this only exercises
        // the defensive fallback for a hand-built `Settings` that skipped that check.
        assert!(
            cors_layer("bad\nvalue").is_none(),
            "an unparseable origin disables CORS rather than panicking"
        );
    }

    #[test]
    fn lock_survives_a_poisoned_registry() {
        // C3: a panic under the lock poisons the mutex; `lock()` must still hand back a usable
        // guard instead of propagating the poison and bricking every later request.
        let reg = Mutex::new(Registry::default());
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = reg.lock().unwrap();
            panic!("engine blew up under the lock");
        }));
        assert!(reg.is_poisoned(), "the panic poisoned the mutex");
        assert!(
            lock(&reg).tables.is_empty(),
            "lock() recovers the guard anyway"
        );
    }

    #[test]
    fn active_table_count_counts_every_seeded_table() {
        let mut registry = Registry::default();
        assert_eq!(
            registry.active_table_count(),
            0,
            "a fresh registry is empty"
        );

        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        registry.tables.insert("live".to_string(), table);
        assert_eq!(
            registry.active_table_count(),
            1,
            "a seeded table counts as active"
        );
    }
}
