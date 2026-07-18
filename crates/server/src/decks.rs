//! Tables and their seed decks.
//!
//! The pre-game lobby (claiming seats, picking decks, readying up) lives entirely in the
//! SolidStart BFF's own store now (`mtgfr_web` Postgres, Drizzle) — see `docs/prds` for the split.
//! A `Table` here is born already seeded: the BFF calls `Tables.Seed` once, handing over
//! the host, the ordered seats (each with its resolved deck), and this module builds the running
//! `Game` right away. There is no more claim/ready/start dance on this side.

use engine::{CardDef, Game, PlayerId};
use schema::SeedSeat;
use tokio::sync::broadcast;

use crate::chrome::ChromeState;

pub use crate::session::{Broadcast, PublishedDelta};

/// One seated player: the user who owns the seat and their display name (public — every
/// viewer's [`schema::PlayerView::username`] shows it).
#[derive(Debug, Clone, Default)]
pub struct Seat {
    pub user_id: Option<i64>,
    pub username: Option<String>,
}

/// A table: up to four seats playing a live game. One `Table` per `table_id` in the registry,
/// born with its game already running (see [`Table::seeded`]).
pub struct Table {
    /// The four seats (a Commander table). Only the first `seats.len()`-many via `seed` are
    /// filled; a table always seeds with 2..=4 players.
    pub seats: [Seat; 4],
    /// The host user (whoever started the game on the BFF side) — kept for display/audit; no
    /// handler on this side gates on it anymore.
    pub host: Option<i64>,
    /// The live game.
    pub game: Option<Game>,
    /// The PRNG seed the game was seeded with (recorded so a replay reproduces the shuffle).
    /// Meaningful only once `game` is `Some`.
    pub seed: u64,
    /// Monotonic delta sequence number; the snapshot watermark for resume.
    pub seq: u64,
    /// Monotonic publish id for the stream fan-out (gRPC broadcast). Advances on every
    /// broadcast, including hold-only ticks that keep game `seq` unchanged (dwell must not kill
    /// the hold timer).
    pub broadcast_seq: u64,
    pub tx: broadcast::Sender<Broadcast>,
    /// Auto-pass / stack-hold / dwell policy — see [`crate::chrome::ChromeState`].
    pub chrome: ChromeState,
    /// Per-seat Card id → Printing UUID from the seat's deck (art preference for ObjectView).
    pub prints: [std::collections::HashMap<String, String>; 4],
}

impl Table {
    /// An empty table shell (no seats, no game) — a test-support builder; production tables are
    /// always born via [`Table::seeded`].
    #[cfg(test)]
    pub fn empty() -> Table {
        Self::shell()
    }

    fn shell() -> Table {
        let (tx, _rx) = broadcast::channel(256);
        Table {
            seats: Default::default(),
            host: None,
            game: None,
            seed: 0,
            seq: 0,
            broadcast_seq: 0,
            tx,
            chrome: ChromeState::default(),
            prints: Default::default(),
        }
    }

    /// Build a table with its seats filled from an already-resolved lobby, ready for
    /// [`Table::game`] to be set by [`seed_game`]. `seats` is ordered by seat index (2..=4).
    pub fn seeded(host_user_id: i64, seats: &[SeedSeat]) -> Table {
        let mut table = Self::shell();
        table.host = Some(host_user_id);
        for (i, seat) in seats.iter().enumerate() {
            table.seats[i] = Seat {
                user_id: Some(seat.user_id),
                username: Some(seat.username.clone()),
            };
        }
        table
    }

    /// Milliseconds until the stack-hold would resolve, or `0` if no hold is active.
    /// Deadline math lives in the session module (shared with the hold poll loop).
    pub fn stack_hold_remaining_ms(&self) -> u32 {
        crate::session::stack_hold_remaining_ms(self.chrome.stack_hold(), self.chrome.any_dwell())
    }

    /// Fan out the current hold remaining without bumping game `seq` (dwell / countdown sync).
    /// Private to chrome — only [`crate::session::TableSession`] calls this.
    pub(crate) fn publish_hold_tick(&mut self) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        self.broadcast_seq += 1;
        let _ = self.tx.send(std::sync::Arc::new(PublishedDelta {
            seq: self.seq,
            broadcast_seq: self.broadcast_seq,
            events: vec![],
            game: game.clone(),
            auto_actions: vec![],
            yields: *self.chrome.yields(),
            turn_yields: *self.chrome.turn_yields(),
            stack_hold_remaining_ms: self.stack_hold_remaining_ms(),
        }));
    }

    /// The seat index a user holds, if any.
    pub fn seat_of(&self, user_id: i64) -> Option<u8> {
        self.seats
            .iter()
            .position(|s| s.user_id == Some(user_id))
            .map(|i| i as u8)
    }
}

