//! The pure core of the delta stream (ADR 0005/0006): snapshot-then-delta framing, per-viewer
//! redaction, and the seq-dedup boundary that prevents double delivery across the
//! subscribe/snapshot gap. Pulled out of the `stream` handler in `lib.rs` so this logic has a
//! test surface with no broadcast channel, keepalive timer, or `Body` involved — the handler
//! shell keeps the genuinely async parts and just pumps this.

use axum::http::StatusCode;
use engine::{Event, Game, PlayerId};
use schema::{
    DeltaCompose, StreamFrame, ViewExtras, VisibleState, complete_visible, compose_delta,
};
use tokio::sync::broadcast;

use crate::AppState;
use crate::session::Broadcast;
use crate::table::Seat;

/// Map Table-owned policy into the schema DTO that finishes a [`schema::VisibleState`].
pub fn view_extras(
    yields: &[bool; 4],
    turn_yields: &[bool; 4],
    seats: &[Seat; 4],
    stack_hold_remaining_ms: u32,
    prints: &[std::collections::HashMap<String, String>; 4],
) -> ViewExtras {
    ViewExtras {
        yields: *yields,
        turn_yields: *turn_yields,
        stack_hold_remaining_ms,
        usernames: std::array::from_fn(|i| {
            seats
                .get(i)
                .and_then(|s| s.username.clone())
                .unwrap_or_default()
        }),
        prints: prints.clone(),
    }
}

/// A resolved subscription to one table's delta stream, ready for a transport (gRPC
/// server-streaming; historically SSE) to pump: the opening snapshot plus everything the caller
/// needs to keep building later delta frames. Built by [`subscribe`] under the registry lock; the
/// transport shell owns the actual async loop over `rx`.
pub struct TableSubscription {
    pub rx: broadcast::Receiver<Broadcast>,
    pub snapshot_seq: u64,
    pub snapshot: VisibleState,
    pub viewer: Option<PlayerId>,
    pub seats: [Seat; 4],
    pub prints: [std::collections::HashMap<String, String>; 4],
    /// The table's `broadcast_seq` at snapshot time — later messages at or below this are
    /// already reflected in the snapshot (see [`should_deliver`]).
    pub snapshot_broadcast_seq: u64,
}

