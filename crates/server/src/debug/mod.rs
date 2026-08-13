//! Debug-build-only atomic table mutation transaction.

use std::collections::{BTreeMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::debug::{EditError, ErrorReason, Mutation, Violation};
use engine::{Game, PlayerId};
use schema::complete_visible;

use crate::chrome::{ChromeState, DebugChromeSnapshot};
use crate::session::{
    PublishedState, PublishedUpdate, STACK_HOLD, arm_stack_resolution,
    schedule_armed_stack_resolution,
};
use crate::stream::{table_view_extras, view_extras};
use crate::{AppState, Table, lock};

pub(crate) const MAX_CHECKPOINTS_PER_TABLE: usize = 16;
pub(crate) const MAX_OBJECT_SLOTS_PER_CHECKPOINT: usize = 4_096;
pub(crate) const MAX_OBJECT_SLOTS_ACROSS_CHECKPOINTS: usize = 32_768;
pub(crate) const MAX_CHECKPOINT_NAME_BYTES: usize = 64;
pub(crate) const MAX_JOURNAL_RECORDS: usize = 1_024;
pub(crate) const MAX_JOURNAL_REQUEST_BYTES: usize = 1_048_576;

#[derive(Clone)]
pub(crate) struct Checkpoint {
    pub(crate) game: Game,
    pub(crate) chrome: DebugChromeSnapshot,
    pub(crate) object_prints: schema::ObjectPrintOverrides,
    pub(crate) source_table_seq: u64,
    pub(crate) object_slots: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JournalKind {
    MutationCommitted { operations: Vec<Mutation> },
    CheckpointCreated { name: String, replaced: bool },
    CheckpointRestored { name: String, source_table_seq: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JournalRecord {
    pub(crate) ordinal: u64,
    pub(crate) timestamp_unix_ms: u64,
    pub(crate) debug_revision: u64,
    pub(crate) table_seq: u64,
    pub(crate) encoded_request_bytes: usize,
    pub(crate) kind: JournalKind,
}

pub struct TableDebugState {
    pub revision: u64,
    pub debug_mutated: bool,
    /// Live exact-object presentation overlays, swapped atomically with debug game replacement.
    pub(crate) object_prints: schema::ObjectPrintOverrides,
    pub(crate) checkpoints: BTreeMap<String, Checkpoint>,
    pub(crate) checkpoint_object_slots: usize,
    pub(crate) journal: VecDeque<JournalRecord>,
    pub(crate) journal_request_bytes: usize,
    pub(crate) next_journal_ordinal: u64,
}

#[allow(clippy::derivable_impls)]
impl Default for TableDebugState {
    fn default() -> Self {
        Self {
            revision: 0,
            debug_mutated: false,
            object_prints: Default::default(),
            checkpoints: BTreeMap::new(),
            checkpoint_object_slots: 0,
            journal: VecDeque::new(),
            journal_request_bytes: 0,
            next_journal_ordinal: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CheckpointCommand {
    pub(crate) table_id: String,
    pub(crate) name: String,
    pub(crate) replace_existing: bool,
    pub(crate) expected_table_seq: Option<u64>,
    pub(crate) encoded_request_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointReceipt {
    pub(crate) debug_revision: u64,
    pub(crate) table_seq: u64,
    pub(crate) object_slots: u32,
    pub(crate) replaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceLimit {
    CheckpointCount,
    CheckpointObjectSlots,
    JournalRecords,
    JournalRequestBytes,
}

#[derive(Debug, Clone)]
pub struct MutateCommand {
    pub table_id: String,
    pub expected_debug_revision: Option<u64>,
    pub expected_table_seq: Option<u64>,
    pub operations: Vec<Mutation>,
    pub encoded_request_bytes: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RestoreCommand {
    pub(crate) table_id: String,
    pub(crate) name: String,
    pub(crate) expected_debug_revision: Option<u64>,
    pub(crate) expected_table_seq: Option<u64>,
    pub(crate) encoded_request_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestoreReceipt {
    pub(crate) debug_revision: u64,
    pub(crate) table_seq: u64,
    pub(crate) restored_source_table_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RestoreTail {
    table_id: String,
    hold_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutateReceipt {
    pub debug_revision: u64,
    pub table_seq: u64,
    pub applied_operation_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAbortReason {
    DebugRevisionMismatch,
    TableSeqMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugFailure {
    CheckpointNotFound,
    CheckpointAlreadyExists,
    InvalidCheckpointName,
    ResourceExhausted {
        reason: ResourceLimit,
    },
    NotFound {
        operation_index: Option<usize>,
    },
    AlreadyExists {
        operation_index: usize,
    },
    Aborted {
        reason: DebugAbortReason,
        actual_debug_revision: u64,
        actual_table_seq: u64,
    },
    Invalid {
        operation_index: Option<usize>,
        reason: ErrorReason,
    },
    FailedPrecondition {
        operation_index: Option<usize>,
        violations: Vec<Violation>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JournalPreflight {
    request_bytes_total: usize,
    ordinal: u64,
    next_ordinal: u64,
}

pub(crate) fn preflight_journal(
    debug: &TableDebugState,
    encoded_request_bytes: usize,
) -> Result<JournalPreflight, DebugFailure> {
    if debug.journal.len() >= MAX_JOURNAL_RECORDS {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRecords,
        });
    }
    let Some(request_bytes_total) = debug
        .journal_request_bytes
        .checked_add(encoded_request_bytes)
    else {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        });
    };
    if request_bytes_total > MAX_JOURNAL_REQUEST_BYTES {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::JournalRequestBytes,
        });
    }
    let next_ordinal = checked_increment(debug.next_journal_ordinal, "journal_ordinal_exhausted")?;
    Ok(JournalPreflight {
        request_bytes_total,
        ordinal: debug.next_journal_ordinal,
        next_ordinal,
    })
}

pub(crate) fn append_journal(
    debug: &mut TableDebugState,
    preflight: JournalPreflight,
    record: JournalRecord,
) {
    debug_assert_eq!(record.ordinal, preflight.ordinal);
    debug_assert_eq!(
        record.encoded_request_bytes + debug.journal_request_bytes,
        preflight.request_bytes_total
    );
    debug.journal.push_back(record);
    debug.journal_request_bytes = preflight.request_bytes_total;
    debug.next_journal_ordinal = preflight.next_ordinal;
}

pub(crate) fn timestamp_unix_ms_at(time: SystemTime) -> u64 {
    let Ok(since_epoch) = time.duration_since(UNIX_EPOCH) else {
        return 0;
    };
    u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX)
}

fn timestamp_unix_ms() -> u64 {
    timestamp_unix_ms_at(SystemTime::now())
}

pub(crate) fn checkpoint_table(
    state: &AppState,
    command: CheckpointCommand,
) -> Result<CheckpointReceipt, DebugFailure> {
    let mut registry = lock(&state.reg);
    let Some(table) = registry.get_mut(&command.table_id) else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    let Some(game) = table.game.as_ref() else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    if command
        .expected_table_seq
        .is_some_and(|expected| expected != table.seq)
    {
        return Err(DebugFailure::Aborted {
            reason: DebugAbortReason::TableSeqMismatch,
            actual_debug_revision: table.debug.revision,
            actual_table_seq: table.seq,
        });
    }
    if !valid_checkpoint_name(&command.name) {
        return Err(DebugFailure::InvalidCheckpointName);
    }

    let object_slots = engine::debug::object_slot_count(game);
    if object_slots > MAX_OBJECT_SLOTS_PER_CHECKPOINT {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        });
    }
    let existing = table.debug.checkpoints.get(&command.name);
    if existing.is_some() && !command.replace_existing {
        return Err(DebugFailure::CheckpointAlreadyExists);
    }
    if existing.is_none() && table.debug.checkpoints.len() >= MAX_CHECKPOINTS_PER_TABLE {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointCount,
        });
    }
    let old_slots = existing.map_or(0, |checkpoint| checkpoint.object_slots);
    let Some(next_object_slots) = table
        .debug
        .checkpoint_object_slots
        .checked_sub(old_slots)
        .and_then(|slots| slots.checked_add(object_slots))
    else {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        });
    };
    if next_object_slots > MAX_OBJECT_SLOTS_ACROSS_CHECKPOINTS {
        return Err(DebugFailure::ResourceExhausted {
            reason: ResourceLimit::CheckpointObjectSlots,
        });
    }
    let journal_preflight = preflight_journal(&table.debug, command.encoded_request_bytes)?;

    let replaced = existing.is_some();
    let checkpoint = Checkpoint {
        game: game.clone(),
        chrome: table.chrome.debug_snapshot(),
        object_prints: table.debug.object_prints.clone(),
        source_table_seq: table.seq,
        object_slots,
    };
    table
        .debug
        .checkpoints
        .insert(command.name.clone(), checkpoint);
    table.debug.checkpoint_object_slots = next_object_slots;
    let journal_record = JournalRecord {
        ordinal: journal_preflight.ordinal,
        timestamp_unix_ms: timestamp_unix_ms(),
        debug_revision: table.debug.revision,
        table_seq: table.seq,
        encoded_request_bytes: command.encoded_request_bytes,
        kind: JournalKind::CheckpointCreated {
            name: command.name,
            replaced,
        },
    };
    append_journal(&mut table.debug, journal_preflight, journal_record);

    Ok(CheckpointReceipt {
        debug_revision: table.debug.revision,
        table_seq: table.seq,
        object_slots: u32::try_from(object_slots)
            .expect("checkpoint slot bound is smaller than u32::MAX"),
        replaced,
    })
}

fn valid_checkpoint_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_CHECKPOINT_NAME_BYTES
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'))
}

pub fn mutate_table(
    state: &AppState,
    command: MutateCommand,
) -> Result<MutateReceipt, DebugFailure> {
    let mut registry = lock(&state.reg);
    let Some(table) = registry.get_mut(&command.table_id) else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    check_guard_values(
        table,
        command.expected_debug_revision,
        command.expected_table_seq,
    )?;
    let Some(game) = table.game.as_ref() else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    let mut candidate_game = game.clone();

    if let Err(error) = engine::debug::apply_operations(&mut candidate_game, &command.operations) {
        return Err(map_edit_error(&candidate_game, &command.operations, error));
    }
    if let Err(violations) = engine::debug::validate_structural(&candidate_game) {
        return Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            violations,
        });
    }

    let clear_chrome = command
        .operations
        .iter()
        .any(|operation| matches!(operation, Mutation::ClearPendingOrchestration { .. }));
    let snapshot = if clear_chrome {
        DebugChromeSnapshot::default()
    } else {
        table.chrome.debug_snapshot()
    };
    projection_sweep_with_logical_chrome(
        &candidate_game,
        table,
        snapshot,
        table.current_object_print_overrides(),
    )?;

    // Preflight every bounded/counted commit fact before touching the live table.
    let next_table_seq = checked_increment(table.seq, "table_seq_exhausted")?;
    let next_broadcast_seq = checked_increment(table.broadcast_seq, "broadcast_seq_exhausted")?;
    let next_debug_revision = checked_increment(table.debug.revision, "debug_revision_exhausted")?;
    let journal_preflight = preflight_journal(&table.debug, command.encoded_request_bytes)?;

    let mut candidate_chrome = ChromeState::default();
    candidate_chrome.restore_debug_snapshot(snapshot);
    table.game = Some(candidate_game);
    table.chrome = candidate_chrome;
    table.seq = next_table_seq;
    table.broadcast_seq = next_broadcast_seq;
    table.debug.revision = next_debug_revision;
    table.debug.debug_mutated = true;
    let schedule_hold = snapshot.hold_requested && arm_stack_resolution(table, next_table_seq);
    let journal_record = JournalRecord {
        ordinal: journal_preflight.ordinal,
        timestamp_unix_ms: timestamp_unix_ms(),
        debug_revision: next_debug_revision,
        table_seq: next_table_seq,
        encoded_request_bytes: command.encoded_request_bytes,
        kind: JournalKind::MutationCommitted {
            operations: command.operations.clone(),
        },
    };
    append_journal(&mut table.debug, journal_preflight, journal_record);
    publish_snapshot(table);

    let receipt = MutateReceipt {
        debug_revision: next_debug_revision,
        table_seq: next_table_seq,
        applied_operation_count: command.operations.len(),
    };
    let tail = RestoreTail {
        table_id: command.table_id,
        hold_seq: schedule_hold.then_some(next_table_seq),
    };
    drop(registry);
    run_restore_tail(state, tail);
    Ok(receipt)
}

pub(crate) fn restore_checkpoint(
    state: &AppState,
    command: RestoreCommand,
) -> Result<RestoreReceipt, DebugFailure> {
    let mut registry = lock(&state.reg);
    let Some(table) = registry.get_mut(&command.table_id) else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    // Revision is intentionally authoritative when both optimistic guards are stale.
    check_guard_values(
        table,
        command.expected_debug_revision,
        command.expected_table_seq,
    )?;
    let Some(checkpoint) = table.debug.checkpoints.get(&command.name).cloned() else {
        return Err(DebugFailure::CheckpointNotFound);
    };
    let candidate_game = checkpoint.game;
    if let Err(violations) = engine::debug::validate_structural(&candidate_game) {
        return Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            violations,
        });
    }

    projection_sweep_with_logical_chrome(
        &candidate_game,
        table,
        checkpoint.chrome,
        &checkpoint.object_prints,
    )?;

    // The stored source sequence is provenance only. Transport always advances from live values.
    let next_table_seq = checked_increment(table.seq, "table_seq_exhausted")?;
    let next_broadcast_seq = checked_increment(table.broadcast_seq, "broadcast_seq_exhausted")?;
    let next_debug_revision = checked_increment(table.debug.revision, "debug_revision_exhausted")?;
    let journal_preflight = preflight_journal(&table.debug, command.encoded_request_bytes)?;

    let mut candidate_chrome = ChromeState::default();
    candidate_chrome.restore_debug_snapshot(checkpoint.chrome);
    table.game = Some(candidate_game);
    table.chrome = candidate_chrome;
    table.seq = next_table_seq;
    table.broadcast_seq = next_broadcast_seq;
    table.debug.revision = next_debug_revision;
    table.debug.debug_mutated = true;
    table.debug.object_prints = checkpoint.object_prints;
    let schedule_hold =
        checkpoint.chrome.hold_requested && arm_stack_resolution(table, next_table_seq);
    let journal_record = JournalRecord {
        ordinal: journal_preflight.ordinal,
        timestamp_unix_ms: timestamp_unix_ms(),
        debug_revision: next_debug_revision,
        table_seq: next_table_seq,
        encoded_request_bytes: command.encoded_request_bytes,
        kind: JournalKind::CheckpointRestored {
            name: command.name,
            source_table_seq: checkpoint.source_table_seq,
        },
    };
    append_journal(&mut table.debug, journal_preflight, journal_record);
    publish_snapshot(table);

    let receipt = RestoreReceipt {
        debug_revision: next_debug_revision,
        table_seq: next_table_seq,
        restored_source_table_seq: checkpoint.source_table_seq,
    };
    let tail = RestoreTail {
        table_id: command.table_id,
        hold_seq: schedule_hold.then_some(next_table_seq),
    };
    drop(registry);
    run_restore_tail(state, tail);
    Ok(receipt)
}

