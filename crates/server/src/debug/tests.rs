use std::sync::{Arc, Barrier};
use std::thread;

use engine::debug::{DebugZone, ErrorReason, Mutation};
use engine::{Intent, PlayerId};
use schema::StreamFrame;
use tokio::sync::broadcast::error::TryRecvError;

use super::*;
use crate::db;
use crate::session::{PublishedUpdate, TableSession};
use crate::stream::frame_for_update;
use crate::{AppState, Table, lock};

async fn state_with_game(table_id: &str, game: engine::Game) -> AppState {
    let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
    let mut table = Table::empty();
    table.game = Some(game);
    assert!(lock(&state.reg).try_insert(table_id.to_string(), table));
    state
}

fn set_life(life: i32) -> Mutation {
    Mutation::SetLife {
        player: PlayerId(0),
        life,
    }
}

fn command(table_id: &str, operations: Vec<Mutation>) -> MutateCommand {
    MutateCommand {
        table_id: table_id.to_string(),
        expected_debug_revision: None,
        expected_table_seq: None,
        operations: operations.into_iter().map(TableMutation::Engine).collect(),
        encoded_request_bytes: 13,
    }
}

fn assert_debug_state_is_empty(debug: &TableDebugState) {
    assert_eq!(debug.revision, 0);
    assert!(!debug.debug_mutated);
    assert!(debug.object_prints.is_empty());
    assert!(debug.checkpoints.is_empty());
    assert_eq!(debug.checkpoint_object_slots, 0);
    assert!(debug.journal.is_empty());
    assert_eq!(debug.journal_request_bytes, 0);
    assert_eq!(debug.next_journal_ordinal, 0);
}

fn game_with_object_slots(object_slots: usize) -> engine::Game {
    let mut game = engine::Game::with_players(2, 0);
    if object_slots == 0 {
        return game;
    }
    let card_id = cards::get_by_name("Plains").unwrap().id.to_string();
    let operations = (0..object_slots)
        .map(|object_id| Mutation::CreateCard {
            object_id: u32::try_from(object_id).expect("test object id fits"),
            card_id: card_id.clone(),
            owner: PlayerId(0),
            controller: PlayerId(0),
            destination: DebugZone::Exile,
            commander: false,
            face_down: false,
        })
        .collect::<Vec<_>>();
    engine::debug::apply_operations(&mut game, &operations).expect("real CreateCard operations");
    assert_eq!(engine::debug::object_slot_count(&game), object_slots);
    game
}

async fn state_with_object_slots(table_id: &str, object_slots: usize) -> AppState {
    state_with_game(table_id, game_with_object_slots(object_slots)).await
}

fn checkpoint_command(name: impl Into<String>, encoded_request_bytes: usize) -> CheckpointCommand {
    CheckpointCommand {
        table_id: "table".to_string(),
        name: name.into(),
        replace_existing: false,
        expected_table_seq: Some(0),
        encoded_request_bytes,
    }
}

fn replace_live_game(state: &AppState, table_id: &str, game: engine::Game) {
    lock(&state.reg).get_mut(table_id).unwrap().game = Some(game);
}

#[derive(Debug, PartialEq, Eq)]
struct CheckpointFingerprint {
    name: String,
    game: engine::debug::Inspection,
    chrome: DebugChromeSnapshot,
    object_prints: schema::ObjectPrintOverrides,
    source_table_seq: u64,
    object_slots: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct TableTransactionFingerprint {
    game: Option<engine::debug::Inspection>,
    logical_chrome: DebugChromeSnapshot,
    active_hold: Option<(u64, tokio::time::Instant)>,
    hold_deadline: Option<tokio::time::Instant>,
    any_dwell: bool,
    seq: u64,
    broadcast_seq: u64,
    revision: u64,
    debug_mutated: bool,
    object_prints: schema::ObjectPrintOverrides,
    checkpoint_object_slots: usize,
    checkpoints: Vec<CheckpointFingerprint>,
    journal: Vec<JournalRecord>,
    journal_request_bytes: usize,
    next_journal_ordinal: u64,
    seats: Vec<(Option<i64>, Option<String>, String)>,
    prints: [std::collections::HashMap<String, String>; 4],
    tx_len: usize,
}

fn table_transaction_fingerprint(state: &AppState, table_id: &str) -> TableTransactionFingerprint {
    let registry = lock(&state.reg);
    let table = registry.get(table_id).unwrap();
    let debug = &table.debug;
    let active_hold = table.chrome.stack_hold();
    let any_dwell = table.chrome.any_dwell();
    let hold_deadline = active_hold.map(|(_, started)| {
        started
            + crate::session::STACK_HOLD
            + if any_dwell {
                crate::session::STACK_HOLD_DWELL_EXTRA
            } else {
                std::time::Duration::ZERO
            }
    });
    TableTransactionFingerprint {
        game: table.game.as_ref().map(engine::debug::inspect),
        logical_chrome: table.chrome.debug_snapshot(),
        active_hold,
        hold_deadline,
        any_dwell,
        seq: table.seq,
        broadcast_seq: table.broadcast_seq,
        revision: debug.revision,
        debug_mutated: debug.debug_mutated,
        object_prints: debug.object_prints.clone(),
        checkpoint_object_slots: debug.checkpoint_object_slots,
        checkpoints: debug
            .checkpoints
            .iter()
            .map(|(name, checkpoint)| CheckpointFingerprint {
                name: name.clone(),
                game: engine::debug::inspect(&checkpoint.game),
                chrome: checkpoint.chrome,
                object_prints: checkpoint.object_prints.clone(),
                source_table_seq: checkpoint.source_table_seq,
                object_slots: checkpoint.object_slots,
            })
            .collect(),
        journal: debug.journal.iter().cloned().collect(),
        journal_request_bytes: debug.journal_request_bytes,
        next_journal_ordinal: debug.next_journal_ordinal,
        seats: table
            .seats
            .iter()
            .map(|seat| {
                (
                    seat.user_id,
                    seat.username.clone(),
                    seat.gravatar_hash.clone(),
                )
            })
            .collect(),
        prints: table.prints.clone(),
        tx_len: table.tx.len(),
    }
}

#[tokio::test]
async fn transaction_rejects_empty_unknown_and_stale_commands_without_change() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();

    assert_eq!(
        mutate_table(&state, command("missing", vec![set_life(1)])),
        Err(DebugFailure::NotFound {
            operation_index: None,
        })
    );
    assert_eq!(
        mutate_table(&state, command("table", vec![])),
        Err(DebugFailure::Invalid {
            operation_index: None,
            reason: ErrorReason::EmptyBatch,
        })
    );

    let mut stale_revision = command("table", vec![set_life(1)]);
    stale_revision.expected_debug_revision = Some(1);
    assert!(matches!(
        mutate_table(&state, stale_revision),
        Err(DebugFailure::Aborted {
            reason: DebugAbortReason::DebugRevisionMismatch,
            actual_debug_revision: 0,
            actual_table_seq: 0,
        })
    ));

    let mut stale_seq = command("table", vec![set_life(1)]);
    stale_seq.expected_table_seq = Some(1);
    assert!(matches!(
        mutate_table(&state, stale_seq),
        Err(DebugFailure::Aborted {
            reason: DebugAbortReason::TableSeqMismatch,
            actual_debug_revision: 0,
            actual_table_seq: 0,
        })
    ));

    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_debug_state_is_empty(&table.debug);
    assert_eq!((table.seq, table.broadcast_seq), (0, 0));
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn transaction_rolls_back_a_late_indexed_failure() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = {
        let reg = lock(&state.reg);
        engine::debug::inspect(reg.get("table").unwrap().game.as_ref().unwrap())
    };

