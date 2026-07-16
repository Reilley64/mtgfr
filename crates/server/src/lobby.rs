//! Seed a running game on this instance from a lobby the SolidStart BFF already resolved.
//!
//! The pre-game lobby (claiming seats, picking decks, readying up) lives entirely on the BFF
//! side now (`mtgfr_web` Postgres, Drizzle — see ADR notes in the plan). This module's only job
//! is `POST /tables/seed/v1`: given a host, an ordered list of seats (each with the deck they'll
//! play), build the `Table` and seed its `Game` once.

use std::sync::atomic::Ordering;

use crate::db::Deck;
use crate::decks::{SeatDeck, Table, seed_game};
use crate::{AppState, auth::AuthUser, legality, lock, precons};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use engine::PlayerId;
use rand::RngCore;
use rand::rngs::OsRng;
use schema::{DeckCardEntry, SeedRequest, SeedResponse};

/// Seed a running game from a lobby the BFF already resolved (host, seats in seat-index order,
/// each seat's chosen deck). 503 while draining — a new table must land on an instance that will
/// stick around. 400 if `table_id` already names a live table (the BFF must mint fresh ids), if
/// `seats` isn't 2..=4 long, or if a seat's deck doesn't resolve.
#[utoipa::path(
    post,
    path = "/tables/seed/v1",
    request_body = SeedRequest,
    responses(
        (status = 200, description = "Seeded game location", body = SeedResponse),
        (status = 400, description = "Duplicate table id, bad seat count, or unknown deck"),
        (status = 503, description = "Instance draining — retry against another instance"),
    ),
)]
pub async fn seed_table(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(req): Json<SeedRequest>,
) -> Result<Json<SeedResponse>, StatusCode> {
    if state.draining.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !(2..=4).contains(&req.seats.len()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if lock(&state.reg).tables.contains_key(&req.table_id) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Resolve every seat's deck before touching the registry — no DB await is held across the
    // registry lock.
    let mut resolved: Vec<(PlayerId, SeatDeck)> = Vec::with_capacity(req.seats.len());
    for (i, seat) in req.seats.iter().enumerate() {
        let deck = resolve_deck(&state, seat.deck_id)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        resolved.push((PlayerId(i as u8), deck));
    }

    let mut reg = lock(&state.reg);
    // Re-check under the lock: another request could have raced the same id past the first check.
    if reg.tables.contains_key(&req.table_id) {
        return Err(StatusCode::BAD_REQUEST);
    }
    // H3: seed each game from the OS CSPRNG so libraries aren't reproducible offline (the pool is
    // five *published* decklists). Record the seed on the table so a replay reproduces the shuffle.
    let seed = OsRng.next_u64();
    let mut table = Table::seeded(req.host_user_id, &req.seats);
    for (player, deck) in &resolved {
        table.prints[player.0 as usize] = deck.prints.clone();
    }
    table.game = Some(seed_game(&resolved, seed));
    reg.tables.insert(req.table_id.clone(), table);
    drop(reg);
    crate::action_log::start(&req.table_id); // outside the lock — blocking disk I/O

    let pod_dns = if state.settings.pod_dns.is_empty() {
        state.settings.instance_id.clone() // dev fallback: no real DNS configured
    } else {
        state.settings.pod_dns.clone()
    };
    Ok(Json(SeedResponse {
        table_id: req.table_id,
        pod_dns,
        version: state.settings.version.clone(),
    }))
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
    legality::validate(&commander_id, &commander_print, &entries).map_err(|_| "IllegalDeck")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::test_support::user_with_deck;
    use schema::SeedSeat;

    async fn seed_seat(state: &AppState, email: &str, username: &str) -> SeedSeat {
        let deck_id = user_with_deck(state, email).await;
        SeedSeat {
            user_id: crate::test_support::as_user(state, email).await.0.id,
            username: username.to_string(),
            deck_id,
        }
    }

    #[tokio::test]
    async fn seed_table_builds_a_running_two_player_game() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;

        let resp = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(SeedRequest {
                table_id: "tbl1".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            }),
        )
        .await
        .expect("seeding succeeds");
        assert_eq!(resp.table_id, "tbl1");
        assert_eq!(resp.version, state.settings.version);
        // Dev fallback: no pod_dns configured, so it falls back to instance_id.
        assert_eq!(resp.pod_dns, state.settings.instance_id);

        let reg = lock(&state.reg);
        let table = reg.tables.get("tbl1").expect("table inserted");
        assert!(table.game.is_some(), "the game is seeded immediately");
        assert_eq!(table.host, Some(host_user_id));
        assert_eq!(table.seats[0].username.as_deref(), Some("host"));
        assert_eq!(table.seats[1].username.as_deref(), Some("guest"));
    }

    #[tokio::test]
    async fn seed_table_is_rejected_with_503_while_draining() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        state.draining.store(true, Ordering::Relaxed);

        let err = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(SeedRequest {
                table_id: "tbl-draining".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            }),
        )
        .await
        .expect_err("draining rejects new tables");
        assert_eq!(err, StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            !lock(&state.reg).tables.contains_key("tbl-draining"),
            "no table was created while draining"
        );
    }

    #[tokio::test]
    async fn seed_table_rejects_an_unknown_deck() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let mut host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        host_seat.deck_id = 999_999; // no such deck

        let err = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(SeedRequest {
                table_id: "tbl-baddeck".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            }),
        )
        .await
        .expect_err("an unresolvable deck is rejected");
        assert_eq!(err, StatusCode::BAD_REQUEST);
        assert!(!lock(&state.reg).tables.contains_key("tbl-baddeck"));
    }

    #[tokio::test]
    async fn seed_table_rejects_a_duplicate_table_id() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;

        let req = SeedRequest {
            table_id: "tbl-dup".to_string(),
            host_user_id,
            seats: vec![host_seat, guest_seat],
        };
        let _ = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(req.clone()),
        )
        .await
        .expect("first seed succeeds");

        let err = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(req),
        )
        .await
        .expect_err("a duplicate table id is rejected");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn seed_table_rejects_fewer_than_two_seats() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let host_user_id = host_seat.user_id;

        let err = seed_table(
            State(state.clone()),
            crate::test_support::as_user(&state, "host@x.c").await,
            Json(SeedRequest {
                table_id: "tbl-solo".to_string(),
                host_user_id,
                seats: vec![host_seat],
            }),
        )
        .await
        .expect_err("a single seat can't start a game");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }
}
