//! The Axum server: hosts the one authoritative game, streams per-viewer SSE deltas,
//! and accepts intents.
//!
//! Single instance, so fan-out is an in-process `tokio::broadcast` rather than Redis
//! (see ADR 0005). Deltas reach clients only over the stream; the POST response is a bare
//! ack. State lives behind a `std::sync::Mutex` — `Game::submit` is synchronous and fast,
//! and the lock is never held across an `.await`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderName, HeaderValue},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{get, post},
};
use engine::PlayerId;
use schema::{
    ActionView, CatalogCard, ChoiceItem, CombatView, CommanderDamageView, CreateTableResponse,
    DeltaEnvelope, IntentEnvelope, JoinRequest, LobbyView, ModalView, ModeView, ObjectView,
    PendingChoiceView, PlayerView, ReadyRequest, SeatView, SeedRequest, SeedResponse, SeedSeat,
    StackObjectView, StartRequest, StreamFrame, VisibleEvent, VisibleState, WireAttack, WireBlock,
    WireCost, WireDamage, WireIntent, WireKind, WireTarget, catalog_card, complete_visible,
};
use serde::Deserialize;
use utoipa::OpenApi;

mod action_log;
pub mod admin;
pub mod auth;
pub mod catalog_search;
pub mod db;
pub mod decks;
pub mod decks_api;
mod game_loop;
pub mod health;
#[cfg(test)]
mod http_tests;
pub mod legality;
mod lobby;
pub mod precons;
mod session;
pub mod settings;
mod stream;
#[cfg(test)]
pub(crate) mod test_support;
use auth::AuthUser;
use decks::Table;
use game_loop::{Ack, submit_intent};
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

/// How often the stream emits a real `Heartbeat` frame. Comfortably under the client's stale
/// threshold (15s) so a couple can be missed before it gives up and reconnects. Unlike an SSE
/// keepalive *comment*, this is a data event the SSE decoder surfaces — so the client can time out
/// on the absence of *any* frame and catch a silently-dropped upstream (killed backend, no FIN).
const HEARTBEAT_SECS: u64 = 5;

/// One SSE `data:` event carrying a stream frame as JSON. Infallible so it slots into the SSE
/// body stream. The client decodes each event's `data` back into a `StreamFrame`.
fn sse_event(frame: &StreamFrame) -> Result<Event, Infallible> {
    let json = serde_json::to_string(frame).expect("stream frame serializes");
    Ok(Event::default().data(json))
}