/// Opening hand size (no mulligan — Phase 3).
const OPENING_HAND: u32 = 7;

/// A seat's resolved deck: the commander, the 99 as `(card, copies)`, and Card-id→Printing
/// for art (including the commander).
#[derive(Debug, Clone)]
pub struct SeatDeck {
    pub commander: CardDef,
    pub cards: Vec<(CardDef, usize)>,
    /// Card id → Printing UUID chosen for this seat's deck.
    pub prints: std::collections::HashMap<String, String>,
}

fn expand(list: &[(CardDef, usize)]) -> Vec<CardDef> {
    list.iter()
        .flat_map(|&(card, n)| std::iter::repeat_n(card, n))
        .collect()
}

/// Build a game for the given seated players (`(seat, deck)`, seats contiguous from 0):
/// designate each commander, seed and shuffle each library, and draw opening hands.
pub fn seed_game(seats: &[(PlayerId, SeatDeck)], seed: u64) -> Game {
    let mut game = Game::with_players(seats.len() as u8, seed);
    for (player, deck) in seats {
        game.designate_commander(*player, deck.commander);
        game.stack_library(*player, &expand(&deck.cards));
        game.shuffle(*player);
    }
    for _ in 0..OPENING_HAND {
        for (player, _) in seats {
            game.draw_card(*player);
        }
    }
    // The board exists now, so run turn 1's beginning steps (untap/upkeep/draw) — the constructor
    // parked at Main1 with them un-run. Auto-pass then carries an empty upkeep into Main1, so the
    // game is handed back at the starting player's first meaningful window, exactly as before.
    game.begin_first_turn();
    // No delta broadcasts yet at start (the table has no subscribers until the stream connects),
    // so there is nothing to fold the forced-choice labels into — a fresh `stream` connect just
    // renders the post-advance state directly. Discard them.
    // (The stack is empty at start, so this can never pause for a stack hold.)
    crate::session::advance_seeded_game(&mut game);
    game
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> CardDef {
        cards::get_by_name(name).expect("card in pool")
    }

    #[test]
    fn a_seeded_game_deals_seven_to_each_player_and_leaves_the_rest() {
        // A minimal legal-shaped deck: a commander plus 99 basics.
        let deck = || {
            let commander = card("Tajic, Legion's Edge");
            let plains = card("Plains");
            let mut prints = std::collections::HashMap::new();
            prints.insert(
                commander.id.to_string(),
                commander.default_print.to_string(),
            );
            prints.insert(plains.id.to_string(), plains.default_print.to_string());
            SeatDeck {
                commander,
                cards: vec![(plains, 99)],
                prints,
            }
        };
        let seats = [(PlayerId(0), deck()), (PlayerId(1), deck())];
        let game = seed_game(&seats, 0);
        assert_eq!(game.player_count(), 2);
        for (player, _) in &seats {
            let snap =
                schema::complete_visible(&game, Some(*player), &schema::ViewExtras::default());
            assert_eq!(snap.players[player.0 as usize].hand_count, OPENING_HAND);
            assert_eq!(
                snap.players[player.0 as usize].library_count,
                99 - OPENING_HAND,
            );
        }
    }
}

#[cfg(test)]
mod soc_deck_tests {
    //! Acceptance tests for the precon decks (fixtures in `fixtures/decks/`): the five `soc`
    //! decks plus one per fidelity-grind deck.

    use super::{SeatDeck, seed_game};
    use engine::{Game, Intent, PendingChoice, PlayerId};
    use schema::DeckCardEntry;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct DeckFixture {
        commander: String,
        commander_print: String,
        cards: Vec<DeckCardEntry>,
    }

    const FIXTURES: [&str; 7] = [
        "silverquill_influence",
        "prismari_artistry",
        "witherbloom_pestilence",
        "lorehold_spirit",
        "quandrix_unlimited",
        "enchantress_rubinia",
        "deathdancer_xira",
    ];

