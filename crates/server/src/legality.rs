//! Full Commander deck legality, checked against the card pool.
//!
//! A legal deck is exactly one commander (a legendary creature, or a legendary planeswalker that
//! can be your commander) plus 99 other cards, singleton except basic lands, every card within
//! the commander's color identity, every Card id in the pool. `validate` returns *all* problems at
//! once so the builder can list them. The five `soc` precon decks each validate (see
//! `tests/deck_legality.rs`).

use engine::CardKind;
use schema::{DeckCardEntry, color_identity};

/// The number of cards a Commander deck holds besides the commander.
const DECK_SIZE: u32 = 99;

fn is_print_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let lens = [8, 4, 4, 4, 12];
    parts
        .iter()
        .zip(lens)
        .all(|(part, len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

fn is_basic(def: &engine::CardDef) -> bool {
    matches!(def.kind, CardKind::Land { basic: true, .. })
}

/// Validate a deck for Commander legality. `Ok(())` = legal; `Err` lists every problem.
/// `commander` and each entry's `id` are Card ids (Scryfall oracle ids). `commander_print` and
/// each entry's `print` must be non-empty Printing UUIDs (art preference — accounts-decks-and-catalog spec).
pub fn validate(
    commander: &str,
    commander_print: &str,
    cards: &[DeckCardEntry],
) -> Result<(), Vec<String>> {
    let mut problems = Vec::new();

    if commander_print.is_empty() {
        problems.push("commander is missing a print".to_string());
    } else if !is_print_uuid(commander_print) {
        problems.push("commander print is not a valid printing id".to_string());
    }

    let commander_identity = match cards::get(commander) {
        None => {
            problems.push(format!("commander {commander:?} is not in the card pool"));
            None
        }
        Some(def) => {
            let can_command = def.legendary
                && matches!(
                    def.kind,
                    CardKind::Creature { .. } | CardKind::Planeswalker { .. }
                );
            if !can_command {
                problems.push(format!(
                    "commander {:?} is not a legendary creature or planeswalker",
                    def.name
                ));
            }
            Some(color_identity(&def))
        }
    };

    let mut total = 0u32;
    for entry in cards {
        total += entry.count;

        if entry.print.is_empty() {
            problems.push(format!("card {:?} is missing a print", entry.id));
        } else if !is_print_uuid(&entry.print) {
            problems.push(format!("card {:?} has an invalid print", entry.id));
        }

        let Some(def) = cards::get(&entry.id) else {
            problems.push(format!("{:?} is not in the card pool", entry.id));
            continue;
        };
        if entry.count > 1 && !is_basic(&def) {
            problems.push(format!(
                "{:?} appears {} times (singleton allows only 1)",
                def.name, entry.count
            ));
        }
        if let Some(cmd_id) = commander_identity
            && color_identity(&def) & !cmd_id != 0
        {
            problems.push(format!(
                "{:?} is outside the commander's color identity",
                def.name
            ));
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
        let def = cards::get_by_name(name).expect("pool card");
        DeckCardEntry {
            id: def.id.to_string(),
            count,
            print: def.default_print.to_string(),
        }
    }

    #[test]
    fn a_legal_deck_validates() {
        let plains = entry("Plains", 99);
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        assert!(validate(cmd.id, cmd.default_print, &[plains]).is_ok());
    }

    #[test]
    fn wrong_size_is_a_problem() {
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(cmd.id, cmd.default_print, &[entry("Plains", 98)]).unwrap_err();
        assert!(err.iter().any(|p| p.contains("98")));
    }

    #[test]
    fn singleton_blocks_duplicates_of_non_basics() {
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(
            cmd.id,
            cmd.default_print,
            &[entry("Sol Ring", 2), entry("Plains", 97)],
        )
        .unwrap_err();
        assert!(err.iter().any(|p| p.contains("Sol Ring")));
    }

    #[test]
    fn off_identity_is_a_problem() {
        // Tajic is RW; Deep Analysis is blue.
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(
            cmd.id,
            cmd.default_print,
            &[entry("Deep Analysis", 1), entry("Plains", 98)],
        )
        .unwrap_err();
        assert!(err.iter().any(|p| p.contains("Deep Analysis")));
    }

    #[test]
    fn unknown_commander_is_a_problem() {
        let err = validate(
            "not-a-real-oracle-id",
            "00000000-0000-0000-0000-000000000000",
            &[entry("Plains", 99)],
        )
        .unwrap_err();
        assert!(err.iter().any(|p| p.contains("not in the card pool")));
    }

    #[test]
    fn missing_print_is_a_problem() {
        let def = cards::get_by_name("Plains").unwrap();
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(
            cmd.id,
            cmd.default_print,
            &[DeckCardEntry {
                id: def.id.to_string(),
                count: 99,
                print: String::new(),
            }],
        )
        .unwrap_err();
        assert!(err.iter().any(|p| p.contains("missing a print")));
    }

    #[test]
    fn missing_commander_print_is_a_problem() {
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(cmd.id, "", &[entry("Plains", 99)]).unwrap_err();
        assert!(
            err.iter()
                .any(|p| p.contains("commander is missing a print"))
        );
    }

    #[test]
    fn invalid_commander_print_is_a_problem() {
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(cmd.id, "not-a-uuid", &[entry("Plains", 99)]).unwrap_err();
        assert!(err.iter().any(|p| p.contains("not a valid printing id")));
    }

    #[test]
    fn invalid_card_print_is_a_problem() {
        let def = cards::get_by_name("Plains").unwrap();
        let cmd = cards::get_by_name("Tajic, Legion's Edge").unwrap();
        let err = validate(
            cmd.id,
            cmd.default_print,
            &[DeckCardEntry {
                id: def.id.to_string(),
                count: 99,
                print: "bad-print".to_string(),
            }],
        )
        .unwrap_err();
        assert!(err.iter().any(|p| p.contains("invalid print")));
    }
}