/// The per-viewer delta stream, as Server-Sent Events (`text/event-stream`). The first event is a
/// full redacted snapshot at the current seq; every later event is a redacted delta. On (re)connect
/// the client just gets a fresh snapshot, so there's no history buffer — the snapshot's seq is the
/// resume point. SSE (over fetch, not `EventSource`) so the generated client can consume it as a
/// typed `Stream<StreamFrame>` (ADR 0005).
#[utoipa::path(
    get,
    path = "/tables/{table}/stream/v1",
    params(("table" = String, Path, description = "table id")),
    responses((status = 200, description = "SSE StreamFrame events", body = StreamFrame, content_type = "text/event-stream")),
)]
pub async fn stream(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table): Path<String>,
) -> Response {
    use axum::http::StatusCode;

    // Subscribe *before* snapshotting so nothing slips through the gap between the two;
    // deltas already reflected in the snapshot are dropped by the seq check below. C1: the
    // viewer is the user's own seat, resolved here — 404 if the table/game is gone. A seated user
    // streams their own view; any other signed-in user (no seat) streams the public spectator
    // view instead of being rejected (6.3). `viewer` is `None` for a spectator — the redaction
    // path never exposes a hand.
    let subscription = {
        let reg = lock(&state.reg);
        match reg.tables.get(&table) {
            Some(table) if table.game.is_some() => {
                let viewer = table.seat_of(user.0.id).map(PlayerId);
                let game = table.game.as_ref().expect("game present per the guard");
                // Thin wrapper: Table → ViewExtras → complete_visible (same pass as deltas).
                let extras = stream::view_extras(
                    &table.yields,
                    &table.turn_yields,
                    &table.seats,
                    table.stack_hold_remaining_ms(),
                    &table.prints,
                );
                let state = complete_visible(game, viewer, &extras);
                let seats = table.seats.clone();
                let prints = table.prints.clone();
                let hold_ms = table.stack_hold_remaining_ms();
                let broadcast_seq = table.broadcast_seq;
                Ok((
                    table.tx.subscribe(),
                    table.seq,
                    state,
                    viewer,
                    seats,
                    prints,
                    hold_ms,
                    broadcast_seq,
                ))
            }
            _ => Err(StatusCode::NOT_FOUND),
        }
    };
    let (mut rx, snapshot_seq, state_view, viewer, seats, prints, _hold_ms, snapshot_broadcast_seq) =
        match subscription {
            Ok(sub) => sub,
            Err(code) => return code.into_response(),
        };

    let events = async_stream::stream! {
        yield sse_event(&StreamFrame::Snapshot { seq: snapshot_seq, state: state_view });
        // A real data frame every HEARTBEAT_SECS keeps a quiet-but-alive game visibly alive to the
        // client's frame-timeout watchdog. The first tick fires immediately — skip it so we don't
        // double up on the opening snapshot.
        let mut heartbeat =
            tokio::time::interval(std::time::Duration::from_secs(HEARTBEAT_SECS));
        heartbeat.tick().await;
        loop {
            tokio::select! {
                // A recv error (Lagged slow-reader or Closed) ends the response; the client
                // reconnects and re-snapshots.
                msg = rx.recv() => {
                    let Ok(msg) = msg else { break };
                    if !stream::should_deliver(msg.broadcast_seq, snapshot_broadcast_seq) {
                        continue; // already captured in the opening snapshot
                    }
                    // Always stamp the message's remaining (including 0) so a cleared hold
                    // never resurrects a prior countdown while the stack is still non-empty.
                    yield sse_event(&stream::frame_for(
                        viewer,
                        msg.seq,
                        &msg.events,
                        &msg.game,
                        msg.auto_actions.clone(),
                        &msg.yields,
                        &msg.turn_yields,
                        &seats,
                        msg.stack_hold_remaining_ms,
                        &prints,
                    ));
                }
                _ = heartbeat.tick() => {
                    yield sse_event(&StreamFrame::Heartbeat);
                }
            }
        }
    };

    let mut resp = Sse::new(events).into_response();
    // Tell reverse proxies not to buffer the stream (harmless direct).
    resp.headers_mut().insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    resp
}

/// The HTTP application, wired to the shared table.
/// The whole card pool, for the deck builder to browse. Public (no auth) and stateless —
/// the pool is a load-once static registry.
#[utoipa::path(get, path = "/cards/v1", responses((status = 200, description = "The card pool", body = [CatalogCard])))]
pub async fn catalog() -> Json<Vec<CatalogCard>> {
    Json(cards::registry().values().map(catalog_card).collect())
}

/// Query params for `/cards/search`: the single search box `q`, plus paging. All optional — an
/// empty `q` returns the first page of the pool.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Search the pool from the deck builder's single input: cards matching every token of `q` against
/// name, card type, subtype, set, color, and keywords. Public (no auth) — the pool isn't private.
/// A DB error yields an empty page rather than a 500 (the projection is best-effort, ADR 0010).
#[utoipa::path(
    get,
    path = "/cards/search/v1",
    params(
        ("q" = Option<String>, Query, description = "search text (space-separated tokens, all must match)"),
        ("limit" = Option<u32>, Query, description = "max results (default 100, capped)"),
        ("offset" = Option<u32>, Query, description = "results to skip, for paging"),
    ),
    responses((status = 200, description = "Matching pool cards", body = [CatalogCard])),
)]
pub async fn search_cards(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Json<Vec<CatalogCard>> {
    let mut db = state.db.clone();
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let cards = catalog_search::search(&mut db, &params.q, limit, offset)
        .await
        .unwrap_or_default();
    Json(cards)
}

/// Query params for `/cards/lookup`: Card ids as a repeated param (?ids=a&ids=b).
#[derive(Debug, Clone, Deserialize)]
pub struct LookupParams {
    #[serde(default)]
    pub ids: Vec<String>,
}

/// Fetch specific pool cards by Card id — lets the deck builder hydrate a saved decklist and
/// commander without pulling the whole pool. Public and best-effort like `/cards/search/v1`.
#[utoipa::path(
    get,
    path = "/cards/lookup/v1",
    params(("ids" = Vec<String>, Query, description = "Card ids / Scryfall oracle ids (repeated param)")),
    responses((status = 200, description = "The pool cards", body = [CatalogCard])),
)]
pub async fn lookup_cards(
    State(state): State<AppState>,
    // axum-extra's Query: the stock axum extractor can't deserialize repeated params into a Vec.
    axum_extra::extract::Query(params): axum_extra::extract::Query<LookupParams>,
) -> Json<Vec<CatalogCard>> {
    let mut db = state.db.clone();
    Json(
        catalog_search::lookup(&mut db, &params.ids)
            .await
            .unwrap_or_default(),
    )
}

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