    let failure = mutate_table(
        &state,
        command(
            "table",
            vec![
                set_life(1),
                Mutation::SetPermanentState {
                    object_id: 999,
                    tapped: Some(true),
                    marked_damage: None,
                    plus_one_counters: None,
                },
            ],
        ),
    );
    assert_eq!(
        failure,
        Err(DebugFailure::NotFound {
            operation_index: Some(1),
        })
    );

    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(engine::debug::inspect(table.game.as_ref().unwrap()), before);
    assert_eq!((table.seq, table.broadcast_seq), (0, 0));
    assert_debug_state_is_empty(&table.debug);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn transaction_maps_an_occupied_create_id_to_already_exists() {
    let mut game = engine::Game::with_players(2, 0);
    game.spawn_in_hand(PlayerId(0), cards::get_by_name("Shock").unwrap());
    let state = state_with_game("table", game).await;
    let plains = cards::get_by_name("Plains").unwrap();

    let result = mutate_table(
        &state,
        command(
            "table",
            vec![Mutation::CreateCard {
                object_id: 0,
                card_id: plains.id.to_string(),
                owner: PlayerId(0),
                controller: PlayerId(0),
                destination: DebugZone::Hand,
                commander: false,
                face_down: false,
            }],
        ),
    );

    assert_eq!(
        result,
        Err(DebugFailure::AlreadyExists { operation_index: 0 })
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (0, 0, 0)
    );
}

#[tokio::test]
async fn transaction_maps_duplicate_library_members_to_invalid_at_the_operation_index() {
    let mut game = engine::Game::with_players(2, 0);
    let object_id = game.spawn_in_library(PlayerId(0), cards::get_by_name("Island").unwrap());
    let state = state_with_game("table", game).await;

    let result = mutate_table(
        &state,
        command(
            "table",
            vec![
                set_life(19),
                Mutation::SetLibraryOrder {
                    player: PlayerId(0),
                    object_ids: vec![object_id, object_id],
                },
            ],
        ),
    );

    assert_eq!(
        result,
        Err(DebugFailure::Invalid {
            operation_index: Some(1),
            reason: ErrorReason::DuplicateId,
        })
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (0, 0, 0)
    );
}

#[tokio::test]
async fn transaction_maps_an_occupied_move_destination_id_to_invalid_at_the_operation_index() {
    let mut game = engine::Game::with_players(2, 0);
    let object_id = game.spawn_in_hand(PlayerId(0), cards::get_by_name("Island").unwrap());
    let state = state_with_game("table", game).await;

    let result = mutate_table(
        &state,
        command(
            "table",
            vec![
                set_life(19),
                Mutation::MoveCard {
                    object_id,
                    new_object_id: object_id,
                    destination: DebugZone::Graveyard,
                    controller: PlayerId(0),
                    face_down: false,
                },
            ],
        ),
    );

    assert_eq!(
        result,
        Err(DebugFailure::Invalid {
            operation_index: Some(1),
            reason: ErrorReason::DuplicateId,
        })
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (0, 0, 0)
    );
}

#[tokio::test]
async fn transaction_success_swaps_once_preserves_logical_chrome_and_publishes_one_fresh_snapshot()
{
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    let mut rx = {
        let mut reg = lock(&state.reg);
        let table = reg.get_mut("table").unwrap();
        table.seats[0].username = Some("fresh-alice".into());
        table.prints[0].insert("fresh-card".into(), "fresh-print".into());
        table.chrome.set_yields_for_test([true; 4]);
        table.chrome.set_turn_yield_flag(1, true);
        table
            .chrome
            .stamp_hold_for_test(table.seq, tokio::time::Instant::now());
        table.chrome.set_dwell_flag(1, true);
        table.tx.subscribe()
    };

    let receipt = mutate_table(&state, command("table", vec![set_life(17)])).unwrap();
    assert_eq!(
        receipt,
        MutateReceipt {
            debug_revision: 1,
            table_seq: 1,
            applied_operation_count: 1,
        }
    );

    {
        let reg = lock(&state.reg);
        let table = reg.get("table").unwrap();
        assert_eq!(
            engine::debug::inspect(table.game.as_ref().unwrap()).players[0].life,
            17
        );
        assert_eq!((table.seq, table.broadcast_seq), (1, 1));
        assert_eq!(table.debug.revision, 1);
        assert!(table.debug.debug_mutated);
        assert_eq!(*table.chrome.yields(), [true; 4]);
        assert_eq!(*table.chrome.turn_yields(), [false, true, false, false]);
        assert!(table.chrome.stack_hold().is_none());
        assert!(!table.chrome.any_dwell());
    }
    {
        let mut reg = lock(&state.reg);
        let table = reg.get_mut("table").unwrap();
        table
            .chrome
            .stamp_hold_for_test(table.seq, tokio::time::Instant::now());
        table.chrome.clear_hold_if_seq(0);
        assert!(
            table.chrome.stack_hold().is_some(),
            "a detached pre-commit timer cannot clear a hold stamped at the new sequence",
        );
        table.chrome.clear_hold();
    }

    let update = rx.try_recv().expect("one snapshot");
    let PublishedUpdate::Snapshot(published) = update.as_ref() else {
        panic!("debug commits publish a snapshot")
    };
    assert_eq!((published.seq, published.broadcast_seq), (1, 1));
    assert_eq!(published.game.life(PlayerId(0)), 17);
    assert_eq!(published.seats[0].username.as_deref(), Some("fresh-alice"));
    assert_eq!(
        published.prints[0].get("fresh-card").map(String::as_str),
        Some("fresh-print")
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn concurrent_guarded_transactions_serialize_and_only_one_commits() {
    let state = Arc::new(state_with_game("table", engine::Game::with_players(2, 0)).await);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = [11, 22]
        .into_iter()
        .map(|life| {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut command = command("table", vec![set_life(life)]);
                command.expected_debug_revision = Some(0);
                command.expected_table_seq = Some(0);
                barrier.wait();
                mutate_table(&state, command)
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(DebugFailure::Aborted { .. })))
            .count(),
        1
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (1, 1, 1)
    );
}

#[tokio::test]
async fn ordinary_intent_interleaves_with_guards_and_continues_after_debug_commit() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let mut reg = lock(&state.reg);
        let table = reg.get_mut("table").unwrap();
        let holder = table.game.as_ref().unwrap().priority_holder();
        let (result, _) = TableSession::new(table).submit(Intent::PassPriority { player: holder });
        assert!(result.accepted);
    }
    let mut stale = command("table", vec![set_life(12)]);
    stale.expected_debug_revision = Some(0);
    stale.expected_table_seq = Some(0);
    assert!(matches!(
        mutate_table(&state, stale),
        Err(DebugFailure::Aborted {
            reason: DebugAbortReason::TableSeqMismatch,
            actual_table_seq: 1,
            ..
        })
    ));

    mutate_table(&state, command("table", vec![set_life(12)])).unwrap();
    let mut reg = lock(&state.reg);
    let table = reg.get_mut("table").unwrap();
    let holder = table.game.as_ref().unwrap().priority_holder();
    let (result, _) = TableSession::new(table).submit(Intent::PassPriority { player: holder });
    assert!(
        result.accepted,
        "ordinary play continues after a debug snapshot"
    );
    assert_eq!(table.seq, 3);
}

