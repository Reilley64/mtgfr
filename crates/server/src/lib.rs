//! The server's shared state and health-only Axum `app()` (`/health/*` on :8080).
//! Game/auth/decks/cards live on the tonic gRPC server in [`grpc`].
//!
//! Single instance, so live-game fan-out is an in-process `tokio::broadcast` rather than Redis
//! (see ADR 0005). State lives behind a `std::sync::Mutex` — `Game::submit` is synchronous and
//! fast, and the lock is never held across an `.await`.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// How long a started table may sit with zero `Game.Stream` subscribers before drain treats it
/// as abandoned ("seats vacated" in the deploy PRD). Long enough for a reconnect blip; short
/// enough that ghost tables from closed browsers don't pin Terminating pods for the full grace.
pub const ABANDONED_TABLE_GRACE: Duration = Duration::from_secs(60);

impl Registry {
    /// Live games this instance holds (every table is born already seeded — see
    /// [`decks::Table::seeded`] — so this is simply how many tables are registered).
    pub fn active_table_count(&self) -> usize {
        self.tables.values().filter(|t| t.game.is_some()).count()
    }

    /// Drop started tables that have had no stream subscribers for at least `grace`.
    /// Returns how many were removed. Call from the SIGTERM drain loop.
    ///
    /// `quiet_since = None` means the table had listeners (or just lost them without a sweep
    /// yet) — the first no-listener observation *arms* grace from `now` instead of treating a
    /// stale seed timestamp as the start of quiet.
    pub fn evict_abandoned(&mut self, now: Instant, grace: Duration) -> usize {
        let before = self.tables.len();
        self.tables.retain(|_, table| {
            if table.game.is_none() {
                return true;
            }
            if table.tx.receiver_count() > 0 {
                table.quiet_since = None;
                return true;
            }
            match table.quiet_since {
                None => {
                    table.quiet_since = Some(now);
                    true
                }
                Some(since) => now.saturating_duration_since(since) < grace,
            }
        });
        before - self.tables.len()
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

    /// Drain waits on `active_table_count() == 0`. A seeded game with no stream subscribers is
    /// "seats vacated" (DEPLOYMENT.md) — it must not block SIGTERM forever the way production
    /// Terminating pods did (ghost `active_tables` long after players left).
    #[test]
    fn abandoned_table_with_no_stream_subscribers_is_evicted_for_drain() {
        use std::time::{Duration, Instant};

        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        // Quiet since long before grace — stands in for a table abandoned hours ago.
        table.quiet_since = Some(Instant::now() - Duration::from_secs(120));
        registry.tables.insert("ghost".to_string(), table);
        assert_eq!(registry.active_table_count(), 1);

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 1, "no-listener table past grace is abandoned");
        assert_eq!(
            registry.active_table_count(),
            0,
            "drain can reach zero after eviction"
        );
    }

    #[test]
    fn table_with_a_live_stream_subscriber_survives_drain_eviction() {
        use std::time::{Duration, Instant};

        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        table.quiet_since = Some(Instant::now() - Duration::from_secs(120));
        let _rx = table.tx.subscribe();
        registry.tables.insert("watched".to_string(), table);

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 0, "a live Game.Stream keeps the table for drain");
        assert_eq!(registry.active_table_count(), 1);
    }

    #[test]
    fn abandoned_table_inside_reconnect_grace_is_kept() {
        use std::time::{Duration, Instant};

        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        table.quiet_since = Some(Instant::now() - Duration::from_secs(30));
        registry.tables.insert("blip".to_string(), table);

        let removed = registry.evict_abandoned(Instant::now(), Duration::from_secs(60));
        assert_eq!(removed, 0, "brief disconnects stay within grace");
        assert_eq!(registry.active_table_count(), 1);
    }

    /// Bugbot: a long-lived game whose streams drop before the first drain sweep still has
    /// seed-era `quiet_since` unless subscribe cleared it — the first quiet sweep must *arm*
    /// grace from `now`, not instant-evict off the seed timestamp.
    #[test]
    fn previously_watched_table_gets_grace_from_first_quiet_sweep() {
        use std::time::{Duration, Instant};

        let mut registry = Registry::default();
        let mut table = Table::empty();
        table.game = Some(crate::decks::seed_game(
            &[
                (PlayerId(0), crate::test_support::seat_deck()),
                (PlayerId(1), crate::test_support::seat_deck()),
            ],
            0,
        ));
        // Subscribe cleared the seed quiet mark; streams then dropped with no further sweep.
        table.quiet_since = None;
        registry.tables.insert("played".to_string(), table);

        let grace = Duration::from_secs(60);
        let t0 = Instant::now();
        assert_eq!(
            registry.evict_abandoned(t0, grace),
            0,
            "first quiet sweep arms grace instead of evicting"
        );
        assert_eq!(registry.active_table_count(), 1);
        assert_eq!(
            registry.evict_abandoned(t0 + Duration::from_secs(30), grace),
            0,
            "still inside grace"
        );
        assert_eq!(
            registry.evict_abandoned(t0 + grace, grace),
            1,
            "evicted once grace elapses from the arming sweep"
        );
        assert_eq!(registry.active_table_count(), 0);
    }
}
