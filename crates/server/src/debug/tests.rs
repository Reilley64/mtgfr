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
        operations,
    }
}

fn assert_debug_state_is_empty(debug: &TableDebugState) {
    assert_eq!(debug.revision, 0);
    assert!(!debug.debug_mutated);
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
    source_table_seq: u64,
    object_slots: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct DebugStorageFingerprint {
    revision: u64,
    debug_mutated: bool,
    checkpoint_object_slots: usize,
    checkpoints: Vec<CheckpointFingerprint>,
    journal: Vec<JournalRecord>,
    journal_request_bytes: usize,
    next_journal_ordinal: u64,
}

fn debug_storage_fingerprint(state: &AppState, table_id: &str) -> DebugStorageFingerprint {
    let registry = lock(&state.reg);
    let debug = &registry.get(table_id).unwrap().debug;
    DebugStorageFingerprint {
        revision: debug.revision,
        debug_mutated: debug.debug_mutated,
        checkpoint_object_slots: debug.checkpoint_object_slots,
        checkpoints: debug
            .checkpoints
            .iter()
            .map(|(name, checkpoint)| CheckpointFingerprint {
                name: name.clone(),
                game: engine::debug::inspect(&checkpoint.game),
                chrome: checkpoint.chrome,
                source_table_seq: checkpoint.source_table_seq,
                object_slots: checkpoint.object_slots,
            })
            .collect(),
        journal: debug.journal.iter().cloned().collect(),
        journal_request_bytes: debug.journal_request_bytes,
        next_journal_ordinal: debug.next_journal_ordinal,
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
async fn transaction_success_swaps_once_clears_chrome_and_publishes_one_fresh_snapshot() {
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
        assert_eq!(*table.chrome.yields(), [false; 4]);
        assert_eq!(*table.chrome.turn_yields(), [false; 4]);
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
        let before = debug_storage_fingerprint(&state, "table");
        assert_eq!(
            checkpoint_table(&state, checkpoint_command(invalid, 1)),
            Err(DebugFailure::InvalidCheckpointName),
        );
        assert_eq!(debug_storage_fingerprint(&state, "table"), before);
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
    let before = debug_storage_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("baseline", 3)),
        Err(DebugFailure::CheckpointAlreadyExists),
    );
    assert_eq!(debug_storage_fingerprint(&state, "table"), before);

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
    let before = debug_storage_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("overflow", 0)),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointCount,
        }),
    );
    assert_eq!(debug_storage_fingerprint(&state, "table"), before);
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
    let before = debug_storage_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, checkpoint_command("too-large", 0)),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        }),
    );
    assert_eq!(debug_storage_fingerprint(&state, "table"), before);
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
    let before = debug_storage_fingerprint(&state, "table");
    assert_eq!(
        checkpoint_table(&state, replacement),
        Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        }),
    );
    assert_eq!(debug_storage_fingerprint(&state, "table"), before);
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
    let before = debug_storage_fingerprint(&state, "table");

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
    assert_eq!(debug_storage_fingerprint(&state, "table"), before);
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