#[tokio::test]
async fn visibility_snapshot_hides_hand_and_library_from_every_non_owner() {
    let state = state_with_game("table", engine::Game::with_players(4, 0)).await;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let shock = cards::get_by_name("Shock").unwrap();
    let plains = cards::get_by_name("Plains").unwrap();
    let result = mutate_table(
        &state,
        command(
            "table",
            vec![
                Mutation::CreateCard {
                    object_id: 0,
                    card_id: shock.id.to_string(),
                    owner: PlayerId(0),
                    controller: PlayerId(0),
                    destination: DebugZone::Hand,
                    commander: false,
                    face_down: false,
                },
                Mutation::CreateCard {
                    object_id: 1,
                    card_id: plains.id.to_string(),
                    owner: PlayerId(0),
                    controller: PlayerId(0),
                    destination: DebugZone::Library,
                    commander: false,
                    face_down: false,
                },
            ],
        ),
    );
    assert!(result.is_ok(), "{result:?}");
    let update = rx.try_recv().unwrap();

    for viewer in [
        Some(PlayerId(0)),
        Some(PlayerId(1)),
        Some(PlayerId(2)),
        Some(PlayerId(3)),
        None,
    ] {
        let StreamFrame::Snapshot { state, .. } = frame_for_update(viewer, &update) else {
            panic!("expected snapshot")
        };
        assert_eq!(state.players[0].hand_count, 1);
        assert_eq!(state.players[0].library_count, 1);
        let visible_private: Vec<_> = state
            .objects
            .iter()
            .filter(|o| o.id == 0 || o.id == 1)
            .collect();
        if viewer == Some(PlayerId(0)) {
            assert_eq!(
                visible_private.len(),
                1,
                "library stays count-only even for its owner"
            );
            assert_eq!(visible_private[0].name, "Shock");
        } else {
            assert!(
                visible_private.is_empty(),
                "non-owners see counts, not identities"
            );
        }
    }
}

#[tokio::test]
async fn projection_panic_rolls_back_without_poisoning_or_disabling_the_registry() {
    // A fifth engine player is structurally coherent but cannot index four-seat chrome extras.
    let state = state_with_game("table", engine::Game::with_players(5, 0)).await;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = {
        let reg = lock(&state.reg);
        engine::debug::inspect(reg.get("table").unwrap().game.as_ref().unwrap())
    };
    let result = mutate_table(&state, command("table", vec![set_life(5)]));
    assert!(matches!(
        result,
        Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            ref violations,
        }) if violations.iter().any(|v| v.code == "projection_failed")
    ));
    let reg = lock(&state.reg);
    let table = reg.get("table").expect("registry remains usable");
    assert_eq!(engine::debug::inspect(table.game.as_ref().unwrap()), before);
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (0, 0, 0)
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn empty_batch_wins_over_exhausted_table_sequence_without_change() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let mut reg = lock(&state.reg);
        reg.get_mut("table").unwrap().seq = u64::MAX;
    }

    let result = mutate_table(&state, command("table", vec![]));

    assert_eq!(
        result,
        Err(DebugFailure::Invalid {
            operation_index: None,
            reason: ErrorReason::EmptyBatch,
        })
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (u64::MAX, 0, 0)
    );
}

#[tokio::test]
async fn missing_game_wins_over_exhausted_counters_without_change() {
    let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
    let mut table = Table::empty();
    table.seq = u64::MAX;
    table.broadcast_seq = u64::MAX;
    table.debug.revision = u64::MAX;
    assert!(lock(&state.reg).try_insert("table".to_string(), table));

    let result = mutate_table(&state, command("table", vec![set_life(19)]));

    assert_eq!(
        result,
        Err(DebugFailure::NotFound {
            operation_index: None,
        })
    );
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert!(table.game.is_none());
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (u64::MAX, u64::MAX, u64::MAX)
    );
}

#[tokio::test]
async fn valid_candidate_at_exhausted_capacity_fails_and_rolls_back() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    let mut rx = {
        let mut reg = lock(&state.reg);
        let table = reg.get_mut("table").unwrap();
        table.seq = 7;
        table.broadcast_seq = 8;
        table.debug.revision = u64::MAX;
        table.tx.subscribe()
    };

    let result = mutate_table(&state, command("table", vec![set_life(19)]));

    assert!(matches!(
        result,
        Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            ref violations,
        }) if violations.len() == 1 && violations[0].code == "debug_revision_exhausted"
    ));
    let reg = lock(&state.reg);
    let table = reg.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (7, 8, u64::MAX)
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn summaries_are_sorted_and_sequence_exhaustion_is_a_stable_rollback() {
    let state = state_with_game("z", engine::Game::with_players(2, 0)).await;
    {
        let mut reg = lock(&state.reg);
        let mut a = Table::empty();
        a.game = Some(engine::Game::with_players(2, 1));
        a.seq = 7;
        a.debug.revision = 3;
        assert!(reg.try_insert("a".into(), a));
        reg.get_mut("z").unwrap().seq = u64::MAX;
        assert_eq!(
            reg.debug_table_summaries(),
            vec![("a".into(), 3, 7), ("z".into(), 0, u64::MAX)]
        );
    }
    let result = mutate_table(&state, command("z", vec![set_life(9)]));
    assert!(matches!(
        result,
        Err(DebugFailure::FailedPrecondition { .. })
    ));
    let reg = lock(&state.reg);
    let table = reg.get("z").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!(table.seq, u64::MAX);
    assert_eq!(table.debug.revision, 0);
}

#[tokio::test]
async fn checkpoint_names_enforce_exact_ascii_grammar_and_byte_bound() {
    for invalid in [
        "".to_string(),
        "a".repeat(65),
        "bad/name".to_string(),
        "bad name".to_string(),
        "é".to_string(),
    ] {
        let state = state_with_object_slots("table", 1).await;
        let before = table_transaction_fingerprint(&state, "table");
        assert_eq!(
            checkpoint_table(&state, checkpoint_command(invalid, 1)),
            Err(DebugFailure::InvalidCheckpointName),
        );
        assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    }

    let state = state_with_object_slots("table", 1).await;
    let name = "a".repeat(64);
    let receipt = checkpoint_table(&state, checkpoint_command(name.clone(), 1)).unwrap();
    assert_eq!(receipt.object_slots, 1);
    assert!(
        lock(&state.reg)
            .get("table")
            .unwrap()
            .debug
            .checkpoints
            .contains_key(&name)
    );
}

#[tokio::test]
async fn checkpoint_requires_explicit_replacement_and_reports_it() {
    let state = state_with_object_slots("table", 2).await;
    let first = checkpoint_table(&state, checkpoint_command("baseline", 3)).unwrap();
    assert!(!first.replaced);
    let before = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("baseline", 3)),
        Err(DebugFailure::CheckpointAlreadyExists),
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);

    let mut replace = checkpoint_command("baseline", 4);
    replace.replace_existing = true;
    let replaced = checkpoint_table(&state, replace).unwrap();
    assert!(replaced.replaced);
    assert_eq!(replaced.debug_revision, 0);
    assert_eq!(replaced.table_seq, 0);
}

#[tokio::test]
async fn checkpoint_count_accepts_sixteen_distinct_names_and_rejects_seventeenth() {
    let state = state_with_object_slots("table", 1).await;
    for index in 0..MAX_CHECKPOINTS_PER_TABLE {
        checkpoint_table(&state, checkpoint_command(format!("cp-{index}"), 0)).unwrap();
    }
    let before = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("overflow", 0)),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointCount,
        }),
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
}