fn check_guard_values(
    table: &Table,
    expected_debug_revision: Option<u64>,
    expected_table_seq: Option<u64>,
) -> Result<(), DebugFailure> {
    if expected_debug_revision.is_some_and(|expected| expected != table.debug.revision) {
        return Err(DebugFailure::Aborted {
            reason: DebugAbortReason::DebugRevisionMismatch,
            actual_debug_revision: table.debug.revision,
            actual_table_seq: table.seq,
        });
    }
    if expected_table_seq.is_some_and(|expected| expected != table.seq) {
        return Err(DebugFailure::Aborted {
            reason: DebugAbortReason::TableSeqMismatch,
            actual_debug_revision: table.debug.revision,
            actual_table_seq: table.seq,
        });
    }
    Ok(())
}

fn run_restore_tail(state: &AppState, tail: RestoreTail) {
    let Some(seq) = tail.hold_seq else {
        return;
    };
    // Domain helpers are also used by synchronous tests. Production calls always have a runtime;
    // without one the hold remains safely stamped and a later authoritative action invalidates it.
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    schedule_armed_stack_resolution(state.clone(), tail.table_id, seq);
}

fn checked_increment(value: u64, code: &'static str) -> Result<u64, DebugFailure> {
    value
        .checked_add(1)
        .ok_or_else(|| DebugFailure::FailedPrecondition {
            operation_index: None,
            violations: vec![Violation {
                code,
                message: "authoritative revision cannot advance".to_string(),
            }],
        })
}

