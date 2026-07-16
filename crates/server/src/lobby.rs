//! Create a table, share its id, claim seats + pick decks, ready-up, host starts. State is
//! polled via `GET /tables/{table}/lobby/v1` until `started`; then clients connect the game stream.

use std::sync::atomic::Ordering;

use crate::db::Deck;
use crate::decks::{SeatDeck, Table, seed_game};
use crate::{AppState, auth::AuthUser, legality, lock, precons};
use axum::http::StatusCode;
use axum::{Json, extract::Path, extract::State};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use engine::PlayerId;
use rand::RngCore;
use rand::rngs::OsRng;
use schema::{
    CreateTableResponse, DeckCardEntry, JoinRequest, LobbyView, ReadyRequest, SeatView,
    StartRequest,
};

/// Host-only sticky cookie: pins the browser to the instance that owns the in-memory table.
const AFFINITY_COOKIE: &str = "mtgfr-instance";

fn affinity_cookie(settings: &crate::settings::Settings) -> Cookie<'static> {
    Cookie::build((AFFINITY_COOKIE, settings.instance_id.clone()))
        .http_only(true)
        .secure(settings.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}

/// Create a lobby table. 503 while draining — new tables must land elsewhere.
#[utoipa::path(
    post,
    path = "/tables/v1",
    responses(
        (status = 200, description = "New table id", body = CreateTableResponse),
        (status = 503, description = "Instance draining — retry against another instance"),
    ),
)]
pub async fn create_table(
    State(state): State<AppState>,
    _user: AuthUser,
    jar: CookieJar,
) -> Result<(CookieJar, Json<CreateTableResponse>), StatusCode> {
    if state.draining.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut reg = lock(&state.reg);
    // Mint a fresh code, re-rolling on the (astronomically unlikely) collision with a live table.
    let mut table_id = random_code();
    while reg.tables.contains_key(&table_id) {
        table_id = random_code();
    }
    reg.tables.insert(table_id.clone(), Table::new_lobby());
    drop(reg);
    crate::action_log::start(&table_id); // outside the lock — blocking disk I/O
    let jar = jar.add(affinity_cookie(&state.settings));
    Ok((jar, Json(CreateTableResponse { table_id })))
}