#[tokio::test]
async fn checkpoint_object_slot_limit_counts_the_real_arena_including_removed_slots() {
    let state = state_with_object_slots("table", MAX_OBJECT_SLOTS_PER_CHECKPOINT).await;
    checkpoint_table(&state, checkpoint_command("exact", 0)).unwrap();

    let mut too_large = game_with_object_slots(MAX_OBJECT_SLOTS_PER_CHECKPOINT + 1);
    engine::debug::apply_operations(&mut too_large, &[Mutation::RemoveCard { object_id: 0 }])
        .unwrap();
    assert_eq!(
        engine::debug::object_slot_count(&too_large),
        MAX_OBJECT_SLOTS_PER_CHECKPOINT + 1,
        "removed arena entries remain counted",
    );
    replace_live_game(&state, "table", too_large);
    let before = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("too-large", 0)),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        }),
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
}

#[tokio::test]
async fn checkpoint_aggregate_accepts_exact_bound_and_rejects_more() {
    let slots_per_checkpoint = MAX_OBJECT_SLOTS_ACROSS_CHECKPOINTS / MAX_CHECKPOINTS_PER_TABLE;
    let state = state_with_object_slots("table", slots_per_checkpoint).await;
    for index in 0..MAX_CHECKPOINTS_PER_TABLE {
        checkpoint_table(&state, checkpoint_command(format!("cp-{index}"), 0)).unwrap();
    }
    assert_eq!(
        lock(&state.reg)
            .get("table")
            .unwrap()
            .debug
            .checkpoint_object_slots,
        MAX_OBJECT_SLOTS_ACROSS_CHECKPOINTS,
    );

    replace_live_game(
        &state,
        "table",
        game_with_object_slots(slots_per_checkpoint + 1),
    );
    let mut replacement = checkpoint_command("cp-0", 0);
    replacement.replace_existing = true;
    let before = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, replacement),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        }),
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
}

#[tokio::test]
async fn failed_larger_replacement_keeps_old_checkpoint_totals_and_journal() {
    let state = state_with_object_slots("table", 8).await;
    let first = checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            replace_existing: false,
            expected_table_seq: Some(0),
            encoded_request_bytes: 20,
        },
    )
    .unwrap();
    assert!(!first.replaced);
    replace_live_game(&state, "table", game_with_object_slots(4_097));
    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table.seq = 9;
        table
            .chrome
            .set_yields_for_test([true, false, false, false]);
        table.chrome.set_turn_yield_flag(1, true);
        table.debug.revision = 7;
        table.debug.debug_mutated = true;
    }
    let before = table_transaction_fingerprint(&state, "table");

    let error = checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            replace_existing: true,
            expected_table_seq: None,
            encoded_request_bytes: 20,
        },
    )
    .unwrap_err();

    assert_eq!(
        error,
        DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        }
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
}

#[test]
fn journal_preflight_accepts_exact_record_and_byte_bounds_without_eviction() {
    let mut debug = TableDebugState::default();
    for index in 0..MAX_JOURNAL_RECORDS {
        let bytes = if index == 0 {
            MAX_JOURNAL_REQUEST_BYTES
        } else {
            0
        };
        let preflight = preflight_journal(&debug, bytes).unwrap();
        append_journal(
            &mut debug,
            preflight,
            JournalRecord {
                ordinal: preflight.ordinal,
                timestamp_unix_ms: 0,
                debug_revision: 0,
                table_seq: 0,
                encoded_request_bytes: bytes,
                kind: JournalKind::CheckpointCreated {
                    name: "audit".to_string(),
                    replaced: false,
                },
            },
        );
    }
    assert_eq!(debug.journal.len(), MAX_JOURNAL_RECORDS);
    assert_eq!(debug.journal_request_bytes, MAX_JOURNAL_REQUEST_BYTES);
    let before = (
        debug.journal.len(),
        debug.journal_request_bytes,
        debug.next_journal_ordinal,
    );
    assert_eq!(
        preflight_journal(&debug, 0),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRecords,
        }),
    );
    assert_eq!(
        (
            debug.journal.len(),
            debug.journal_request_bytes,
            debug.next_journal_ordinal
        ),
        before,
    );
}

#[test]
fn journal_preflight_rejects_byte_and_ordinal_overflow_without_change() {
    let mut bytes_full = TableDebugState {
        journal_request_bytes: MAX_JOURNAL_REQUEST_BYTES,
        ..TableDebugState::default()
    };
    assert_eq!(
        preflight_journal(&bytes_full, 1),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        }),
    );
    bytes_full.journal_request_bytes = usize::MAX;
    assert_eq!(
        preflight_journal(&bytes_full, 1),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        }),
    );

    let ordinal_full = TableDebugState {
        next_journal_ordinal: u64::MAX,
        ..TableDebugState::default()
    };
    assert!(matches!(
        preflight_journal(&ordinal_full, 0),
        Err(DebugFailure::FailedPrecondition { ref violations, .. })
            if violations.iter().any(|violation| violation.code == "journal_ordinal_exhausted")
    ));
}

#[test]
fn timestamp_conversion_saturates_before_epoch_and_above_u64_milliseconds() {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    assert_eq!(
        timestamp_unix_ms_at(UNIX_EPOCH - Duration::from_millis(1)),
        0
    );
    let beyond = UNIX_EPOCH
        .checked_add(Duration::from_millis(u64::MAX))
        .and_then(|time| time.checked_add(Duration::from_millis(1)));
    if let Some(beyond) = beyond {
        assert_eq!(timestamp_unix_ms_at(beyond), u64::MAX);
    }
    assert!(timestamp_unix_ms_at(SystemTime::now()) > 0);
}

#[tokio::test]
async fn checkpoint_creation_only_appends_audit_state_and_preserves_live_state_and_stream() {
    let state = state_with_object_slots("table", 2).await;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let receipt = checkpoint_table(&state, checkpoint_command("baseline", 17)).unwrap();
    assert_eq!(
        receipt,
        CheckpointReceipt {
            debug_revision: 0,
            table_seq: 0,
            object_slots: 2,
            replaced: false,
        }
    );

    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((table.seq, table.broadcast_seq), (0, 0));
    assert_eq!(table.debug.revision, 0);
    assert!(!table.debug.debug_mutated);
    assert_eq!(table.debug.journal.len(), 1);
    assert_eq!(table.debug.journal_request_bytes, 17);
    assert_eq!(table.debug.next_journal_ordinal, 1);
    let record = table.debug.journal.front().unwrap();
    assert_eq!(record.ordinal, 0);
    assert_eq!(record.debug_revision, 0);
    assert_eq!(record.table_seq, 0);
    assert_eq!(record.encoded_request_bytes, 17);
    assert!(record.timestamp_unix_ms > 0);
    assert_eq!(
        record.kind,
        JournalKind::CheckpointCreated {
            name: "baseline".to_string(),
            replaced: false,
        }
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn debug_restore_replaces_game_chrome_and_object_prints_monotonically() {
    let state = state_with_game("table", game_with_object_slots(1)).await;
    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table
            .chrome
            .set_yields_for_test([true, false, false, false]);
        table.chrome.set_turn_yield_flag(1, true);
        table
            .debug
            .object_prints
            .insert(0, "checkpoint-print".to_string());
    }
    checkpoint_table(&state, checkpoint_command("baseline", 7)).unwrap();
    mutate_table(&state, command("table", vec![set_life(9)])).unwrap();
    lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .object_prints
        .insert(0, "newer-live-print".to_string());
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();

    let receipt = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(1),
            expected_table_seq: Some(1),
            encoded_request_bytes: 11,
        },
    )
    .unwrap();

    assert_eq!(receipt.debug_revision, 2);
    assert_eq!(receipt.table_seq, 2);
    assert_eq!(receipt.restored_source_table_seq, 0);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!((table.seq, table.broadcast_seq), (2, 2));
    assert_eq!(*table.chrome.yields(), [true, false, false, false]);
    assert_eq!(*table.chrome.turn_yields(), [false, true, false, false]);
    assert!(!table.chrome.any_dwell());
    assert_eq!(
        table.debug.object_prints.get(&0).map(String::as_str),
        Some("checkpoint-print")
    );
    let update = rx.try_recv().expect("one replacement snapshot");
    let PublishedUpdate::Snapshot(snapshot) = update.as_ref() else {
        panic!("restore publishes a snapshot")
    };
    assert_eq!((snapshot.seq, snapshot.broadcast_seq), (2, 2));
    assert_eq!(snapshot.game.life(PlayerId(0)), 20);
    assert_eq!(
        snapshot.object_print_overrides.get(&0).map(String::as_str),
        Some("checkpoint-print")
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(
        table.debug.journal.back().unwrap().kind,
        JournalKind::CheckpointRestored {
            name: "baseline".into(),
            source_table_seq: 0,
        }
    );
    drop(registry);

    let mut registry = lock(&state.reg);
    let table = registry.get_mut("table").unwrap();
    let holder = table.game.as_ref().unwrap().priority_holder();
    let (result, _) = TableSession::new(table).submit(Intent::PassPriority { player: holder });
    assert!(result.accepted, "ordinary intent succeeds after restore");
    assert_eq!(table.seq, 3);
}