fn map_edit_error(candidate: &Game, operations: &[Mutation], error: EditError) -> DebugFailure {
    match error.reason {
        ErrorReason::UnknownEntity => DebugFailure::NotFound {
            operation_index: error.operation_index,
        },
        ErrorReason::DuplicateId => match error.operation_index {
            Some(operation_index)
                if matches!(
                    operations.get(operation_index),
                    Some(Mutation::CreateCard { .. })
                ) =>
            {
                DebugFailure::AlreadyExists { operation_index }
            }
            operation_index => DebugFailure::Invalid {
                operation_index,
                reason: ErrorReason::DuplicateId,
            },
        },
        ErrorReason::InvalidValue if error.operation_index.is_none() => {
            match engine::debug::validate_structural(candidate) {
                Ok(()) => DebugFailure::Invalid {
                    operation_index: None,
                    reason: ErrorReason::InvalidValue,
                },
                Err(violations) => DebugFailure::FailedPrecondition {
                    operation_index: None,
                    violations,
                },
            }
        }
        reason => DebugFailure::Invalid {
            operation_index: error.operation_index,
            reason,
        },
    }
}

/// Exercise the production projection for every seat and a spectator before committing.
///
/// Only this projection boundary catches unwind: editor, validation, swapping, counters, chrome,
/// and publication retain their normal panic semantics.
pub fn projection_sweep(game: &Game, table: &Table) -> Result<(), DebugFailure> {
    let extras = table_view_extras(table);
    projection_sweep_with_extras(game, &extras)
}

