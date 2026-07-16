//! Full Commander deck legality, checked against the card pool.
//!
//! A legal deck is exactly one commander (a legendary creature, or a legendary planeswalker that
//! can be your commander) plus 99 other cards, singleton except basic lands, every card within
//! the commander's color identity, every name in the pool. `validate` returns *all* problems at
//! once so the builder can list them. The five `soc` precon decks each validate (see
//! `tests/deck_legality.rs`).

use engine::CardKind;
use schema::{DeckCardEntry, color_identity};

/// The number of cards a Commander deck holds besides the commander.
const DECK_SIZE: u32 = 99;

/// The basic land names, exempt from the singleton rule.
const BASICS: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

fn is_basic(name: &str) -> bool {
    BASICS.contains(&name)
}

/// Validate a deck for Commander legality. `Ok(())` = legal; `Err` lists every problem.
pub fn validate(commander: &str, cards: &[DeckCardEntry]) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();

    // Commander must resolve and be a legendary creature; its identity bounds the deck.
    let commander_identity = match cards::get(commander) {
        None => {
            problems.push(format!("commander {commander:?} is not in the card pool"));
            None
        }
        Some(def) => {
            // A commander is a legendary creature, or a planeswalker that says it can be your
            // commander (CR 903.3a). ponytail: no "can be your commander" flag on CardDef — the
            // pool's *only* legendary planeswalker (Quintorius, History Chaser) is exactly such a
            // commander, so accepting any legendary planeswalker is correct here. Add the flag if
            // the pool ever gains a legendary planeswalker that can't command.
            let can_command = def.legendary
                && matches!(
                    def.kind,
                    CardKind::Creature { .. } | CardKind::Planeswalker { .. }
                );
            if !can_command {
                problems.push(format!(
                    "commander {commander:?} is not a legendary creature or planeswalker"
                ));
            }
            Some(color_identity(&def))
        }
    };

    let mut total = 0u32;
    for entry in cards {
        total += entry.count;

        let Some(def) = cards::get(&entry.name) else {
            problems.push(format!("{:?} is not in the card pool", entry.name));
            continue;
        };
        if entry.count > 1 && !is_basic(&entry.name) {
            problems.push(format!(
                "{:?} appears {} times (singleton allows only 1)",
                entry.name, entry.count
            ));
        }
        if let Some(cmd_id) = commander_identity {
            // A card is off-identity if it has any color the commander lacks.
            if color_identity(&def) & !cmd_id != 0 {
                problems.push(format!(
                    "{:?} is outside the commander's color identity",
                    entry.name
                ));
            }
        }
    }

    if total != DECK_SIZE {
        let plural = if total == 1 { "card" } else { "cards" };
        problems.push(format!(
            "deck has {total} {plural} besides the commander (needs {DECK_SIZE})"
        ));
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, count: u32) -> DeckCardEntry {
        DeckCardEntry {
            name: name.to_string(),
            count,
        }
    }

    /// A legal Tajic (RW) deck: the pool's RW-legal nonbasics (singleton) padded to 99 with
    /// basic lands. `pad` shifts the basic-land count so tests can make it 98/99/100.
    fn tajic_deck(pad: i32) -> Vec<DeckCardEntry> {
        let nonbasics = [
            "Savannah Lions",
            "Goblin Guide",
            "Serra Angel",
            "Glorious Anthem",
            "Shock",
            "Brute Force",
        ];
        let mut cards: Vec<DeckCardEntry> = nonbasics.iter().map(|n| entry(n, 1)).collect();
        let basics = (DECK_SIZE as i32 - nonbasics.len() as i32 + pad) as u32;
        cards.push(entry("Plains", basics));
        cards
    }

    #[test]
    fn a_legal_mostly_basic_deck_passes() {
        assert_eq!(validate("Tajic, Legion's Edge", &tajic_deck(0)), Ok(()));
    }

    #[test]
    fn a_deck_of_the_wrong_size_is_rejected() {
        let err = validate("Tajic, Legion's Edge", &tajic_deck(-1)).unwrap_err();
        assert!(err.iter().any(|p| p.contains("98 cards")), "got {err:?}");
    }

    #[test]
    fn a_second_copy_of_a_nonbasic_breaks_singleton() {
        let mut deck = tajic_deck(-1); // make room so the count still totals 99
        deck[0].count = 2; // two Savannah Lions
        let err = validate("Tajic, Legion's Edge", &deck).unwrap_err();
        assert!(
            err.iter().any(|p| p.contains("Savannah Lions")),
            "got {err:?}"
        );
    }

    #[test]
    fn many_copies_of_a_basic_land_are_fine() {
        // tajic_deck already leans on ~93 Plains; singleton must not flag basics.
        let err = validate("Tajic, Legion's Edge", &tajic_deck(0));
        assert_eq!(err, Ok(()));
    }

    #[test]
    fn an_off_identity_card_is_rejected() {
        let mut deck = tajic_deck(-1);
        deck.push(entry("Llanowar Elves", 1)); // green — outside RW
        let err = validate("Tajic, Legion's Edge", &deck).unwrap_err();
        assert!(
            err.iter().any(|p| p.contains("Llanowar Elves")),
            "got {err:?}"
        );
    }

    #[test]
    fn a_non_legendary_commander_is_rejected() {
        let err = validate("Grizzly Bear", &tajic_deck(0)).unwrap_err();
        assert!(err.iter().any(|p| p.contains("legendary")), "got {err:?}");
    }

    #[test]
    fn a_legendary_planeswalker_commander_is_legal() {
        // Quintorius, History Chaser — Lorehold's RW planeswalker commander ("can be your
        // commander"). tajic_deck is an RW list, so it's within Quintorius's identity.
        assert_eq!(
            validate("Quintorius, History Chaser", &tajic_deck(0)),
            Ok(())
        );
    }

    #[test]
    fn an_unknown_card_is_rejected() {
        let mut deck = tajic_deck(-1);
        deck.push(entry("Black Lotus", 1));
        let err = validate("Tajic, Legion's Edge", &deck).unwrap_err();
        assert!(err.iter().any(|p| p.contains("Black Lotus")), "got {err:?}");
    }

    /// A B/G dual land flattened to a single green producer must still count its dropped black
    /// half toward color identity — otherwise a G/U deck could smuggle in a black land. Regression
    /// for the `color_identity` fix that reads each card's `identity_pips`.
    #[test]
    fn a_flattened_dual_lands_dropped_color_still_breaks_off_identity_legality() {
        let nonbasics = ["Llanowar Elves", "Dimir Informant", "Woodland Cemetery"];
        let mut deck: Vec<DeckCardEntry> = nonbasics.iter().map(|n| entry(n, 1)).collect();
        deck.push(entry("Forest", DECK_SIZE - nonbasics.len() as u32));

        let err = validate("Zimone, Infinite Analyst", &deck).unwrap_err();
        assert!(
            err.iter().any(|p| p.contains("Woodland Cemetery")),
            "got {err:?}"
        );
    }
}
