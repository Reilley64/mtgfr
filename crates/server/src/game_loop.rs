//! Live-table chrome verbs for gRPC: submit, yield, turn-yield, dwell.
//!
//! One deep **TableOps** seam owns lock → seat check → [`TableSession`] → action log →
//! [`settle_after_apply`]. gRPC adapters call the `*_core` entry points only.

use engine::PlayerId;
use schema::{IntentEnvelope, to_intent_for_seat};
use serde::{Deserialize, Serialize};

use crate::session::{ApplyResult, Disposition, DwellResult, TableSession, settle_after_apply};
use crate::{AppState, Registry, Table, lock};

/// The response to a submitted intent. Deltas arrive on the stream, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub accepted: bool,
    /// Why the intent was rejected, if it was.
    pub reason: Option<String>,
}

impl From<ApplyResult> for Ack {
    fn from(result: ApplyResult) -> Self {
        Self {
            accepted: result.accepted,
            reason: result.reason,
        }
    }
}

impl From<DwellResult> for Ack {
    fn from(result: DwellResult) -> Self {
        Self {
            accepted: result.accepted,
            reason: result.reason,
        }
    }
}

/// Outcome of a drive verb under the registry lock (before unlock-tail settle + log append).
struct DriveOutcome {
    result: ApplyResult,
    disposition: Disposition,
    log_row: String,
}

/// Lock → seated table → `drive` → settle → unlock → append action log.
///
/// The sole place callers learn the unlock-tail ordering invariant (Disposition requires
/// [`settle_after_apply`]). Dwell does not use this path — it never produces a Disposition.
fn with_seated_drive(
    state: &AppState,
    user_id: i64,
    table_id: &str,
    drive: impl FnOnce(&mut Table, u8) -> DriveOutcome,
) -> Ack {
    let (ack, log_row) = {
        let mut reg = lock(&state.reg);
        let (table, seat) = match seated_table(&mut reg, table_id, user_id) {
            Ok(pair) => pair,
            Err(ack) => return ack,
        };
        let DriveOutcome {
            result,
            disposition,
            log_row,
        } = drive(table, seat);
        let seq = table.seq;
        let ack = Ack::from(result);
        settle_after_apply(&mut reg, state, table_id, disposition, seq);
        (ack, log_row)
    };
    crate::action_log::append(table_id, &log_row);
    ack
}

/// Submit a player's intent: validate against the engine, and on success bump the delta
/// sequence and broadcast the resulting events to every viewer's stream. Called by the gRPC
/// `Game.SubmitIntent` service (`grpc::game_svc`). Live games are in-memory only (no durable
/// store): a restart loses running games. Format the debug trace row under the lock; write the
/// file after releasing it so disk I/O can't stall other tables.
pub(crate) async fn submit_intent_core(
    state: &AppState,
    user_id: i64,
    table_id: &str,
    env: IntentEnvelope,
) -> Ack {
    let span = tracing::info_span!(
        "submit_intent_core",
        table_id = %table_id,
        user_id = user_id,
        accepted = tracing::field::Empty,
    );
    let _enter = span.enter();

    let ack = with_seated_drive(state, user_id, table_id, |table, seat| {
        let intent = to_intent_for_seat(env.intent.clone(), PlayerId(seat));
        let (result, disposition) = TableSession::new(table).submit(intent);
        let log_row = crate::action_log::format_row(
            table.seq,
            seat,
            &env.intent,
            &result,
            &result.events,
            table.game.as_ref(),
        );
        DriveOutcome {
            result,
            disposition,
            log_row,
        }
    });
    span.record("accepted", ack.accepted);
    ack
}

/// Resolve `user_id`'s seat at a started table, or the `Ack` rejection to return. The one
/// seat-validation gate shared by every route that acts on a seat (C1: the seat must belong
/// to the signed-in user), so HTTP and gRPC can't drift apart.
fn seated_table<'r>(
    reg: &'r mut Registry,
    table_id: &str,
    user_id: i64,
) -> Result<(&'r mut Table, u8), Ack> {
    let Some(table) = reg.get_mut(table_id) else {
        return Err(reject("UnknownTable"));
    };
    if table.game.is_none() {
        return Err(reject("GameNotStarted"));
    }
    let Some(seat) = table.seat_of(user_id) else {
        return Err(reject("NotSeated"));
    };
    Ok((table, seat))
}

/// Mark (or clear) a seat's "don't care" yield: while the stack is non-empty, that seat is
/// auto-passed as if it had no meaningful action, so the stack resolves without waiting on
/// them. Cleared automatically once the stack empties (it's a per-stack yield, not a standing
/// one). Enabling may unstick the game immediately — the yielder might be the very player
/// everyone is waiting on — so this drives auto-advance and broadcasts like any intent. Called
/// by the gRPC `Game.SetYield` service.
pub(crate) async fn set_yield_core(
    state: &AppState,
    user_id: i64,
    table_id: &str,
    enabled: bool,
) -> Ack {
    with_seated_drive(state, user_id, table_id, |table, seat| {
        let (result, disposition) = TableSession::new(table).set_yield(PlayerId(seat), enabled);
        let label = if enabled { "yield" } else { "unyield" };
        let log_row = crate::action_log::format_labeled(
            table.seq,
            seat,
            label,
            &result,
            &result.events,
            table.game.as_ref(),
        );
        DriveOutcome {
            result,
            disposition,
            log_row,
        }
    })
}

/// Mark (or clear) a seat's turn yield: auto-pass until that seat's next turn, or until they
/// take an intentional action (turn-priority-and-stack spec). While that seat is active, the same flag is Arena
/// End Turn (turn-priority-and-stack spec). Independent of stack yield. Called by the gRPC `Game.SetTurnYield`
/// service.
pub(crate) async fn set_turn_yield_core(
    state: &AppState,
    user_id: i64,
    table_id: &str,
    enabled: bool,
) -> Ack {
    with_seated_drive(state, user_id, table_id, |table, seat| {
        let (result, disposition) =
            TableSession::new(table).set_turn_yield(PlayerId(seat), enabled);
        let label = if enabled {
            "turn-yield"
        } else {
            "un-turn-yield"
        };
        let log_row = crate::action_log::format_labeled(
            table.seq,
            seat,
            label,
            &result,
            &result.events,
            table.game.as_ref(),
        );
        DriveOutcome {
            result,
            disposition,
            log_row,
        }
    })
}

/// Helpless-reader hover on the stack during a hold. No settle: dwell never produces a
/// `Disposition` / never arms a hold timer. Called by the gRPC `Game.SetStackDwell` service.
pub(crate) fn set_stack_dwell_core(
    state: &AppState,
    user_id: i64,
    table_id: &str,
    dwelling: bool,
) -> Ack {
    let mut reg = lock(&state.reg);
    let (table, seat) = match seated_table(&mut reg, table_id, user_id) {
        Ok(pair) => pair,
        Err(ack) => return ack,
    };
    Ack::from(TableSession::new(table).set_dwell(PlayerId(seat), dwelling))
}

/// A rejected ack with a reason.
fn reject(reason: &str) -> Ack {
    Ack {
        accepted: false,
        reason: Some(reason.to_string()),
    }
}