#[tokio::test]
async fn debug_restore_checks_revision_before_table_seq_and_rolls_back_everything() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    let before = table_transaction_fingerprint(&state, "table");
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();

    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(9),
            expected_table_seq: Some(9),
            encoded_request_bytes: 2,
        },
    );

    assert!(matches!(
        result,
        Err(DebugFailure::Aborted {
            reason: DebugAbortReason::DebugRevisionMismatch,
            ..
        })
    ));
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((table.seq, table.broadcast_seq), (0, 0));
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn debug_restore_projection_failure_precedes_exhausted_capacity_without_change() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    {
        let mut registry = lock(&state.reg);
        registry
            .get_mut("table")
            .unwrap()
            .debug
            .checkpoints
            .get_mut("baseline")
            .unwrap()
            .game = engine::Game::with_players(5, 0);
        registry.get_mut("table").unwrap().broadcast_seq = u64::MAX;
    }
    let before = table_transaction_fingerprint(&state, "table");

    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            encoded_request_bytes: 1,
        },
    );

    assert!(
        matches!(result, Err(DebugFailure::FailedPrecondition { ref violations, .. })
        if violations.iter().any(|violation| violation.code == "projection_failed"))
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((table.seq, table.broadcast_seq), (0, u64::MAX));
}

#[tokio::test]
async fn successful_mutation_appends_operations_to_the_bounded_audit() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    let operations = vec![set_life(18)];
    mutate_table(&state, command("table", operations.clone())).unwrap();

    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(table.debug.journal_request_bytes, 13);
    assert_eq!(table.debug.journal.len(), 1);
    assert_eq!(
        table.debug.journal.front().unwrap().kind,
        JournalKind::MutationCommitted {
            operations: operations.into_iter().map(TableMutation::Engine).collect(),
        }
    );
}

#[tokio::test]
async fn mutation_without_stream_subscribers_commits_and_journals_exactly_once() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let registry = lock(&state.reg);
        let table = registry.get("table").unwrap();
        assert_eq!(table.tx.receiver_count(), 0);
    }

    let receipt = mutate_table(&state, command("table", vec![set_life(18)])).unwrap();

    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((receipt.debug_revision, receipt.table_seq), (1, 1));
    assert_eq!(
        (table.debug.revision, table.seq, table.broadcast_seq),
        (1, 1, 1)
    );
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 18);
    assert_eq!(table.debug.journal.len(), 1);
    assert_eq!(table.debug.next_journal_ordinal, 1);
    assert_eq!(table.debug.journal_request_bytes, 13);
    assert_eq!(
        table.tx.len(),
        0,
        "failed zero-subscriber send retains no phantom frame"
    );
}

#[tokio::test]
async fn failed_mutation_at_the_journal_record_limit_rolls_back_the_full_table() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let mut registry = lock(&state.reg);
        let debug = &mut registry.get_mut("table").unwrap().debug;
        let filler = JournalRecord {
            ordinal: 0,
            timestamp_unix_ms: 0,
            debug_revision: 0,
            table_seq: 0,
            encoded_request_bytes: 0,
            kind: JournalKind::MutationCommitted { operations: vec![] },
        };
        debug.journal.resize(MAX_JOURNAL_RECORDS, filler);
    }
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = table_transaction_fingerprint(&state, "table");

    assert_eq!(
        mutate_table(&state, command("table", vec![set_life(1)])),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRecords,
        })
    );

    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn failed_mutation_at_the_journal_byte_limit_rolls_back_the_full_table() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let mut registry = lock(&state.reg);
        registry
            .get_mut("table")
            .unwrap()
            .debug
            .journal_request_bytes = MAX_JOURNAL_REQUEST_BYTES;
    }
    let before = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        mutate_table(&state, command("table", vec![set_life(1)])),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert_eq!((table.seq, table.broadcast_seq), (0, 0));
}

#[tokio::test]
async fn ordinary_mutation_preserves_logical_chrome_while_clear_pending_resets_it() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table
            .chrome
            .set_yields_for_test([true, false, false, false]);
        table.chrome.set_turn_yield_flag(1, true);
        table.chrome.set_dwell_flag(2, true);
    }
    mutate_table(&state, command("table", vec![set_life(19)])).unwrap();
    {
        let registry = lock(&state.reg);
        let table = registry.get("table").unwrap();
        assert_eq!(*table.chrome.yields(), [true, false, false, false]);
        assert_eq!(*table.chrome.turn_yields(), [false, true, false, false]);
        assert!(!table.chrome.any_dwell());
    }

    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let clear_receipt = mutate_table(
        &state,
        command(
            "table",
            vec![Mutation::ClearPendingOrchestration {
                clear_queued_triggers: true,
            }],
        ),
    )
    .unwrap();
    let clear_update = rx
        .try_recv()
        .expect("clear publishes one replacement snapshot");
    let PublishedUpdate::Snapshot(clear_snapshot) = clear_update.as_ref() else {
        panic!("clear publishes a snapshot")
    };
    assert_eq!(clear_snapshot.seq, clear_receipt.table_seq);

    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        assert_eq!(*table.chrome.yields(), [false; 4]);
        assert_eq!(*table.chrome.turn_yields(), [false; 4]);
        assert!(table.chrome.stack_hold().is_none());
        let holder = table.game.as_ref().unwrap().priority_holder();
        let (result, _) = TableSession::new(table).submit(Intent::PassPriority { player: holder });
        assert!(
            result.accepted,
            "ordinary intent succeeds after clear-pending"
        );
    }

    let ordinary_update = rx
        .try_recv()
        .expect("ordinary intent publishes the next delta");
    let PublishedUpdate::Delta { state, .. } = ordinary_update.as_ref() else {
        panic!("ordinary intent publishes a delta")
    };
    assert_eq!(state.seq, clear_receipt.table_seq + 1);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

fn cast_bear_table_with_armed_hold() -> Table {
    let mut table = Table::empty();
    let mut game = engine::Game::new();
    game.fund_mana(PlayerId(0));
    let bear = game.spawn_in_hand(PlayerId(0), cards::get_by_name("Grizzly Bears").unwrap());
    table.game = Some(game);
    let (result, _) = TableSession::new(&mut table).submit(Intent::Cast {
        player: PlayerId(0),
        object: bear,
        target: None,
        x: 0,
        modes: vec![],
        discard_cost: vec![],
        graveyard_exile: vec![],
        sacrifice_cost: vec![],
        kicked: false,
        bought_back: false,
        evoked: false,
        strive_count: 0,
        replicate_count: 0,
        multikicker_count: 0,
        alternative_cost: false,
    });
    assert!(result.accepted);
    let seq = table.seq;
    assert!(crate::session::arm_stack_resolution(&mut table, seq));
    table
}

