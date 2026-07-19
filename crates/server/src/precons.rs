//! The precon decks (the five Secrets of Strixhaven decks plus one per fidelity-grind deck),
//! offered to every player as read-only decks.
//!
//! These are *virtual* decks: static data baked into the server, not per-user DB rows. That keeps
//! them free — no signup seeding, no migration, no guards against a user editing their copy — at
//! the cost of one convention: they carry **fixed negative ids**, which never collide with the
//! positive autoincrement ids of DB-backed `Deck` rows. Everywhere a deck id is resolved (list,
//! get, join, game start) a negative id routes here instead of to Postgres, and edit/delete of a
//! negative id is refused. See [`is_precon`].
//!
//! The decklists are shared with the Phase 5.5 legality fixtures (`fixtures/decks/*.json`,
//! generated from `decklists/*.md`) — one source of truth, `include_str!`'d in at build time.

use std::sync::LazyLock;

use schema::{DeckCardEntry, DeckDetail, DeckSummary};

/// A precon's fixture (name + count list); the display name lives in [`SOURCES`] alongside.
#[derive(serde::Deserialize)]
struct Fixture {
    commander: String,
    commander_print: String,
    cards: Vec<DeckCardEntry>,
}

struct Source {
    id: i64,
    name: &'static str,
    json: &'static str,
}

/// The precons with their fixed ids. Ids are negative so they can never collide with a DB deck's
/// autoincrement id; each new precon takes the next id down.
static SOURCES: [Source; 8] = [
    Source {
        id: -1,
        name: "Silverquill Influence",
        json: include_str!("../fixtures/decks/silverquill_influence.json"),
    },
    Source {
        id: -2,
        name: "Prismari Artistry",
        json: include_str!("../fixtures/decks/prismari_artistry.json"),
    },
    Source {
        id: -3,
        name: "Witherbloom Pestilence",
        json: include_str!("../fixtures/decks/witherbloom_pestilence.json"),
    },
    Source {
        id: -4,
        name: "Lorehold Spirit",
        json: include_str!("../fixtures/decks/lorehold_spirit.json"),
    },
    Source {
        id: -5,
        name: "Quandrix Unlimited",
        json: include_str!("../fixtures/decks/quandrix_unlimited.json"),
    },
    Source {
        id: -6,
        name: "Enchantress Rubinia",
        json: include_str!("../fixtures/decks/enchantress_rubinia.json"),
    },
    Source {
        id: -7,
        name: "Deathdancer Xira",
        json: include_str!("../fixtures/decks/deathdancer_xira.json"),
    },
    Source {
        id: -8,
        name: "Political Puppets",
        json: include_str!("../fixtures/decks/political_puppets.json"),
    },
];

/// The precon decks, parsed once. A malformed fixture is a build-baked bug, so panic on it.
static PRECONS: LazyLock<Vec<DeckDetail>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .map(|s| {
            let f: Fixture =
                serde_json::from_str(s.json).unwrap_or_else(|e| panic!("precon {}: {e}", s.name));
            DeckDetail {
                id: s.id,
                name: s.name.to_string(),
                commander: f.commander,
                commander_print: f.commander_print,
                cards: f.cards,
            }
        })
        .collect()
});

/// Whether a deck id refers to a (read-only) precon rather than a DB-backed deck.
pub fn is_precon(id: i64) -> bool {
    id < 0
}

/// The precon with this id, if any.
pub fn get(id: i64) -> Option<&'static DeckDetail> {
    PRECONS.iter().find(|d| d.id == id)
}

/// The precons as list-view summaries, to prepend to a user's deck list.
pub fn summaries() -> Vec<DeckSummary> {
    PRECONS
        .iter()
        .map(|d| DeckSummary {
            id: d.id,
            name: d.name.clone(),
            commander: d.commander.clone(),
            commander_print: d.commander_print.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_precons_parse_with_negative_ids_and_a_commander() {
        assert_eq!(PRECONS.len(), SOURCES.len());
        for d in PRECONS.iter() {
            assert!(is_precon(d.id), "{} should have a precon id", d.name);
            assert!(!d.commander.is_empty(), "{} needs a commander", d.name);
            assert!(
                !d.commander_print.is_empty(),
                "{} needs a commander print",
                d.name
            );
            assert!(
                d.cards.iter().all(|c| !c.print.is_empty()),
                "{} every card needs a print",
                d.name
            );
            assert_eq!(get(d.id).map(|g| &g.name), Some(&d.name));
        }
    }
}