/// A short, human-friendly table code the host can read aloud — 6 chars from an unambiguous
/// alphabet (no 0/O/1/I/L look-alikes). Not a security token: joining needs a signed-in session and
/// tables are ephemeral, so ~30 bits is plenty against casual enumeration.
fn random_code() -> String {
    // ponytail: 256 % 31 leaves a tiny modulo bias toward the first chars — irrelevant for a code.
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ"; // 31 chars
    let mut bytes = [0u8; 6];
    OsRng.fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

/// Claim the next open seat with one of your saved decks (idempotent — a re-join just updates
/// the seat's deck). The first joiner becomes the host. Identity is the session cookie. On a
/// successful claim/re-join, also sets the affinity cookie (join is a hop that could otherwise
/// land on a different instance than the one holding the table).
#[utoipa::path(post, path = "/tables/join/v1", request_body = JoinRequest, responses((status = 200, description = "Lobby state", body = LobbyView)))]
pub async fn join_table(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
    Json(req): Json<JoinRequest>,
) -> (CookieJar, Json<LobbyView>) {
    let uid = user.0.id;
    // Resolve the deck (and confirm it's theirs) before touching the registry — no DB await is
    // held across the registry lock.
    let deck_name = if let Some(precon) = precons::get(req.deck_id) {
        precon.name.clone() // a precon is everyone's — no ownership check
    } else {
        let mut db = state.db.clone();
        match Deck::filter_by_id(req.deck_id).get(&mut db).await {
            Ok(deck) if deck.user_id == uid => deck.name,
            _ => return (jar, Json(error_lobby(&req.table_id, "UnknownDeck"))),
        }
    };

    let mut reg = lock(&state.reg);
    let Some(table) = reg.tables.get_mut(&req.table_id) else {
        return (jar, Json(error_lobby(&req.table_id, "UnknownTable")));
    };
    if table.game.is_some() {
        return (
            jar,
            Json(lobby_view(
                table,
                &req.table_id,
                Some(uid),
                Some("AlreadyStarted"),
            )),
        );
    }
    table.touch();
    let jar = jar.add(affinity_cookie(&state.settings));
    if let Some(seat) = table.seat_of(uid) {
        let s = &mut table.seats[seat as usize]; // re-join: update deck
        s.deck_id = Some(req.deck_id);
        s.deck_name = Some(deck_name);
        s.username = Some(user.0.username.clone());
        return (jar, Json(lobby_view(table, &req.table_id, Some(uid), None)));
    }
    let Some(open) = table.seats.iter().position(|s| s.user_id.is_none()) else {
        return (
            jar,
            Json(lobby_view(
                table,
                &req.table_id,
                Some(uid),
                Some("TableFull"),
            )),
        );
    };
    table.seats[open].user_id = Some(uid);
    table.seats[open].username = Some(user.0.username.clone());
    table.seats[open].deck_id = Some(req.deck_id);
    table.seats[open].deck_name = Some(deck_name);
    table.host.get_or_insert(uid);
    (jar, Json(lobby_view(table, &req.table_id, Some(uid), None)))
}

/// Toggle a seated player's ready flag.
#[utoipa::path(post, path = "/tables/ready/v1", request_body = ReadyRequest, responses((status = 200, description = "Lobby state", body = LobbyView)))]
pub async fn ready_up(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ReadyRequest>,
) -> Json<LobbyView> {
    let uid = user.0.id;
    let mut reg = lock(&state.reg);
    let Some(table) = reg.tables.get_mut(&req.table_id) else {
        return Json(error_lobby(&req.table_id, "UnknownTable"));
    };
    let Some(seat) = table.seat_of(uid) else {
        return Json(lobby_view(
            table,
            &req.table_id,
            Some(uid),
            Some("NotSeated"),
        ));
    };
    table.touch();
    table.seats[seat as usize].ready = req.ready;
    Json(lobby_view(table, &req.table_id, Some(uid), None))
}

/// The host starts the game once ≥2 seats are claimed and every claimed seat is ready. Each
/// seat's deck is loaded and re-validated for legality, then resolved to cards, before seeding.
#[utoipa::path(post, path = "/tables/start/v1", request_body = StartRequest, responses((status = 200, description = "Lobby state", body = LobbyView)))]
pub async fn start_game(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<StartRequest>,
) -> Json<LobbyView> {
    let uid = user.0.id;

    // Phase 1 (locked): check start conditions and collect each seat's (player, deck_id).
    let seat_decks: Vec<(PlayerId, i64)> = {
        let mut reg = lock(&state.reg);
        let Some(table) = reg.tables.get_mut(&req.table_id) else {
            return Json(error_lobby(&req.table_id, "UnknownTable"));
        };
        if let Some(e) = start_error(table, uid) {
            return Json(lobby_view(table, &req.table_id, Some(uid), Some(e)));
        }
        table.touch();
        table
            .seats
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.deck_id.map(|id| (PlayerId(i as u8), id)))
            .collect()
    };

    // Phase 2 (no lock): load, legality-check, and resolve each deck to cards.
    let mut resolved: Vec<(PlayerId, SeatDeck)> = Vec::new();
    for (player, deck_id) in seat_decks {
        match resolve_deck(&state, deck_id).await {
            Ok(seat_deck) => resolved.push((player, seat_deck)),
            Err(e) => return Json(lobby_error(&state, &req.table_id, uid, e)),
        }
    }

    // Phase 3 (locked): seed the game. Live games are in-memory only — nothing is persisted, so
    // a server restart loses running games.
    let mut reg = lock(&state.reg);
    let Some(table) = reg.tables.get_mut(&req.table_id) else {
        return Json(error_lobby(&req.table_id, "UnknownTable"));
    };
    if table.game.is_some() {
        return Json(lobby_view(
            table,
            &req.table_id,
            Some(uid),
            Some("AlreadyStarted"),
        ));
    }
    // H3: seed each game from the OS CSPRNG so libraries aren't reproducible offline (the pool
    // is five *published* decklists). Record the seed on the table so a replay reproduces the
    // exact shuffle.
    let seed = OsRng.next_u64();
    table.seed = seed;
    for (player, deck) in &resolved {
        table.prints[player.0 as usize] = deck.prints.clone();
    }
    table.game = Some(seed_game(&resolved, seed));
    Json(lobby_view(table, &req.table_id, Some(uid), None))
}