#[tokio::test(start_paused = true)]
async fn debug_restore_invalidates_old_hold_tasks_and_polls_only_the_new_sequence() {
    let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
    let table = cast_bear_table_with_armed_hold();
    let source_seq = table.seq;
    assert!(lock(&state.reg).try_insert("table".into(), table));
    crate::session::schedule_armed_stack_resolution(state.clone(), "table".into(), source_seq);
    checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".into(),
            name: "held".into(),
            replace_existing: false,
            expected_table_seq: Some(source_seq),
            encoded_request_bytes: 1,
        },
    )
    .unwrap();
    mutate_table(&state, command("table", vec![set_life(17)])).unwrap();
    let before_restore_seq = lock(&state.reg).get("table").unwrap().seq;

    let receipt = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "held".into(),
            expected_debug_revision: Some(1),
            expected_table_seq: Some(before_restore_seq),
            encoded_request_bytes: 1,
        },
    )
    .unwrap();
    {
        let registry = lock(&state.reg);
        let table = registry.get("table").unwrap();
        assert_eq!(
            table.chrome.stack_hold().map(|(seq, _)| seq),
            Some(receipt.table_seq)
        );
        assert!(!table.chrome.any_dwell());
    }

    tokio::task::yield_now().await;
    tokio::time::advance(crate::session::STACK_HOLD + std::time::Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(
        table.seq,
        receipt.table_seq + 1,
        "only the fresh timer resolves once"
    );
    assert!(table.chrome.stack_hold().is_none());
    assert!(table.game.as_ref().unwrap().stack().is_empty());
}

#[tokio::test(start_paused = true)]
async fn debug_restore_commits_saved_hold_intent_without_timer_when_game_is_now_ineligible() {
    let state = AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"));
    let table = cast_bear_table_with_armed_hold();
    let source_seq = table.seq;
    assert!(lock(&state.reg).try_insert("table".into(), table));
    checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".into(),
            name: "held".into(),
            replace_existing: false,
            expected_table_seq: Some(source_seq),
            encoded_request_bytes: 1,
        },
    )
    .unwrap();
    {
        let mut registry = lock(&state.reg);
        registry
            .get_mut("table")
            .unwrap()
            .debug
            .checkpoints
            .get_mut("held")
            .unwrap()
            .game = engine::Game::with_players(2, 0);
    }

    let receipt = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "held".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(source_seq),
            encoded_request_bytes: 1,
        },
    )
    .unwrap();

    {
        let registry = lock(&state.reg);
        let table = registry.get("table").unwrap();
        assert_eq!(table.seq, receipt.table_seq);
        assert!(table.game.as_ref().unwrap().stack().is_empty());
        assert!(table.chrome.stack_hold().is_none());
        assert!(!table.chrome.any_dwell());
    }
    tokio::time::advance(crate::session::STACK_HOLD + std::time::Duration::from_millis(100)).await;
    tokio::task::yield_now().await;
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(
        table.seq, receipt.table_seq,
        "no post-lock poll was scheduled"
    );
    assert!(table.chrome.stack_hold().is_none());
}

#[tokio::test]
async fn concurrent_restore_with_identical_guards_has_exactly_one_winner() {
    let state = Arc::new(state_with_game("table", engine::Game::with_players(2, 0)).await);
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    mutate_table(&state, command("table", vec![set_life(7)])).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let state = Arc::clone(&state);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                restore_checkpoint(
                    &state,
                    RestoreCommand {
                        table_id: "table".into(),
                        name: "baseline".into(),
                        expected_debug_revision: Some(1),
                        expected_table_seq: Some(1),
                        encoded_request_bytes: 1,
                    },
                )
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(DebugFailure::Aborted {
                    reason: DebugAbortReason::DebugRevisionMismatch,
                    ..
                })
            ))
            .count(),
        1
    );
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (2, 2, 2)
    );
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
}

#[tokio::test]
async fn debug_restore_preflights_counter_and_journal_capacity_before_live_swap() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        table.seq = 3;
        table.broadcast_seq = u64::MAX;
    }
    let before = table_transaction_fingerprint(&state, "table");
    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(3),
            encoded_request_bytes: 1,
        },
    );
    assert!(
        matches!(result, Err(DebugFailure::FailedPrecondition { ref violations, .. })
        if violations.iter().any(|violation| violation.code == "broadcast_seq_exhausted"))
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((table.seq, table.broadcast_seq), (3, u64::MAX));
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
}

#[tokio::test]
async fn debug_restore_at_the_journal_record_limit_rolls_back_the_full_table() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    {
        let mut registry = lock(&state.reg);
        let debug = &mut registry.get_mut("table").unwrap().debug;
        let filler = debug.journal.front().unwrap().clone();
        debug.journal.resize(MAX_JOURNAL_RECORDS, filler);
    }
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = table_transaction_fingerprint(&state, "table");

    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            encoded_request_bytes: 1,
        },
    );

    assert_eq!(
        result,
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRecords,
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn debug_restore_at_the_journal_byte_limit_rolls_back_the_full_table() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .journal_request_bytes = MAX_JOURNAL_REQUEST_BYTES;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = table_transaction_fingerprint(&state, "table");

    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            encoded_request_bytes: 1,
        },
    );

    assert_eq!(
        result,
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn debug_restore_at_journal_ordinal_exhaustion_rolls_back_the_full_table() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .next_journal_ordinal = u64::MAX;
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = table_transaction_fingerprint(&state, "table");

    let result = restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".into(),
            name: "baseline".into(),
            expected_debug_revision: Some(0),
            expected_table_seq: Some(0),
            encoded_request_bytes: 1,
        },
    );

    assert!(
        matches!(result, Err(DebugFailure::FailedPrecondition { ref violations, .. })
        if violations.iter().any(|violation| violation.code == "journal_ordinal_exhausted"))
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn debug_restore_missing_checkpoint_stale_seq_and_full_journal_leave_no_trace() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 1)).unwrap();
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();

    let before_missing = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        restore_checkpoint(
            &state,
            RestoreCommand {
                table_id: "table".into(),
                name: "missing".into(),
                expected_debug_revision: Some(0),
                expected_table_seq: Some(0),
                encoded_request_bytes: 1,
            }
        ),
        Err(DebugFailure::CheckpointNotFound),
    );
    assert_eq!(
        table_transaction_fingerprint(&state, "table"),
        before_missing
    );

    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        let holder = table.game.as_ref().unwrap().priority_holder();
        let (result, _) = TableSession::new(table).submit(Intent::PassPriority { player: holder });
        assert!(result.accepted, "ordinary intent advances the live table");
    }
    let ordinary_update = rx.try_recv().expect("ordinary intent publishes one delta");
    let PublishedUpdate::Delta {
        state: ordinary_state,
        ..
    } = ordinary_update.as_ref()
    else {
        panic!("ordinary intent publishes a delta")
    };
    assert_eq!(ordinary_state.seq, 1);

    let before_stale = table_transaction_fingerprint(&state, "table");
    assert!(matches!(
        restore_checkpoint(
            &state,
            RestoreCommand {
                table_id: "table".into(),
                name: "baseline".into(),
                expected_debug_revision: Some(0),
                expected_table_seq: Some(0),
                encoded_request_bytes: 1,
            }
        ),
        Err(DebugFailure::Aborted {
            reason: DebugAbortReason::TableSeqMismatch,
            actual_table_seq: 1,
            ..
        })
    ));
    assert_eq!(table_transaction_fingerprint(&state, "table"), before_stale);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

    {
        let mut registry = lock(&state.reg);
        registry
            .get_mut("table")
            .unwrap()
            .debug
            .journal_request_bytes = MAX_JOURNAL_REQUEST_BYTES;
    }
    let before_full = table_transaction_fingerprint(&state, "table");
    assert_eq!(
        restore_checkpoint(
            &state,
            RestoreCommand {
                table_id: "table".into(),
                name: "baseline".into(),
                expected_debug_revision: Some(0),
                expected_table_seq: Some(1),
                encoded_request_bytes: 1,
            }
        ),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        }),
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before_full);
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert_eq!((table.seq, table.broadcast_seq), (1, 1));
    assert_eq!(table.game.as_ref().unwrap().life(PlayerId(0)), 20);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

