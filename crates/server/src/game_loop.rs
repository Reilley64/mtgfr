//! HTTP handlers for intent submission, yield, and helpless dwell. Seat validation and ack
//! shaping live here; chrome policy is the three [`crate::session::TableSession`] verbs.
//! Table id is always a path param (`/tables/{table}/…`) so the BFF can route without body peek.

use axum::{
    Json,
    extract::{Path, State},
};
use engine::PlayerId;
use schema::{IntentEnvelope, to_intent_for_seat};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::decks::Table;
use crate::session::{ApplyResult, DwellResult, TableSession, settle_after_apply};
use crate::{AppState, auth::AuthUser, lock};

/// The response to a submitted intent. Deltas arrive on the stream, not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

/// Submit a player's intent: validate against the engine, and on success bump the delta
/// sequence and broadcast the resulting events to every viewer's stream.
#[utoipa::path(
    post,
    path = "/tables/{table}/intent/v1",
    params(("table" = String, Path, description = "table id")),
    request_body = IntentEnvelope,
    responses((status = 200, description = "Intent accepted or rejected", body = Ack)),
)]
pub async fn submit_intent(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table_id): Path<String>,
    Json(env): Json<IntentEnvelope>,
) -> Json<Ack> {
    // Live games are in-memory only (no durable store): a restart loses running games.
    // Format the debug trace row under the lock; write the file after releasing it so disk
    // I/O can't stall other tables.
    let (ack, log_row) = {
        let mut reg = lock(&state.reg);
        let (table, seat) = match seated_table(&mut reg, &table_id, &user) {
            Ok(pair) => pair,
            Err(ack) => return Json(ack),
        };
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
        let seq = table.seq;
        let ack = Ack::from(result);
        settle_after_apply(&mut reg, &state, &table_id, disposition, seq);
        (ack, log_row)
    };
    crate::action_log::append(&table_id, &log_row);
    Json(ack)
}

/// Resolve the caller's seat at a started table, or the `Ack` rejection to return. The one
/// seat-validation gate shared by every route that acts on a seat (C1: the seat must belong
/// to the signed-in user), so the routes can't drift apart.
fn seated_table<'r>(
    reg: &'r mut crate::Registry,
    table_id: &str,
    user: &AuthUser,
) -> Result<(&'r mut Table, u8), Ack> {
    let Some(table) = reg.tables.get_mut(table_id) else {
        return Err(reject("UnknownTable"));
    };
    if table.game.is_none() {
        return Err(reject("GameNotStarted"));
    }
    let Some(seat) = table.seat_of(user.0.id) else {
        return Err(reject("NotSeated"));
    };
    Ok((table, seat))
}

/// Mark (or clear) a seat's "don't care" yield: while the stack is non-empty, that seat is
/// auto-passed as if it had no meaningful action, so the stack resolves without waiting on
/// them. Cleared automatically once the stack empties (it's a per-stack yield, not a standing
/// one). Enabling may unstick the game immediately — the yielder might be the very player
/// everyone is waiting on — so this drives auto-advance and broadcasts like any intent.
#[utoipa::path(
    post,
    path = "/tables/{table}/yield/v1",
    params(("table" = String, Path, description = "table id")),
    request_body = YieldRequest,
    responses((status = 200, description = "Yield recorded", body = Ack)),
)]
pub async fn set_yield(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table_id): Path<String>,
    Json(req): Json<YieldRequest>,
) -> Json<Ack> {
    let (ack, log_row) = {
        let mut reg = lock(&state.reg);
        let (table, seat) = match seated_table(&mut reg, &table_id, &user) {
            Ok(pair) => pair,
            Err(ack) => return Json(ack),
        };
        let (result, disposition) = TableSession::new(table).set_yield(PlayerId(seat), req.enabled);
        let label = if req.enabled { "yield" } else { "unyield" };
        let log_row = crate::action_log::format_labeled(
            table.seq,
            seat,
            label,
            &result,
            &result.events,
            table.game.as_ref(),
        );
        let seq = table.seq;
        let ack = Ack::from(result);
        settle_after_apply(&mut reg, &state, &table_id, disposition, seq);
        (ack, log_row)
    };
    crate::action_log::append(&table_id, &log_row);
    Json(ack)
}

/// A player's "don't care about this stack" toggle (see [`set_yield`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct YieldRequest {
    /// True to auto-pass this seat while the current stack resolves; false to opt back in.
    pub enabled: bool,
}

/// Mark (or clear) a seat's turn yield: auto-pass until that seat's next turn, or until they
/// take an intentional action (ADR 0029). Independent of stack yield.
#[utoipa::path(
    post,
    path = "/tables/{table}/turn-yield/v1",
    params(("table" = String, Path, description = "table id")),
    request_body = YieldRequest,
    responses((status = 200, description = "Turn yield recorded", body = Ack)),
)]
pub async fn set_turn_yield(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table_id): Path<String>,
    Json(req): Json<YieldRequest>,
) -> Json<Ack> {
    let (ack, log_row) = {
        let mut reg = lock(&state.reg);
        let (table, seat) = match seated_table(&mut reg, &table_id, &user) {
            Ok(pair) => pair,
            Err(ack) => return Json(ack),
        };
        let (result, disposition) =
            TableSession::new(table).set_turn_yield(PlayerId(seat), req.enabled);
        let label = if req.enabled {
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
        let seq = table.seq;
        let ack = Ack::from(result);
        settle_after_apply(&mut reg, &state, &table_id, disposition, seq);
        (ack, log_row)
    };
    crate::action_log::append(&table_id, &log_row);
    Json(ack)
}

/// Helpless-reader hover on the stack during a hold (see [`set_stack_dwell`]).
#[utoipa::path(
    post,
    path = "/tables/{table}/stack-dwell/v1",
    params(("table" = String, Path, description = "table id")),
    request_body = StackDwellRequest,
    responses((status = 200, description = "Dwell recorded", body = Ack)),
)]
pub async fn set_stack_dwell(
    State(state): State<AppState>,
    user: AuthUser,
    Path(table_id): Path<String>,
    Json(req): Json<StackDwellRequest>,
) -> Json<Ack> {
    let mut reg = lock(&state.reg);
    let (table, seat) = match seated_table(&mut reg, &table_id, &user) {
        Ok(pair) => pair,
        Err(ack) => return Json(ack),
    };
    // No settle: dwell never produces a Disposition / never arms a hold timer.
    Json(Ack::from(
        TableSession::new(table).set_dwell(PlayerId(seat), req.dwelling),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StackDwellRequest {
    pub dwelling: bool,
}

/// A rejected ack with a reason.
fn reject(reason: &str) -> Ack {
    Ack {
        accepted: false,
        reason: Some(reason.to_string()),
    }
}
