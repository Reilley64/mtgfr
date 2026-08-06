//! The pure core of the delta stream (lobby-table-routing-and-live-game spec / wire-protocol-and-visibility spec): snapshot-then-delta framing, per-viewer
//! redaction, and the seq-dedup boundary that prevents double delivery across the
//! subscribe/snapshot gap. Pulled out of the `stream` handler in `lib.rs` so this logic has a
//! test surface with no broadcast channel, keepalive timer, or `Body` involved — the handler
//! shell keeps the genuinely async parts and just pumps this.

use axum::http::StatusCode;
use engine::{Event, Game, PlayerId};
use schema::{
    CardTextView, DeltaCompose, MessageRef, StreamFrame, ViewExtras, VisibleState, card_text,
    complete_visible, compose_delta,
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
        gravatar_hashes: std::array::from_fn(|i| {
            seats
                .get(i)
                .map(|s| s.gravatar_hash.clone())
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
    /// Printed words for the viewer's own deck, sent once with the snapshot.
    pub card_text: Vec<CardTextView>,
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
    // The viewer's own deck, plus whatever of anyone else's the snapshot already shows them — a
    // reconnect lands mid-game with opponents' permanents already on the battlefield, and those
    // faces have to draw their words without waiting for the next delta to mention them.
    let own = match viewer {
        Some(PlayerId(seat)) => table.prints[seat as usize].clone(),
        None => Default::default(),
    };
    let mut card_text = card_text_book(&own);
    card_text.extend(public_card_text(&snapshot, &own));
    Ok(TableSubscription {
        rx: table.tx.subscribe(),
        snapshot_seq: table.seq,
        snapshot,
        viewer,
        card_text,
        seats: table.seats.clone(),
        prints: table.prints.clone(),
        snapshot_broadcast_seq: table.broadcast_seq,
    })
}

/// The printed words of one seat's whole deck, joined by the printing that deck plays.
///
/// `prints` is that seat's Card id → Printing UUID map — the deck list itself, so the book covers
/// every card whose face that player can ever be shown, and no other seat's. Flavor is per
/// printing ([`cards::print_flavor`]), so the join is on the print id, not the card id. Sorted by
/// card id: the wire frame is compared byte-for-byte in tests, and a HashMap has no order.
pub fn card_text_book(prints: &std::collections::HashMap<String, String>) -> Vec<CardTextView> {
    let mut book: Vec<CardTextView> = prints
        .iter()
        .filter_map(|(card_id, print)| {
            let def = cards::get(card_id)?;
            Some(card_text(&def, cards::print_flavor(print)))
        })
        .collect();
    book.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    book
}

/// The printed words of every card `state` shows that isn't in `own` — an opponent's spell on the
/// stack, their permanent on the battlefield, a card exiled from another library you may cast.
///
/// This widens no visibility, and the reason is the whole safety argument: `state` has already
/// been through per-viewer redaction, so a `card_id` only survives on it when this viewer is
/// allowed to know which card that object is. A face-down permanent and a hidden pile card have
/// theirs blanked, so they are skipped here for free. Telling someone the printed rules of a card
/// whose *name* they are already being shown reveals nothing further — where the full decklist
/// book ([`card_text_book`]) genuinely would, which is why that one stays own-deck only.
///
/// `own` is the viewer's decklist (empty for a spectator); those cards already rode the snapshot,
/// so they are skipped rather than re-sent. Each object carries the printing its owner's deck
/// plays, so flavor joins on that print rather than the card's default.
pub fn public_card_text(
    state: &VisibleState,
    own: &std::collections::HashMap<String, String>,
) -> Vec<CardTextView> {
    let objects = state.objects.iter().map(|o| (&o.card_id, &o.print));
    let stack = state.stack.iter().map(|e| (&e.card_id, &e.print));
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut book: Vec<CardTextView> = objects
        .chain(stack)
        .filter(|(card_id, _)| !card_id.is_empty() && !own.contains_key(card_id.as_str()))
        .filter(|(card_id, _)| seen.insert(card_id.as_str()))
        .filter_map(|(card_id, print)| {
            let def = cards::get(card_id)?;
            Some(card_text(&def, cards::print_flavor(print)))
        })
        .collect();
    book.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    book
}

/// Keep only card words this connection has not already received.
///
/// [`frame_for`] is deliberately connection-agnostic and derives the complete public book from
/// each redacted state. The transport owns this small per-stream set so ordinary priority frames
/// do not resend every visible permanent's rules text.
pub fn retain_new_card_text(
    frame: &mut StreamFrame,
    known: &mut std::collections::HashSet<String>,
) {
    let StreamFrame::Delta(envelope) = frame else {
        return;
    };
    envelope
        .card_text
        .retain(|text| known.insert(text.card_id.clone()));
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
/// subscribe-before-snapshot gap (lobby-table-routing-and-live-game spec). Hold-only ticks advance `broadcast_seq` without
/// bumping game `seq`, so dwell updates still reach clients.
pub fn should_deliver(broadcast_seq: u64, snapshot_broadcast_seq: u64) -> bool {
    broadcast_seq > snapshot_broadcast_seq
}

/// Build the redacted delta frame for one viewer. `viewer` is `None` for a spectator (6.3) —
/// the redaction path never exposes a hand or library to them, exactly as for an opponent.
/// `auto_actions` are the stable labels of any forced choices `auto_advance` submitted
/// while folding this intent's fallout into the frame — same for every viewer (no redaction: a
/// label never names a private card).
///
/// Thin transport adapter: maps into [`schema::compose_delta`]. Redaction stays separate from
/// completeness inside schema (wire-protocol-and-visibility spec).
pub fn frame_for(
    viewer: Option<PlayerId>,
    seq: u64,
    events: &[Event],
    game: &Game,
    auto_actions: Vec<MessageRef>,
    extras: &ViewExtras,
) -> StreamFrame {
    let mut frame = compose_delta(DeltaCompose {
        game,
        viewer,
        seq,
        events,
        auto_actions,
        extras,
    });
    // `schema` composes the frame but cannot join printed words (no card registry there), so the
    // book is filled here from the state it just built.
    if let StreamFrame::Delta(env) = &mut frame {
        let own = match viewer {
            Some(PlayerId(seat)) => extras.prints[seat as usize].clone(),
            None => Default::default(),
        };
        env.card_text = public_card_text(&env.state, &own);
    }
    frame
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
            card: engine::intern_card_def(def("Shock")),
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
        seats[0].gravatar_hash = "abc".into();
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
        assert_eq!(state.players[0].gravatar_hash, "abc");
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

    #[test]
    fn the_card_text_book_joins_the_printing_the_deck_plays() {
        let bolt = def("Lightning Bolt");
        let prints = std::collections::HashMap::from([(
            bolt.id.to_string(),
            // The M10 printing, whose flavor the Alpha printing does not print.
            "435589bb-27c6-4a6d-9d63-394d5092b9d8".to_string(),
        )]);

        let book = card_text_book(&prints);

        assert_eq!(book.len(), 1);
        assert_eq!(book[0].card_id, bolt.id);
        assert_eq!(book[0].type_line, "Instant");
        assert!(book[0].oracle.contains("3 damage"));
        assert!(
            book[0].flavor.starts_with("The sparkmage shrieked"),
            "the deck's printing prints its own flavor: {:?}",
            book[0].flavor,
        );
    }

    #[test]
    fn the_card_text_book_is_only_that_seats_deck() {
        // The book is built from one seat's print map, so it never carries another seat's list —
        // and a spectator, who has no seat, gets nothing.
        let alice = std::collections::HashMap::from([(
            def("Lightning Bolt").id.to_string(),
            "435589bb-27c6-4a6d-9d63-394d5092b9d8".to_string(),
        )]);
        let shock = def("Shock").id.to_string();

        let book = card_text_book(&alice);

        assert!(book.iter().all(|text| text.card_id != shock));
        assert!(card_text_book(&Default::default()).is_empty());
    }

    /// A board with one of each seat's creatures on it, and the extras that name their printings.
    fn two_seats_on_the_battlefield() -> (Game, ViewExtras) {
        let mut game = Game::new();
        game.spawn_on_battlefield(PlayerId(0), def("Lightning Bolt"));
        game.spawn_on_battlefield(PlayerId(1), def("Grizzly Bears"));
        let mut prints: [std::collections::HashMap<String, String>; 4] = Default::default();
        prints[0].insert(
            def("Lightning Bolt").id.to_string(),
            "435589bb-27c6-4a6d-9d63-394d5092b9d8".to_string(),
        );
        let extras = view_extras(
            &[false; 4],
            &[false; 4],
            &std::array::from_fn(|_| Seat::default()),
            0,
            &prints,
        );
        (game, extras)
    }

    #[test]
    fn a_delta_carries_the_printed_words_of_an_opponents_card() {
        // The stack is where a player reads what is about to resolve, and three quarters of what
        // lands there is someone else's card. Their words are not in this viewer's own-deck book,
        // so the frame that shows them the object has to carry them.
        let (game, extras) = two_seats_on_the_battlefield();

        let StreamFrame::Delta(DeltaEnvelope { card_text, .. }) =
            frame_for(Some(PlayerId(0)), 1, &[], &game, vec![], &extras)
        else {
            panic!("expected a delta frame");
        };

        let bears = card_text
            .iter()
            .find(|text| text.card_id == def("Grizzly Bears").id)
            .expect("P1's creature is on P0's board, so its words ride the frame");
        assert_eq!(bears.type_line, "Creature — Bear");
    }

    #[test]
    fn a_delta_leaves_out_the_cards_the_snapshot_already_sent() {
        // The viewer's own deck rode the opening snapshot whole. Re-sending those words on every
        // delta would put the player's entire decklist on the wire once per priority pass.
        let (game, extras) = two_seats_on_the_battlefield();

        let StreamFrame::Delta(DeltaEnvelope { card_text, .. }) =
            frame_for(Some(PlayerId(0)), 1, &[], &game, vec![], &extras)
        else {
            panic!("expected a delta frame");
        };

        let bolt = def("Lightning Bolt").id.to_string();
        assert!(
            card_text.iter().all(|text| text.card_id != bolt),
            "P0's own card is already in their book",
        );
    }

    #[test]
    fn a_connection_sends_each_public_cards_words_only_once() {
        let (game, extras) = two_seats_on_the_battlefield();
        let mut known = std::collections::HashSet::new();
        let mut first = frame_for(Some(PlayerId(0)), 1, &[], &game, vec![], &extras);

        retain_new_card_text(&mut first, &mut known);
        let StreamFrame::Delta(DeltaEnvelope { card_text, .. }) = first else {
            panic!("expected a delta frame");
        };
        assert_eq!(
            card_text.len(),
            1,
            "the opponent's visible card arrives once"
        );

        let mut next = frame_for(Some(PlayerId(0)), 2, &[], &game, vec![], &extras);
        retain_new_card_text(&mut next, &mut known);
        let StreamFrame::Delta(DeltaEnvelope { card_text, .. }) = next else {
            panic!("expected a delta frame");
        };
        assert!(
            card_text.is_empty(),
            "a later priority frame does not resend it"
        );
    }

    #[test]
    fn a_spectator_reads_the_board_they_are_watching() {
        // A spectator has no deck, so their own-deck book is empty — everything they are shown has
        // to arrive this way or their whole view draws blank cards.
        let (game, extras) = two_seats_on_the_battlefield();

        let StreamFrame::Delta(DeltaEnvelope { card_text, .. }) =
            frame_for(None, 1, &[], &game, vec![], &extras)
        else {
            panic!("expected a delta frame");
        };

        let ids: Vec<&str> = card_text.iter().map(|text| text.card_id.as_str()).collect();
        assert!(ids.contains(&def("Lightning Bolt").id));
        assert!(ids.contains(&def("Grizzly Bears").id));
    }

    #[test]
    fn an_object_whose_card_id_was_redacted_away_contributes_no_words() {
        // The safety argument for this book is that it reads an already-redacted state: a
        // face-down permanent and a hidden pile card have their `card_id` blanked by the
        // projection, so they never reach the join. This pins that mechanically — blank the id the
        // way redaction does, and the words go with it.
        let (game, extras) = two_seats_on_the_battlefield();
        let mut state = complete_visible(&game, Some(PlayerId(0)), &extras);
        let bears = def("Grizzly Bears").id.to_string();
        assert!(
            !public_card_text(&state, &Default::default())
                .iter()
                .all(|text| text.card_id != bears),
            "sanity: the words are there while the card id is",
        );

        for obj in &mut state.objects {
            obj.card_id.clear();
        }

        assert!(
            public_card_text(&state, &Default::default()).is_empty(),
            "no card id, no words — a face-down permanent reveals nothing",
        );
    }
}