/// Load a stored deck and resolve it to its commander + cards, re-checking legality.
async fn resolve_deck(state: &AppState, deck_id: i64) -> Result<SeatDeck, &'static str> {
    let (commander_id, commander_print, entries): (String, String, Vec<DeckCardEntry>) =
        if let Some(precon) = precons::get(deck_id) {
            (
                precon.commander.clone(),
                precon.commander_print.clone(),
                precon.cards.clone(),
            )
        } else {
            let mut db = state.db.clone();
            let deck = Deck::filter_by_id(deck_id)
                .get(&mut db)
                .await
                .map_err(|_| "UnknownDeck")?;
            let entries = serde_json::from_str(&deck.cards).map_err(|_| "CorruptDeck")?;
            (deck.commander, deck.commander_print, entries)
        };
    legality::validate(&commander_id, &entries).map_err(|_| "IllegalDeck")?;
    let commander = cards::get(&commander_id).ok_or("UnknownCard")?;
    let mut prints = std::collections::HashMap::new();
    prints.insert(commander.id.to_string(), commander_print);
    let mut cards = Vec::with_capacity(entries.len());
    for e in &entries {
        let def = cards::get(&e.id).ok_or("UnknownCard")?;
        prints.insert(def.id.to_string(), e.print.clone());
        cards.push((def, e.count as usize));
    }
    Ok(SeatDeck {
        commander,
        cards,
        prints,
    })
}

/// A lobby view carrying an error, re-reading the table under the lock (used from async paths).
fn lobby_error(state: &AppState, table_id: &str, uid: i64, error: &str) -> LobbyView {
    let reg = lock(&state.reg);
    match reg.tables.get(table_id) {
        Some(table) => lobby_view(table, table_id, Some(uid), Some(error)),
        None => error_lobby(table_id, error),
    }
}

/// Current lobby state (client polls this until `started`).
#[utoipa::path(get, path = "/tables/{table}/lobby/v1", params(("table" = String, Path, description = "table id")), responses((status = 200, description = "Lobby state", body = LobbyView)))]
pub async fn lobby_state(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table): Path<String>,
) -> Json<LobbyView> {
    let reg = lock(&state.reg);
    match reg.tables.get(&table) {
        Some(t) => Json(lobby_view(t, &table, Some(user.0.id), None)),
        None => Json(error_lobby(&table, "UnknownTable")),
    }
}

/// Why the host can't start yet, if anything.
fn start_error(table: &Table, user_id: i64) -> Option<&'static str> {
    if table.host != Some(user_id) {
        return Some("NotHost");
    }
    if table.game.is_some() {
        return Some("AlreadyStarted");
    }
    if table.claimed_count() < 2 {
        return Some("NeedTwoPlayers");
    }
    if !table
        .seats
        .iter()
        .filter(|s| s.user_id.is_some())
        .all(|s| s.ready)
    {
        return Some("NotAllReady");
    }
    None
}

/// Build the lobby view for a caller (whose user id marks their seat), with an optional error.
fn lobby_view(
    table: &Table,
    table_id: &str,
    user_id: Option<i64>,
    error: Option<&str>,
) -> LobbyView {
    let you = user_id.and_then(|id| table.seat_of(id));
    let seats = table
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| SeatView {
            player: i as u8,
            claimed: s.user_id.is_some(),
            username: s.username.clone(),
            deck_name: s.deck_name.clone(),
            ready: s.ready,
            is_host: s.user_id.is_some() && s.user_id == table.host,
            is_you: Some(i as u8) == you,
        })
        .collect();
    LobbyView {
        table_id: table_id.to_string(),
        seats,
        you,
        started: table.game.is_some(),
        // An unseated caller (no token seat) can't start either; say so with the same vocabulary.
        start_error: user_id
            .map_or(Some("NotSeated"), |id| start_error(table, id))
            .map(str::to_string),
        error: error.map(str::to_string),
    }
}

