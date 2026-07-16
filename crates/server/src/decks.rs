//! Tables and their seed decks.
//!
//! A `Table` starts in a lobby (up-to-four claimable seats, each picking one of the joiner's
//! saved decks) and becomes a live `Game` when the host starts it. Seat identity is the
//! authenticated user; the deck a seat plays is resolved from the durable store at start.

use std::time::Duration;

use engine::{CardDef, Game, PlayerId};
use tokio::sync::broadcast;
use tokio::time::Instant;

pub use crate::session::{Broadcast, PublishedDelta};

/// How long a never-started lobby may sit idle (claimed or empty) before the sweeper evicts it,
/// and how recently a claimed-but-not-started lobby must have seen activity to still count as
/// "active" for drain purposes. See [`Table::touch`], `Registry::active_table_count`, and
/// `Registry::sweep_idle_lobbies`.
pub const IDLE_LOBBY_TTL: Duration = Duration::from_secs(30 * 60);

/// One lobby seat: which user claimed it, the saved deck they'll play (id + name for display),
/// and their ready flag.
#[derive(Debug, Clone, Default)]
pub struct Seat {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub deck_id: Option<i64>,
    pub deck_name: Option<String>,
    pub ready: bool,
}

/// A table: a lobby of up to four seats that becomes a running game once the host starts.
/// One `Table` per `table_id` in the registry.
pub struct Table {
    /// The four claimable seats (a Commander table). Filled front-to-back as players join.
    pub seats: [Seat; 4],
    /// The host user (the first player to join); only they may start the game.
    pub host: Option<i64>,
    /// The live game once started — `None` while still in the lobby.
    pub game: Option<Game>,
    /// The PRNG seed the game was seeded with (recorded so a replay reproduces the shuffle).
    /// Meaningful only once `game` is `Some`.
    pub seed: u64,
    /// Monotonic delta sequence number; the snapshot watermark for resume.
    pub seq: u64,
    /// Monotonic publish id for the SSE fan-out. Advances on every broadcast, including
    /// hold-only ticks that keep game `seq` unchanged (dwell must not kill the hold timer).
    pub broadcast_seq: u64,
    pub tx: broadcast::Sender<Broadcast>,
    /// Per-seat "don't care" yields: a yielded seat is auto-passed while the stack is
    /// non-empty. Mutated via [`crate::session::TableSession::set_yield`]; cleared whenever
    /// the stack empties.
    pub yields: [bool; 4],
    /// Per-seat turn yield (ADR 0029): auto-pass until that seat's turn / until they act.
    pub turn_yields: [bool; 4],
    /// Active stack-hold (uncontested resolve pause): seq + when the hold started
    /// (`tokio::time::Instant` so hold timers honor the test paused clock).
    pub stack_hold: Option<(u64, tokio::time::Instant)>,
    /// Per-seat helpless stack dwell (hover pause). Mutated via
    /// [`crate::session::TableSession::set_dwell`]; cleared when the hold ends.
    pub stack_dwell: [bool; 4],
    /// Last join/ready/start for idle-lobby TTL (ignored once `game` is set).
    pub last_activity: Instant,
    /// Per-seat Card id → Printing UUID from the seat's deck (art preference for ObjectView).
    pub prints: [std::collections::HashMap<String, String>; 4],
}

impl Table {
    /// A fresh, empty lobby table.
    pub fn new_lobby() -> Table {
        let (tx, _rx) = broadcast::channel(256);
        Table {
            seats: Default::default(),
            host: None,
            game: None,
            seed: 0,
            seq: 0,
            broadcast_seq: 0,
            tx,
            yields: [false; 4],
            turn_yields: [false; 4],
            stack_hold: None,
            stack_dwell: [false; 4],
            last_activity: Instant::now(),
            prints: Default::default(),
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Started game, or claimed lobby still inside [`IDLE_LOBBY_TTL`].
    pub fn is_active(&self) -> bool {
        self.game.is_some()
            || (self.claimed_count() >= 1 && self.last_activity.elapsed() < IDLE_LOBBY_TTL)
    }

    /// Never-started lobby idle past `ttl` — sweeper candidate.
    pub fn is_idle_lobby(&self, ttl: Duration) -> bool {
        self.game.is_none() && self.last_activity.elapsed() >= ttl
    }

    /// Milliseconds until the stack-hold would resolve, or `0` if no hold is active.
    /// Deadline math lives in the session module (shared with the hold poll loop).
    pub fn stack_hold_remaining_ms(&self) -> u32 {
        crate::session::stack_hold_remaining_ms(
            self.stack_hold,
            self.stack_dwell.iter().any(|&d| d),
        )
    }

    /// Fan out the current hold remaining without bumping game `seq` (dwell / countdown sync).
    /// Called from [`crate::session::TableSession::set_dwell`], not from HTTP handlers.
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
            yields: self.yields,
            turn_yields: self.turn_yields,
            stack_hold_remaining_ms: self.stack_hold_remaining_ms(),
        }));
    }

    /// The number of claimed seats (contiguous from seat 0 — players join the next open seat).
    pub fn claimed_count(&self) -> usize {
        self.seats.iter().filter(|s| s.user_id.is_some()).count()
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
    //! Acceptance tests for the five `soc` precon decks (fixtures in `fixtures/decks/`).

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

    const FIXTURES: [&str; 5] = [
        "silverquill_influence",
        "prismari_artistry",
        "witherbloom_pestilence",
        "lorehold_spirit",
        "quandrix_unlimited",
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
