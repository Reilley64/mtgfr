//! The pure core of the delta stream (ADR 0005/0006): snapshot-then-delta framing, per-viewer
//! redaction, and the seq-dedup boundary that prevents double delivery across the
//! subscribe/snapshot gap. Pulled out of the `stream` handler in `lib.rs` so this logic has a
//! test surface with no broadcast channel, keepalive timer, or `Body` involved — the handler
//! shell keeps the genuinely async parts and just pumps this.

use engine::{Event, Game, PlayerId};
use schema::{
    DeltaEnvelope, StreamFrame, ViewExtras, VisibleEvent, complete_visible, redact,
    spectator_redact,
};

use crate::decks::Seat;

/// Map Table-owned policy into the schema DTO that finishes a [`schema::VisibleState`].
#[allow(clippy::too_many_arguments)]
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
/// Thin wrapper: redact events, build [`ViewExtras`] from Table facts, call
/// [`complete_visible`]. Redaction stays separate from completeness (ADR 0006).
#[allow(clippy::too_many_arguments)]
pub fn frame_for(
    viewer: Option<PlayerId>,
    seq: u64,
    events: &[Event],
    game: &Game,
    auto_actions: Vec<String>,
    yields: &[bool; 4],
    turn_yields: &[bool; 4],
    seats: &[Seat; 4],
    stack_hold_remaining_ms: u32,
    prints: &[std::collections::HashMap<String, String>; 4],
) -> StreamFrame {
    let visible: Vec<VisibleEvent> = events
        .iter()
        .map(|e| match viewer {
            Some(v) => redact(e, v),
            None => spectator_redact(e),
        })
        .collect();
    let extras = view_extras(yields, turn_yields, seats, stack_hold_remaining_ms, prints);
    let state = complete_visible(game, viewer, &extras);
    StreamFrame::Delta(DeltaEnvelope {
        seq,
        events: visible,
        state,
        auto_actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn frame_for_a_seated_viewer_reveals_their_own_draw() {
        let game = Game::new();
        let frame = frame_for(
            Some(PlayerId(0)),
            5,
            &[alice_draws_a_shock()],
            &game,
            vec![],
            &[false; 4],
            &[false; 4],
            &std::array::from_fn(|_| Seat::default()),
            0,
            &Default::default(),
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
            &[false; 4],
            &[false; 4],
            &std::array::from_fn(|_| Seat::default()),
            0,
            &Default::default(),
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

        let StreamFrame::Delta(DeltaEnvelope { state, .. }) = frame_for(
            Some(PlayerId(0)),
            1,
            &[],
            &game,
            vec![],
            &yields,
            &turn_yields,
            &seats,
            900,
            &Default::default(),
        ) else {
            panic!("expected a delta frame");
        };

        assert!(state.yielded);
        assert!(!state.turn_yielded, "viewer P0 is not turn-yielded");
        assert_eq!(state.stack_hold_remaining_ms, 900);
        assert_eq!(state.players[0].username, "alice");
        assert_eq!(state.players[1].username, "bob");

        let StreamFrame::Delta(DeltaEnvelope { state: p1, .. }) = frame_for(
            Some(PlayerId(1)),
            1,
            &[],
            &game,
            vec![],
            &yields,
            &turn_yields,
            &seats,
            900,
            &Default::default(),
        ) else {
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
