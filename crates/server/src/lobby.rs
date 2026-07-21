//! Seed a live `Table` from seats the SolidStart BFF already resolved, for the gRPC
//! `Tables.Seed` service (`grpc::tables_svc`).

use std::sync::atomic::Ordering;

use crate::beacon::{EntropySource, HttpEntropySource, resolve_entropy_with_env};
use crate::db::Deck;
use crate::decks::{SeatDeck, seed_game};
use crate::{AppState, Table, legality, lock, precons};
use axum::http::StatusCode;
use engine::PlayerId;
use schema::{DeckCardEntry, SeedRequest, SeedResponse};

/// Seed a running game from BFF-resolved seats. Rejects with `SERVICE_UNAVAILABLE` while
/// draining. Called by the gRPC `Tables.Seed` service.
pub(crate) async fn seed_table_core(
    state: &AppState,
    caller_user_id: i64,
    req: SeedRequest,
) -> Result<SeedResponse, StatusCode> {
    seed_table_core_with_entropy(state, caller_user_id, req, &HttpEntropySource).await
}

pub(crate) async fn seed_table_core_with_entropy(
    state: &AppState,
    caller_user_id: i64,
    req: SeedRequest,
    entropy_source: &impl EntropySource,
) -> Result<SeedResponse, StatusCode> {
    if state.draining.load(Ordering::Relaxed) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if !(2..=4).contains(&req.seats.len()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if caller_user_id != req.host_user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if !req.seats.iter().any(|s| s.user_id == caller_user_id) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if lock(&state.reg).get(&req.table_id).is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Resolve decks before touching the registry — no DB await across the lock.
    let mut resolved: Vec<(PlayerId, SeatDeck)> = Vec::with_capacity(req.seats.len());
    for (i, seat) in req.seats.iter().enumerate() {
        let deck = resolve_deck(state, seat.deck_id, seat.user_id)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        resolved.push((PlayerId(i as u8), deck));
    }

    let entropy = resolve_entropy_with_env(entropy_source, state.settings.master_seed.as_deref())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let mut reg = lock(&state.reg);
    let mut table = Table::seeded(req.host_user_id, &req.seats);
    for (player, deck) in &resolved {
        table.prints[player.0 as usize] = deck.prints.clone();
    }
    table.seed = entropy.master_seed;
    table.beacon_round = entropy.beacon_round;
    table.game = Some(seed_game(&resolved, entropy.master_seed));
    // try_insert under the lock: another request could have raced the same id past the first check.
    if !reg.try_insert(req.table_id.clone(), table) {
        return Err(StatusCode::BAD_REQUEST);
    }
    drop(reg);
    crate::action_log::start(&req.table_id); // outside the lock — blocking disk I/O

    // Empty `pod_dns` means single-process dev: hand the BFF an absolute upstream it can
    // proxy to. Returning bare `instance_id` ("local") made the BFF dial `http://local:8080`.
    let pod_dns = if state.settings.pod_dns.is_empty() {
        format!("http://{}:{}", state.settings.host, state.settings.port)
    } else {
        state.settings.pod_dns.clone()
    };
    Ok(SeedResponse {
        table_id: req.table_id,
        pod_dns,
        version: state.settings.version.clone(),
    })
}

/// Load a deck and resolve commander + cards; non-precons must be owned by `seat_user_id`.
async fn resolve_deck(
    state: &AppState,
    deck_id: i64,
    seat_user_id: i64,
) -> Result<SeatDeck, &'static str> {
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
            if deck.user_id != seat_user_id {
                return Err("NotOwner");
            }
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

        let resp = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl1".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
        )
        .await
        .expect("seeding succeeds");
        assert_eq!(resp.table_id, "tbl1");
        assert_eq!(resp.version, state.settings.version);
        // Dev fallback: no pod_dns configured → absolute listen address for the BFF proxy.
        assert_eq!(
            resp.pod_dns,
            format!("http://{}:{}", state.settings.host, state.settings.port)
        );

        let reg = lock(&state.reg);
        let table = reg.get("tbl1").expect("table inserted");
        assert!(table.game.is_some(), "the game is seeded immediately");
        assert_eq!(table.host, Some(host_user_id));
        assert_eq!(table.seats[0].username.as_deref(), Some("host"));
        assert_eq!(table.seats[1].username.as_deref(), Some("guest"));
        assert_eq!(
            table.seed,
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
        );
        assert_eq!(table.beacon_round, 0);
    }

    #[tokio::test]
    async fn seed_table_is_rejected_with_503_while_draining() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        state.draining.store(true, Ordering::Relaxed);

        let err = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-draining".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
        )
        .await
        .expect_err("draining rejects new tables");
        assert_eq!(err, StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            lock(&state.reg).get("tbl-draining").is_none(),
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

        let err = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-baddeck".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
        )
        .await
        .expect_err("an unresolvable deck is rejected");
        assert_eq!(err, StatusCode::BAD_REQUEST);
        assert!(lock(&state.reg).get("tbl-baddeck").is_none());
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
        let _ = seed_table_core(&state, host_user_id, req.clone())
            .await
            .expect("first seed succeeds");

        let err = seed_table_core(&state, host_user_id, req)
            .await
            .expect_err("a duplicate table id is rejected");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn seed_table_rejects_fewer_than_two_seats() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let host_user_id = host_seat.user_id;

        let err = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-solo".to_string(),
                host_user_id,
                seats: vec![host_seat],
            },
        )
        .await
        .expect_err("a single seat can't start a game");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn seed_table_rejects_a_non_host_caller() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        let guest_user_id = guest_seat.user_id;

        let err = seed_table_core(
            &state,
            guest_user_id,
            SeedRequest {
                table_id: "tbl-spoof".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
        )
        .await
        .expect_err("only the host may seed");
        assert_eq!(err, StatusCode::FORBIDDEN);
        assert!(lock(&state.reg).get("tbl-spoof").is_none());
    }

    #[tokio::test]
    async fn seed_table_rejects_when_host_is_not_in_seats() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_a = seed_seat(&state, "a@x.c", "a").await;
        let guest_b = seed_seat(&state, "b@x.c", "b").await;
        let host_user_id = host_seat.user_id;

        let err = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-absent-host".to_string(),
                host_user_id,
                seats: vec![guest_a, guest_b],
            },
        )
        .await
        .expect_err("host must be seated");
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn seed_table_rejects_a_deck_owned_by_another_user() {
        let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
        let mut host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        // Host tries to play the guest's private deck.
        host_seat.deck_id = guest_seat.deck_id;

        let err = seed_table_core(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-stolen-deck".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
        )
        .await
        .expect_err("deck must belong to the seated user");
        assert_eq!(err, StatusCode::BAD_REQUEST);
        assert!(lock(&state.reg).get("tbl-stolen-deck").is_none());
    }

    struct FixedEntropy(crate::beacon::MasterEntropy);

    impl EntropySource for FixedEntropy {
        async fn latest(&self) -> Result<crate::beacon::MasterEntropy, crate::beacon::BeaconError> {
            Ok(self.0)
        }
    }

    struct FailingEntropy;

    impl EntropySource for FailingEntropy {
        async fn latest(&self) -> Result<crate::beacon::MasterEntropy, crate::beacon::BeaconError> {
            Err(crate::beacon::BeaconError::Unavailable)
        }
    }

    #[tokio::test]
    async fn beacon_failure_rejects_seed_with_503() {
        let mut settings = crate::settings::for_test();
        settings.master_seed = None;
        let state = AppState::new(
            db::connect("sqlite::memory:").await.expect("sqlite"),
            std::sync::Arc::new(settings),
        );
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;

        let err = seed_table_core_with_entropy(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-beacon-down".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
            &FailingEntropy,
        )
        .await
        .expect_err("beacon failure rejects new tables");

        assert_eq!(err, StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            lock(&state.reg).get("tbl-beacon-down").is_none(),
            "no table is created without beacon entropy"
        );
    }

    #[tokio::test]
    async fn seed_table_records_beacon_entropy() {
        let mut settings = crate::settings::for_test();
        settings.master_seed = None;
        let state = AppState::new(
            db::connect("sqlite::memory:").await.expect("sqlite"),
            std::sync::Arc::new(settings),
        );
        let host_seat = seed_seat(&state, "host@x.c", "host").await;
        let guest_seat = seed_seat(&state, "guest@x.c", "guest").await;
        let host_user_id = host_seat.user_id;
        let master_seed = [7; 32];

        let _ = seed_table_core_with_entropy(
            &state,
            host_user_id,
            SeedRequest {
                table_id: "tbl-beacon-recorded".to_string(),
                host_user_id,
                seats: vec![host_seat, guest_seat],
            },
            &FixedEntropy(crate::beacon::MasterEntropy {
                master_seed,
                beacon_round: 123_456,
            }),
        )
        .await
        .expect("beacon entropy seeds table");

        let reg = lock(&state.reg);
        let table = reg.get("tbl-beacon-recorded").expect("table inserted");
        assert_eq!(table.seed, master_seed);
        assert_eq!(table.beacon_round, 123_456);
        assert!(table.game.is_some(), "the game is seeded immediately");
    }
}