fn projection_sweep_with_logical_chrome(
    game: &Game,
    table: &Table,
    chrome: DebugChromeSnapshot,
    object_prints: &schema::ObjectPrintOverrides,
) -> Result<(), DebugFailure> {
    let hold_ms = if chrome.hold_requested {
        u32::try_from(STACK_HOLD.as_millis()).expect("stack hold fits u32 milliseconds")
    } else {
        0
    };
    let extras = view_extras(
        &chrome.yields,
        &chrome.turn_yields,
        &table.seats,
        hold_ms,
        &table.prints,
        object_prints,
    );
    projection_sweep_with_extras(game, &extras)
}

fn projection_sweep_with_extras(
    game: &Game,
    extras: &schema::ViewExtras,
) -> Result<(), DebugFailure> {
    let projected = catch_unwind(AssertUnwindSafe(|| {
        for seat in 0..game.player_count() {
            let _ = complete_visible(game, Some(PlayerId(seat as u8)), extras);
        }
        let _ = complete_visible(game, None, extras);
    }));
    if projected.is_err() {
        return Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            violations: vec![Violation {
                code: "projection_failed",
                message: "candidate projection failed".to_string(),
            }],
        });
    }
    Ok(())
}

fn publish_snapshot(table: &Table) {
    let Some(game) = table.game.as_ref() else {
        return;
    };
    let state = PublishedState {
        seq: table.seq,
        broadcast_seq: table.broadcast_seq,
        game: game.clone(),
        yields: *table.chrome.yields(),
        turn_yields: *table.chrome.turn_yields(),
        stack_hold_remaining_ms: table.stack_hold_remaining_ms(),
        seats: table.seats.clone(),
        prints: table.prints.clone(),
        object_print_overrides: table.current_object_print_overrides().clone(),
    };
    let _ = table.tx.send(Arc::new(PublishedUpdate::Snapshot(state)));
}

#[cfg(test)]
mod tests;