pub fn app(state: AppState) -> Router {
    let cors = cors_layer(&state.settings.cors_origin);
    let router = Router::new()
        .route("/tables/seed/v1", post(lobby::seed_table))
        .route("/cards/v1", get(catalog))
        .route("/cards/search/v1", get(search_cards))
        .route("/cards/lookup/v1", get(lookup_cards))
        .route("/auth/signup/v1", post(auth::signup))
        .route("/auth/login/v1", post(auth::login))
        .route("/auth/logout/v1", post(auth::logout))
        .route("/auth/me/v1", get(auth::me))
        .route(
            "/decks/v1",
            post(decks_api::create_deck).get(decks_api::list_decks),
        )
        .route(
            "/decks/{id}/v1",
            get(decks_api::get_deck)
                .put(decks_api::update_deck)
                .delete(decks_api::delete_deck),
        )
        .route("/tables/{table}/intent/v1", post(submit_intent))
        .route("/tables/{table}/yield/v1", post(game_loop::set_yield))
        .route(
            "/tables/{table}/turn-yield/v1",
            post(game_loop::set_turn_yield),
        )
        .route(
            "/tables/{table}/stack-dwell/v1",
            post(game_loop::set_stack_dwell),
        )
        .route("/tables/{table}/stream/v1", get(stream))
        .route("/openapi.json", get(openapi_spec))
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/health/drain", get(health::drain))
        .with_state(state);
    match cors {
        Some(cors) => router.layer(cors),
        None => router,
    }
}

/// Serves the OpenAPI document at the conventional `/openapi.json` path for any spec consumer
/// (schema diffing, external clients). The client no longer generates types from it — its wire
/// types are hand-maintained and it integrates through an Effect service (ADR 0001, superseded).
async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// The OpenAPI document describing the wire protocol. Kept as machine-readable API documentation;
/// the client's TypeScript types are now hand-maintained rather than generated from it.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "mtgfr",
        description = "Wire protocol for the browser Commander game.",
        license(name = "Proprietary"),
    ),
    paths(
        game_loop::submit_intent,
        game_loop::set_yield,
        game_loop::set_turn_yield,
        game_loop::set_stack_dwell,
        stream,
        lobby::seed_table,
        catalog,
        search_cards,
        lookup_cards,
        auth::signup,
        auth::login,
        auth::logout,
        auth::me,
        decks_api::create_deck,
        decks_api::list_decks,
        decks_api::get_deck,
        decks_api::update_deck,
        decks_api::delete_deck,
    ),
    components(schemas(
        IntentEnvelope,
        WireIntent,
        WireTarget,
        WireAttack,
        WireBlock,
        WireDamage,
        DeltaEnvelope,
        VisibleEvent,
        VisibleState,
        PlayerView,
        CommanderDamageView,
        ObjectView,
        WireKind,
        WireCost,
        StackObjectView,
        CombatView,
        PendingChoiceView,
        ChoiceItem,
        ActionView,
        ModalView,
        ModeView,
        StreamFrame,
        Ack,
        game_loop::YieldRequest,
        game_loop::StackDwellRequest,
        CreateTableResponse,
        JoinRequest,
        ReadyRequest,
        StartRequest,
        SeedRequest,
        SeedResponse,
        SeedSeat,
        SeatView,
        LobbyView,
        CatalogCard,
        schema::Credentials,
        schema::SignupCredentials,
        schema::Me,
        schema::DeckCardEntry,
        schema::DeckSummary,
        schema::DeckDetail,
        schema::SaveDeckRequest,
        schema::DeckError,
    ))
)]
pub struct ApiDoc;

/// The OpenAPI document as pretty JSON. `GET /openapi.json` serves this over the wire; kept as
/// a standalone helper for tests and any other in-process consumer.
pub fn openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("OpenAPI document serializes to JSON")
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
    fn openapi_document_exposes_the_endpoints_and_wire_types() {
        let doc = openapi_json();
        for needle in [
            "/tables/{table}/intent",
            "/tables/{table}/stream",
            "IntentEnvelope",
            "DeltaEnvelope",
            "VisibleState",
            "StreamFrame",
        ] {
            assert!(
                doc.contains(needle),
                "the OpenAPI doc must mention {needle}"
            );
        }
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