/// A lobby view for an unknown/absent table — only its error is meaningful.
fn error_lobby(table_id: &str, error: &str) -> LobbyView {
    LobbyView {
        table_id: table_id.to_string(),
        seats: Vec::new(),
        you: None,
        started: false,
        start_error: Some("UnknownTable".to_string()),
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::test_support::{as_user, user_with_deck};
    use schema::{IntentEnvelope, WireIntent};

    #[tokio::test]
    async fn lobby_seats_players_and_rejects_intents_from_the_wrong_user() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));

        let host_deck = user_with_deck(&state, "host@x.c").await;
        let guest_deck = user_with_deck(&state, "guest@x.c").await;

        // Host creates a table and joins; a second player joins.
        let table_id = create_table(
            State(state.clone()),
            as_user(&state, "host@x.c").await,
            CookieJar::new(),
        )
        .await
        .expect("draining is off")
        .1
        .0
        .table_id;
        let host = join_table(
            State(state.clone()),
            as_user(&state, "host@x.c").await,
            CookieJar::new(),
            Json(JoinRequest {
                table_id: table_id.clone(),
                deck_id: host_deck,
            }),
        )
        .await
        .1
        .0;
        assert_eq!(host.you, Some(0), "host takes seat 0");
        assert!(host.seats[0].is_host);
        assert_eq!(host.seats[0].deck_name.as_deref(), Some("deck"));

        let guest = join_table(
            State(state.clone()),
            as_user(&state, "guest@x.c").await,
            CookieJar::new(),
            Json(JoinRequest {
                table_id: table_id.clone(),
                deck_id: guest_deck,
            }),
        )
        .await
        .1
        .0;
        assert_eq!(guest.you, Some(1), "guest takes the next seat");

        // Both ready up, host starts.
        for email in ["host@x.c", "guest@x.c"] {
            let _ = ready_up(
                State(state.clone()),
                as_user(&state, email).await,
                Json(ReadyRequest {
                    table_id: table_id.clone(),
                    ready: true,
                }),
            )
            .await;
        }
        let started = start_game(
            State(state.clone()),
            as_user(&state, "host@x.c").await,
            Json(StartRequest {
                table_id: table_id.clone(),
            }),
        )
        .await
        .0;
        assert!(started.started, "the host started the game");
        assert!(started.error.is_none());

        let _ = user_with_deck(&state, "noseat@x.c").await;

        // A spectator (signed in but not seated) cannot submit intents.
        let spoof = crate::game_loop::submit_intent(
            State(state.clone()),
            as_user(&state, "noseat@x.c").await,
            Json(IntentEnvelope {
                table_id: table_id.clone(),
                client_seq: 0,
                intent: WireIntent::PassPriority { player: 0 },
            }),
        )
        .await
        .0;
        assert!(!spoof.accepted && spoof.reason.as_deref() == Some("NotSeated"));

        // The wire intent's `player` field is stamped from the session seat.
        let ok = crate::game_loop::submit_intent(
            State(state.clone()),
            as_user(&state, "host@x.c").await,
            Json(IntentEnvelope {
                table_id: table_id.clone(),
                client_seq: 1,
                intent: WireIntent::PassPriority { player: 99 },
            }),
        )
        .await
        .0;
        assert!(ok.accepted, "seat owner's intent is accepted");
    }

    #[tokio::test]
    async fn create_table_is_rejected_with_503_while_draining() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let _ = user_with_deck(&state, "host@x.c").await;
        state
            .draining
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let err = create_table(
            State(state.clone()),
            as_user(&state, "host@x.c").await,
            CookieJar::new(),
        )
        .await
        .expect_err("draining rejects new tables");
        assert_eq!(err, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            lock(&state.reg).tables.is_empty(),
            "no table was created while draining"
        );
    }

    #[test]
    fn create_table_mints_a_short_friendly_code() {
        let a = random_code();
        let b = random_code();
        assert_eq!(a.len(), 6, "codes are 6 chars");
        assert!(
            a.chars()
                .all(|c| "23456789ABCDEFGHJKMNPQRSTUVWXYZ".contains(c)),
            "only unambiguous alphabet chars"
        );
        assert_ne!(a, b, "codes are random");
    }
}