fn table_command(table_id: &str, operations: Vec<TableMutation>) -> MutateCommand {
    MutateCommand {
        table_id: table_id.to_string(),
        expected_debug_revision: None,
        expected_table_seq: None,
        operations,
        encoded_request_bytes: 97,
    }
}

fn create_hand_card(object_id: u32, name: &str) -> TableMutation {
    TableMutation::Engine(Mutation::CreateCard {
        object_id,
        card_id: cards::get_by_name(name)
            .expect("fixture card")
            .id
            .to_string(),
        owner: PlayerId(0),
        controller: PlayerId(0),
        destination: DebugZone::Hand,
        commander: false,
        face_down: false,
    })
}

fn known_spell(
    entry_id: u64,
    from_object_id: u32,
    spell_object_id: u32,
) -> engine::debug::DebugStackEntrySpec {
    engine::debug::DebugStackEntrySpec::KnownSpell {
        entry_id: engine::StackEntryId(entry_id),
        from_object_id,
        spell_object_id,
        controller: PlayerId(0),
        targets: vec![],
        targets_second: vec![],
        x: 0,
    }
}

fn public_ghost(entry_id: u64) -> engine::debug::DebugStackEntrySpec {
    engine::debug::DebugStackEntrySpec::PublicGhost {
        entry_id: engine::StackEntryId(entry_id),
        controller: PlayerId(1),
        public: engine::PublicStackGhost {
            name: "Public fixture".to_string(),
            label: "Public no-op".to_string(),
            printing_id: "ghost-print".to_string(),
            card_id: None,
            printed_sentences: vec!["This fixture does nothing.".to_string()],
        },
    }
}

#[tokio::test]
async fn stack_and_print_transaction_publishes_one_private_filtered_snapshot_and_restores_together()
{
    let state = state_with_game("table", engine::Game::with_players(4, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 7)).unwrap();
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    let mut owner_rx = table.tx.subscribe();
    let mut opponent_rx = table.tx.subscribe();
    let mut spectator_rx = table.tx.subscribe();
    drop(registry);

    let names = [
        "Dark Ritual",
        "Vision Skeins",
        "Night's Whisper",
        "Harmonize",
        "Fog",
        "Time Walk",
    ];
    let prints = [
        "print-a", "print-b", "print-c", "print-d", "print-e", "print-f",
    ];
    let mut operations = vec![
        create_hand_card(0, "Shock"),
        TableMutation::Engine(Mutation::CreateCard {
            object_id: 1,
            card_id: cards::get_by_name("Plains").unwrap().id.to_string(),
            owner: PlayerId(0),
            controller: PlayerId(0),
            destination: DebugZone::Library,
            commander: false,
            face_down: false,
        }),
    ];
    operations.extend(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| create_hand_card(2 + index as u32, name)),
    );
    operations.extend(prints.iter().enumerate().map(|(index, printing_id)| {
        TableMutation::SetObjectPrintOverride {
            object_id: 2 + index as u32,
            printing_id: Some((*printing_id).to_string()),
        }
    }));
    let mut entries = (0..names.len())
        .map(|index| known_spell(10 + index as u64, 2 + index as u32, 8 + index as u32))
        .collect::<Vec<_>>();
    entries.push(public_ghost(16));
    operations.push(TableMutation::ReplaceStack(entries));

    let receipt = mutate_table(&state, table_command("table", operations.clone())).unwrap();
    assert_eq!((receipt.debug_revision, receipt.table_seq), (1, 1));
    assert_eq!(receipt.applied_operation_count, operations.len());

    for (viewer, rx) in [
        (Some(PlayerId(0)), &mut owner_rx),
        (Some(PlayerId(1)), &mut opponent_rx),
        (None, &mut spectator_rx),
    ] {
        let update = rx.try_recv().expect("one replacement snapshot");
        let StreamFrame::Snapshot { state: visible, .. } = frame_for_update(viewer, &update) else {
            panic!("debug commit publishes a snapshot");
        };
        assert_eq!(visible.stack.len(), 7);
        assert_eq!(
            visible
                .stack
                .iter()
                .map(|entry| entry.entry_id)
                .collect::<Vec<_>>(),
            (10..=16).collect::<Vec<_>>()
        );
        assert_eq!(
            visible
                .stack
                .iter()
                .map(|entry| entry.source)
                .collect::<Vec<_>>(),
            vec![
                Some(8),
                Some(9),
                Some(10),
                Some(11),
                Some(12),
                Some(13),
                None
            ]
        );
        assert_eq!(
            visible
                .stack
                .iter()
                .map(|entry| entry.print.as_str())
                .collect::<Vec<_>>(),
            vec![
                "print-a",
                "print-b",
                "print-c",
                "print-d",
                "print-e",
                "print-f",
                "ghost-print"
            ]
        );
        let private_ids = visible
            .objects
            .iter()
            .filter(|object| object.id <= 1)
            .map(|object| object.id)
            .collect::<Vec<_>>();
        if viewer == Some(PlayerId(0)) {
            assert_eq!(private_ids, vec![0]);
        } else {
            assert!(private_ids.is_empty());
        }
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    {
        let registry = lock(&state.reg);
        let table = registry.get("table").unwrap();
        assert_eq!(
            (table.seq, table.broadcast_seq, table.debug.revision),
            (1, 1, 1)
        );
        assert_eq!(table.debug.journal.len(), 2);
        assert_eq!(
            table.debug.object_prints,
            prints
                .iter()
                .enumerate()
                .map(|(index, print)| (8 + index as u32, (*print).to_string()))
                .collect()
        );
        assert_eq!(
            table.debug.journal.back().unwrap().kind,
            JournalKind::MutationCommitted { operations }
        );
    }

    restore_checkpoint(
        &state,
        RestoreCommand {
            table_id: "table".to_string(),
            name: "baseline".to_string(),
            expected_debug_revision: Some(1),
            expected_table_seq: Some(1),
            encoded_request_bytes: 11,
        },
    )
    .unwrap();
    let registry = lock(&state.reg);
    let table = registry.get("table").unwrap();
    assert!(
        engine::debug::inspect(table.game.as_ref().unwrap())
            .stack
            .is_empty()
    );
    assert!(table.debug.object_prints.is_empty());
    assert_eq!(
        (table.seq, table.broadcast_seq, table.debug.revision),
        (2, 2, 2)
    );
    drop(registry);
    for rx in [&mut owner_rx, &mut opponent_rx, &mut spectator_rx] {
        let update = rx.try_recv().expect("one restore snapshot");
        let PublishedUpdate::Snapshot(snapshot) = update.as_ref() else {
            panic!("restore publishes a snapshot");
        };
        assert!(snapshot.object_print_overrides.is_empty());
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }
}