    fn load(fixture: &str) -> DeckFixture {
        let path = format!(
            "{}/fixtures/decks/{fixture}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        serde_json::from_str(&text).unwrap()
    }

    fn fixture_seat_deck(fixture: &str) -> SeatDeck {
        let deck = load(fixture);
        let commander = cards::get(&deck.commander)
            .unwrap_or_else(|| panic!("{:?} not in pool", deck.commander));
        let mut prints = std::collections::HashMap::new();
        prints.insert(commander.id.to_string(), deck.commander_print.clone());
        let cards = deck
            .cards
            .iter()
            .map(|c| {
                let def = cards::get(&c.id).unwrap_or_else(|| panic!("{:?} not in pool", c.id));
                prints.insert(def.id.to_string(), c.print.clone());
                (def, c.count as usize)
            })
            .collect();
        SeatDeck {
            commander,
            cards,
            prints,
        }
    }

    fn assert_legal(fixture: &str) {
        let deck = load(fixture);
        if let Err(problems) =
            crate::legality::validate(&deck.commander, &deck.commander_print, &deck.cards)
        {
            panic!(
                "{fixture} ({} + {} others) is not a legal Commander deck:\n  {}",
                deck.commander,
                deck.cards.iter().map(|c| c.count).sum::<u32>(),
                problems.join("\n  ")
            );
        }
    }

    #[test]
    fn silverquill_influence_is_a_legal_commander_deck() {
        assert_legal("silverquill_influence");
    }

    #[test]
    fn prismari_artistry_is_a_legal_commander_deck() {
        assert_legal("prismari_artistry");
    }

    #[test]
    fn witherbloom_pestilence_is_a_legal_commander_deck() {
        assert_legal("witherbloom_pestilence");
    }

    #[test]
    fn lorehold_spirit_is_a_legal_commander_deck() {
        assert_legal("lorehold_spirit");
    }

    #[test]
    fn quandrix_unlimited_is_a_legal_commander_deck() {
        assert_legal("quandrix_unlimited");
    }

    #[test]
    fn enchantress_rubinia_is_a_legal_commander_deck() {
        assert_legal("enchantress_rubinia");
    }

    #[test]
    fn deathdancer_xira_is_a_legal_commander_deck() {
        assert_legal("deathdancer_xira");
    }

    fn seed_four(first: &str) -> Game {
        let others: Vec<&str> = FIXTURES.iter().copied().filter(|f| *f != first).collect();
        let seats = [
            (PlayerId(0), fixture_seat_deck(first)),
            (PlayerId(1), fixture_seat_deck(others[0])),
            (PlayerId(2), fixture_seat_deck(others[1])),
            (PlayerId(3), fixture_seat_deck(others[2])),
        ];
        seed_game(&seats, 0x50c_2026)
    }

    fn advance_whole_turns(game: &mut Game, turns: usize) {
        let mut seen = 0;
        let mut prev = game.active_player();
        let mut guard = 0;
        while seen < turns {
            if game.winner().is_some() {
                return;
            }
            if let Some(PendingChoice::DiscardToHandSize {
                player,
                hand,
                count,
            }) = game.pending_choice()
            {
                game.submit(Intent::Discard {
                    player,
                    cards: hand[..count].to_vec(),
                })
                .unwrap();
                guard += 1;
                assert!(
                    guard < 200_000,
                    "did not complete {turns} turns within a sane bound"
                );
                continue;
            }
            // Decline any optional trigger (e.g. Pawn of Ulamog's "you may create a token" off
            // its own or another nontoken creature's death, #81) — this smoke test only cares
            // that the game keeps advancing, not that every "may" is taken.
            if let Some(PendingChoice::MayYesNo { player, .. }) = game.pending_choice() {
                game.submit(Intent::AnswerMay { player, yes: false })
                    .unwrap();
                guard += 1;
                assert!(
                    guard < 200_000,
                    "did not complete {turns} turns within a sane bound"
                );
                continue;
            }
            let p = game.priority_holder();
            game.submit(Intent::PassPriority { player: p }).unwrap();
            if game.active_player() != prev {
                prev = game.active_player();
                seen += 1;
            }
            guard += 1;
            assert!(
                guard < 200_000,
                "did not complete {turns} turns within a sane bound"
            );
        }
    }

    #[test]
    fn the_five_decks_seed_a_running_four_player_game() {
        for first in FIXTURES {
            let mut game = seed_four(first);
            assert_eq!(game.player_count(), 4);
            advance_whole_turns(&mut game, 8);
        }
    }
}
