//! Seed helpers: turn resolved seat decks into a running [`Game`].
//!
//! Live [`crate::Table`] values live in [`crate::table`]. The pre-game lobby (claiming seats,
//! picking decks, readying up) lives entirely in the SolidStart BFF's own store
//! (`mtgfr_web` Postgres, Drizzle) — see lobby/live-game + accounts/decks specs for the split. A table is born already
//! seeded: the BFF calls `Tables.Seed` once, handing over the host and ordered seats (each
//! with its resolved deck); [`seed_game`] builds the running game.

use engine::{CardDef, Game, Intent, PlayerId};

/// Opening hand size before pre-game mulligans.
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
/// designate each commander, seed and shuffle each library, draw opening hands, and wait for
/// each player to keep or mulligan.
pub fn seed_game(seats: &[(PlayerId, SeatDeck)], master_seed: [u8; 32]) -> Game {
    let mut game = Game::with_master_seed(seats.len() as u8, master_seed);
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
    game.begin_mulligans();
    game
}

/// Test helper: keep every opening hand, then advance to the first playable priority window.
pub fn keep_all_hands(game: &mut Game) {
    while game.mulliganing() {
        for p in 0..game.player_count() {
            let player = PlayerId(p as u8);
            if game.hand_kept(player) {
                continue;
            }
            game.submit(Intent::KeepHand { player })
                .expect("KeepHand accepted during seeded mulligans");
        }
    }
    crate::session::advance_seeded_game(game);
}

#[cfg(test)]
pub(crate) fn master_from_u64(seed: u64) -> [u8; 32] {
    let mut master_seed = [0; 32];
    master_seed[..8].copy_from_slice(&seed.to_le_bytes());
    master_seed
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
        let mut game = seed_game(&seats, master_from_u64(0));
        assert!(game.mulliganing());
        assert_eq!(game.player_count(), 2);
        for (player, _) in &seats {
            assert!(!game.hand_kept(*player));
            let snap =
                schema::complete_visible(&game, Some(*player), &schema::ViewExtras::default());
            assert_eq!(snap.players[player.0 as usize].hand_count, OPENING_HAND);
            assert_eq!(
                snap.players[player.0 as usize].library_count,
                99 - OPENING_HAND,
            );
        }
        keep_all_hands(&mut game);
        assert!(!game.mulliganing());
    }

    #[test]
    fn seed_game_does_not_start_turns_until_keeps() {
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
        let game = seed_game(&seats, master_from_u64(0));

        assert!(game.mulliganing());
        assert_eq!(game.current_step(), engine::Step::Main1);
        assert_eq!(game.active_player(), PlayerId(0));
    }
}

#[cfg(test)]
mod soc_deck_tests {
    //! Acceptance tests for the precon decks (fixtures in `fixtures/decks/`): the five `soc`
    //! decks plus one per fidelity-grind deck.

    use super::{SeatDeck, keep_all_hands, master_from_u64, seed_game};
    use engine::{Game, Intent, PendingChoice, PlayerId};
    use schema::DeckCardEntry;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct DeckFixture {
        commander: String,
        commander_print: String,
        cards: Vec<DeckCardEntry>,
    }

    const FIXTURES: [&str; 9] = [
        "silverquill_influence",
        "prismari_artistry",
        "witherbloom_pestilence",
        "lorehold_spirit",
        "quandrix_unlimited",
        "enchantress_rubinia",
        "deathdancer_xira",
        "political_puppets",
        "mirror_mastery",
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

    #[test]
    fn political_puppets_is_a_legal_commander_deck() {
        assert_legal("political_puppets");
    }

    #[test]
    fn mirror_mastery_is_a_legal_commander_deck() {
        assert_legal("mirror_mastery");
    }

    fn seed_four(first: &str) -> Game {
        let others: Vec<&str> = FIXTURES.iter().copied().filter(|f| *f != first).collect();
        let seats = [
            (PlayerId(0), fixture_seat_deck(first)),
            (PlayerId(1), fixture_seat_deck(others[0])),
            (PlayerId(2), fixture_seat_deck(others[1])),
            (PlayerId(3), fixture_seat_deck(others[2])),
        ];
        let mut game = seed_game(&seats, master_from_u64(0x50c_2026));
        keep_all_hands(&mut game);
        game
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
