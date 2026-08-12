//! Debug-build-only atomic table mutation transaction.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use engine::debug::{EditError, ErrorReason, Mutation, Violation};
use engine::{Game, PlayerId};
use schema::complete_visible;

use crate::session::{PublishedState, PublishedUpdate};
use crate::stream::table_view_extras;
use crate::{AppState, Table, lock};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableDebugState {
    pub revision: u64,
    pub debug_mutated: bool,
}

#[derive(Debug, Clone)]
pub struct MutateCommand {
    pub table_id: String,
    pub expected_debug_revision: Option<u64>,
    pub expected_table_seq: Option<u64>,
    pub operations: Vec<Mutation>,
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
    check_guards(table, &command)?;
    let Some(game) = table.game.as_ref() else {
        return Err(DebugFailure::NotFound {
            operation_index: None,
        });
    };
    let mut candidate = game.clone();

    if let Err(error) = engine::debug::apply_operations(&mut candidate, &command.operations) {
        return Err(map_edit_error(&candidate, &command.operations, error));
    }
    if let Err(violations) = engine::debug::validate_structural(&candidate) {
        return Err(DebugFailure::FailedPrecondition {
            operation_index: None,
            violations,
        });
    }
    projection_sweep(&candidate, table)?;

    let next_table_seq = checked_increment(table.seq, "table_seq_exhausted")?;
    let next_broadcast_seq = checked_increment(table.broadcast_seq, "broadcast_seq_exhausted")?;
    let next_debug_revision = checked_increment(table.debug.revision, "debug_revision_exhausted")?;

    table.game = Some(candidate);
    table.seq = next_table_seq;
    table.broadcast_seq = next_broadcast_seq;
    table.debug.revision = next_debug_revision;
    table.debug.debug_mutated = true;
    table.chrome.clear_for_debug_commit();
    publish_snapshot(table);

    Ok(MutateReceipt {
        debug_revision: next_debug_revision,
        table_seq: next_table_seq,
        applied_operation_count: command.operations.len(),
    })
}

fn check_guards(table: &Table, command: &MutateCommand) -> Result<(), DebugFailure> {
    if command
        .expected_debug_revision
        .is_some_and(|expected| expected != table.debug.revision)
    {
        return Err(DebugFailure::Aborted {
            reason: DebugAbortReason::DebugRevisionMismatch,
            actual_debug_revision: table.debug.revision,
            actual_table_seq: table.seq,
        });
    }
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
    Ok(())
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
    let projected = catch_unwind(AssertUnwindSafe(|| {
        let extras = table_view_extras(table);
        for seat in 0..game.player_count() {
            let _ = complete_visible(game, Some(PlayerId(seat as u8)), &extras);
        }
        let _ = complete_visible(game, None, &extras);
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
    };
    let _ = table.tx.send(Arc::new(PublishedUpdate::Snapshot(state)));
}

#[cfg(test)]
mod tests;