#[tokio::test]
async fn late_stack_or_print_failure_reports_global_index_and_preserves_full_fingerprint() {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    checkpoint_table(&state, checkpoint_command("baseline", 5)).unwrap();
    let mut rx = lock(&state.reg).get("table").unwrap().tx.subscribe();
    let before = table_transaction_fingerprint(&state, "table");
    let operations = vec![
        create_hand_card(0, "Dark Ritual"),
        TableMutation::SetObjectPrintOverride {
            object_id: 0,
            printing_id: Some("source-print".to_string()),
        },
        TableMutation::ReplaceStack(vec![known_spell(1, 0, 1)]),
        TableMutation::SetObjectPrintOverride {
            object_id: 999,
            printing_id: Some("missing".to_string()),
        },
    ];

    assert_eq!(
        mutate_table(&state, table_command("table", operations)),
        Err(DebugFailure::NotFound {
            operation_index: Some(3),
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

    let late_engine = vec![
        create_hand_card(0, "Dark Ritual"),
        TableMutation::SetObjectPrintOverride {
            object_id: 0,
            printing_id: Some("source-print".to_string()),
        },
        TableMutation::Engine(Mutation::SetController {
            object_id: 999,
            controller: PlayerId(0),
        }),
    ];
    assert_eq!(
        mutate_table(&state, table_command("table", late_engine)),
        Err(DebugFailure::NotFound {
            operation_index: Some(2),
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

    let bad_stack = vec![
        TableMutation::Engine(set_life(19)),
        TableMutation::ReplaceStack(vec![public_ghost(0)]),
    ];
    assert_eq!(
        mutate_table(&state, table_command("table", bad_stack)),
        Err(DebugFailure::Invalid {
            operation_index: Some(1),
            reason: ErrorReason::InvalidValue,
        })
    );
    assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn print_override_validation_cleanup_and_checkpoint_slots_are_exact() {
    let oversized = "x".repeat(MAX_OBJECT_PRINT_ID_BYTES + 1);
    for invalid in ["", "   ", oversized.as_str()] {
        let state = state_with_object_slots("table", 1).await;
        let before = table_transaction_fingerprint(&state, "table");
        let error = mutate_table(
            &state,
            table_command(
                "table",
                vec![TableMutation::SetObjectPrintOverride {
                    object_id: 0,
                    printing_id: Some(invalid.to_string()),
                }],
            ),
        );
        assert_eq!(
            error,
            Err(DebugFailure::Invalid {
                operation_index: Some(0),
                reason: ErrorReason::InvalidValue,
            })
        );
        assert_eq!(table_transaction_fingerprint(&state, "table"), before);
    }

    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    mutate_table(
        &state,
        table_command(
            "table",
            vec![
                create_hand_card(0, "Dark Ritual"),
                TableMutation::SetObjectPrintOverride {
                    object_id: 0,
                    printing_id: Some("spell-print".to_string()),
                },
                TableMutation::PushStack(known_spell(1, 0, 1)),
            ],
        ),
    )
    .unwrap();
    {
        let table = lock(&state.reg);
        assert_eq!(
            table.get("table").unwrap().debug.object_prints,
            [(1, "spell-print".to_string())].into_iter().collect()
        );
    }
    let checkpoint = checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".to_string(),
            name: "with-print".to_string(),
            replace_existing: false,
            expected_table_seq: Some(1),
            encoded_request_bytes: 5,
        },
    )
    .unwrap();
    assert_eq!(
        checkpoint.object_slots, 3,
        "two arena slots plus one exact print"
    );
    assert_eq!(
        lock(&state.reg)
            .get("table")
            .unwrap()
            .debug
            .checkpoint_object_slots,
        3
    );

    mutate_table(
        &state,
        table_command("table", vec![TableMutation::PopStack { count: 1 }]),
    )
    .unwrap();
    assert!(
        lock(&state.reg)
            .get("table")
            .unwrap()
            .debug
            .object_prints
            .is_empty()
    );
}

#[tokio::test]
async fn ordinary_spell_resolution_prunes_inherited_print_before_print_only_mutation_and_checkpoint()
 {
    let state = state_with_game("table", engine::Game::with_players(2, 0)).await;
    mutate_table(
        &state,
        table_command(
            "table",
            vec![
                create_hand_card(0, "Dark Ritual"),
                TableMutation::Engine(Mutation::CreateCard {
                    object_id: 1,
                    card_id: cards::get_by_name("Plains").unwrap().id.to_string(),
                    owner: PlayerId(0),
                    controller: PlayerId(0),
                    destination: DebugZone::Hand,
                    commander: false,
                    face_down: false,
                }),
                TableMutation::SetObjectPrintOverride {
                    object_id: 0,
                    printing_id: Some("spell-print".to_string()),
                },
                TableMutation::PushStack(known_spell(1, 0, 2)),
            ],
        ),
    )
    .unwrap();
    assert_eq!(
        lock(&state.reg).get("table").unwrap().debug.object_prints,
        [(2, "spell-print".to_string())].into_iter().collect()
    );

    {
        let mut registry = lock(&state.reg);
        let table = registry.get_mut("table").unwrap();
        for _ in 0..4 {
            if engine::debug::inspect(table.game.as_ref().unwrap())
                .stack
                .is_empty()
            {
                break;
            }
            let holder = table.game.as_ref().unwrap().priority_holder();
            let (result, _) =
                TableSession::new(table).submit_system(Intent::PassPriority { player: holder });
            assert!(result.accepted);
        }
        assert!(
            engine::debug::inspect(table.game.as_ref().unwrap())
                .stack
                .is_empty(),
            "ordinary priority passes resolve the debug-created spell"
        );
        assert!(
            table.debug.object_prints.is_empty(),
            "the exact spell override is pruned at the accepted-intent publication boundary"
        );
    }

    mutate_table(
        &state,
        table_command(
            "table",
            vec![TableMutation::SetObjectPrintOverride {
                object_id: 1,
                printing_id: Some("plains-print".to_string()),
            }],
        ),
    )
    .expect("a later print-only transaction is not poisoned by the resolved spell");
    checkpoint_table(
        &state,
        CheckpointCommand {
            table_id: "table".to_string(),
            name: "after-resolution".to_string(),
            replace_existing: false,
            expected_table_seq: None,
            encoded_request_bytes: 10,
        },
    )
    .expect("a later checkpoint validates the exact-live override map");
}

#[tokio::test]
async fn print_only_mutation_prunes_preexisting_stale_overrides_before_validation() {
    let state = state_with_object_slots("table", 1).await;
    lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .object_prints
        .insert(99, "stale-print".to_string());

    mutate_table(
        &state,
        table_command(
            "table",
            vec![TableMutation::SetObjectPrintOverride {
                object_id: 0,
                printing_id: Some("live-print".to_string()),
            }],
        ),
    )
    .expect("print-only transactions self-heal stale inherited state");
    assert_eq!(
        lock(&state.reg).get("table").unwrap().debug.object_prints,
        [(0, "live-print".to_string())].into_iter().collect()
    );
}

#[tokio::test]
async fn clearing_a_stale_print_override_is_idempotent_recovery() {
    let state = state_with_object_slots("table", 1).await;
    lock(&state.reg)
        .get_mut("table")
        .unwrap()
        .debug
        .object_prints
        .insert(99, "stale-print".to_string());

    mutate_table(
        &state,
        table_command(
            "table",
            vec![TableMutation::SetObjectPrintOverride {
                object_id: 99,
                printing_id: None,
            }],
        ),
    )
    .expect("clearing an already-dead exact object is allowed");
    assert!(
        lock(&state.reg)
            .get("table")
            .unwrap()
            .debug
            .object_prints
            .is_empty()
    );
}