/// Resolve `user_id`'s subscription to `table_id`'s delta stream: their own seat if they have
/// one, or the public spectator view otherwise (C1/6.3 — the viewer is resolved server-side,
/// never from the client). `NOT_FOUND` if the table or its game doesn't exist. Subscribes to the
/// broadcast channel *before* snapshotting, so nothing slips through the subscribe/snapshot gap
/// (deltas already reflected in the snapshot are dropped later by [`should_deliver`]).
pub fn subscribe(
    state: &AppState,
    table_id: &str,
    user_id: i64,
) -> Result<TableSubscription, StatusCode> {
    let mut reg = crate::lock(&state.reg);
    let Some(table) = reg.get_mut(table_id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    if table.game.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    // Clear the seed/quiet mark so a later drain sweep arms grace from disconnect, not seed.
    table.quiet_since = None;
    let viewer = table.seat_of(user_id).map(PlayerId);
    let extras = table_view_extras(table);
    let snapshot = complete_visible(
        table.game.as_ref().expect("game checked above"),
        viewer,
        &extras,
    );
    Ok(TableSubscription {
        rx: table.tx.subscribe(),
        snapshot_seq: table.seq,
        snapshot,
        viewer,
        seats: table.seats.clone(),
        prints: table.prints.clone(),
        snapshot_broadcast_seq: table.broadcast_seq,
    })
}

/// Table → [`ViewExtras`] for the opening snapshot (and for tests that build frames from a live
/// table). Hold remaining is computed from chrome; seats/prints come from the table shell.
pub fn table_view_extras(table: &crate::Table) -> ViewExtras {
    view_extras(
        table.chrome.yields(),
        table.chrome.turn_yields(),
        &table.seats,
        table.stack_hold_remaining_ms(),
        &table.prints,
    )
}

/// Whether a broadcast message at `broadcast_seq` should reach a stream whose opening
/// snapshot was already at `snapshot_broadcast_seq`. Anything already reflected in that
/// snapshot is dropped — this is what prevents double delivery across the
/// subscribe-before-snapshot gap (ADR 0005). Hold-only ticks advance `broadcast_seq` without
/// bumping game `seq`, so dwell updates still reach clients.
pub fn should_deliver(broadcast_seq: u64, snapshot_broadcast_seq: u64) -> bool {
    broadcast_seq > snapshot_broadcast_seq
}

/// Build the redacted delta frame for one viewer. `viewer` is `None` for a spectator (6.3) —
/// the redaction path never exposes a hand or library to them, exactly as for an opponent.
/// `auto_actions` are the human-readable labels of any forced choices `auto_advance` submitted
/// while folding this intent's fallout into the frame — same for every viewer (no redaction: a
/// label never names a private card).
///
/// Thin transport adapter: maps into [`schema::compose_delta`]. Redaction stays separate from
/// completeness inside schema (ADR 0006).
pub fn frame_for(
    viewer: Option<PlayerId>,
    seq: u64,
    events: &[Event],
    game: &Game,
    auto_actions: Vec<String>,
    extras: &ViewExtras,
) -> StreamFrame {
    compose_delta(DeltaCompose {
        game,
        viewer,
        seq,
        events,
        auto_actions,
        extras,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schema::{DeltaEnvelope, VisibleEvent};

    fn def(name: &str) -> engine::CardDef {
        cards::get_by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}"))
    }

    /// A `CardDrawn` event is the sharpest fixture for redaction: its `card`/`from` fields are
    /// visible only to the drawer (see `schema::redact_for`), so it proves `frame_for` actually
    /// dispatches through the viewer-vs-spectator redaction path rather than passing events
    /// through untouched.
    fn alice_draws_a_shock() -> Event {
        Event::CardDrawn {
            player: PlayerId(0),
            object: 7,
            from: 3,
            card: def("Shock"),
        }
    }

    fn empty_extras() -> ViewExtras {
        ViewExtras::default()
    }

    #[test]
    fn frame_for_a_seated_viewer_reveals_their_own_draw() {
        let game = Game::new();
        let frame = frame_for(
            Some(PlayerId(0)),
            5,
            &[alice_draws_a_shock()],
            &game,
            vec![],
            &empty_extras(),
        );

        let StreamFrame::Delta(DeltaEnvelope { seq, events, .. }) = frame else {
            panic!("expected a delta frame");
        };
        assert_eq!(seq, 5);
        assert_eq!(
            events,
            vec![VisibleEvent::CardDrawn {
                player: 0,
                object: 7,
                from: Some(3),
                card: Some("Shock".to_string()),
            }],
            "the drawer sees their own draw's identity",
        );
    }

    #[test]
    fn frame_for_a_spectator_hides_the_drawn_cards_identity() {
        let game = Game::new();
        let frame = frame_for(
            None,
            5,
            &[alice_draws_a_shock()],
            &game,
            vec![],
            &empty_extras(),
        );

        let StreamFrame::Delta(DeltaEnvelope { events, .. }) = frame else {
            panic!("expected a delta frame");
        };
        assert_eq!(
            events,
            vec![VisibleEvent::CardDrawn {
                player: 0,
                object: 7,
                from: None,
                card: None,
            }],
            "a spectator sees that a draw happened, but never which card (6.3)",
        );
    }

    #[test]
    fn frame_for_stamps_table_extras_onto_the_visible_state() {
        let game = Game::new();
        let mut seats = std::array::from_fn(|_| Seat::default());
        seats[0].username = Some("alice".into());
        seats[1].username = Some("bob".into());
        let yields = [true, false, false, false];
        let turn_yields = [false, true, false, false];
        let extras = view_extras(&yields, &turn_yields, &seats, 900, &Default::default());

        let StreamFrame::Delta(DeltaEnvelope { state, .. }) =
            frame_for(Some(PlayerId(0)), 1, &[], &game, vec![], &extras)
        else {
            panic!("expected a delta frame");
        };

        assert!(state.yielded);
        assert!(!state.turn_yielded, "viewer P0 is not turn-yielded");
        assert_eq!(state.stack_hold_remaining_ms, 900);
        assert_eq!(state.players[0].username, "alice");
        assert_eq!(state.players[1].username, "bob");

        let StreamFrame::Delta(DeltaEnvelope { state: p1, .. }) =
            frame_for(Some(PlayerId(1)), 1, &[], &game, vec![], &extras)
        else {
            panic!("expected a delta frame");
        };
        assert!(!p1.yielded);
        assert!(p1.turn_yielded, "viewer P1's turn yield comes from extras");
    }

    #[test]
    fn a_message_already_reflected_in_the_opening_snapshot_is_skipped() {
        assert!(
            !should_deliver(10, 10),
            "broadcast_seq == snapshot: already captured in the snapshot",
        );
    }

    #[test]
    fn the_first_message_past_the_snapshot_is_delivered() {
        assert!(
            should_deliver(11, 10),
            "broadcast_seq == snapshot + 1: the first genuinely new message",
        );
    }
}
