//! Printing records: what is true of a printed object rather than of the card.
//!
//! A card is one oracle id; a printing is one Scryfall card UUID, and the two differ in the words
//! under the rules divider. Flavor text is printed, not oracle — a deck playing the Commander 2011
//! Terminate shows different italics than the Planar Chaos one. So the card TOML carries no flavor
//! and `data/prints/<slug>.toml` records every printing: its id, its set, and the flavor it prints.
//!
//! The server joins on the id it already has ([`crate::CardDef`] consumers see `ObjectView.print`),
//! so a board never asks a card API for a printing's words. [`CardDef::sets`] is derived from these
//! records at load rather than written a second time in the card TOML.
//!
//! Regenerate with `just cards-printings` (`tooling/gen-printings.mjs`, Scryfall bulk data).

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use serde::Deserialize;

/// One `data/prints/<slug>.toml` file: every printing of one card, oldest first.
#[derive(Debug, Deserialize)]
struct PrintingsToml {
    /// The Scryfall oracle id these printings belong to — [`crate::CardDef::id`].
    card: String,
    #[serde(default)]
    printings: Vec<PrintingToml>,
}

#[derive(Debug, Deserialize)]
struct PrintingToml {
    /// Scryfall card UUID: the id a deck stores and the wire carries as `ObjectView.print`.
    id: String,
    /// Scryfall set code.
    set: String,
    /// The italic words this printing prints, absent when it prints none.
    #[serde(default)]
    flavor: Option<String>,
}

pub(crate) struct Printings {
    /// Printing UUID → the flavor that printing prints. Only flavored printings are keyed.
    flavor: HashMap<String, &'static str>,
    /// Oracle id → set codes of its printings, alphabetical. Fills [`crate::CardDef::sets`].
    sets: HashMap<String, Arc<[&'static str]>>,
}

static PRINTINGS: OnceLock<Printings> = OnceLock::new();

pub(crate) fn install(printings: Printings) {
    PRINTINGS
        .set(printings)
        .unwrap_or_else(|_| panic!("printing registry installed twice"));
}

pub(crate) fn loaded() -> &'static Printings {
    PRINTINGS.get().expect("printings loaded during card load")
}

impl Printings {
    pub(crate) fn sets_of(&self, card_id: &str) -> Arc<[&'static str]> {
        self.sets
            .get(card_id)
            .cloned()
            .unwrap_or_else(|| Arc::from(Vec::new()))
    }
}

/// The flavor text of one printing, by Scryfall card UUID. `None` when the printing prints none
/// (most cards) or is not recorded.
pub fn print_flavor(print_id: &str) -> Option<&'static str> {
    loaded().flavor.get(print_id).copied()
}

pub(crate) fn load(data_dir: &Path) -> Printings {
    let dir = data_dir.join("prints");
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading printings dir {}: {e}", dir.display()));

    let mut flavor = HashMap::new();
    let mut sets = HashMap::new();
    for entry in entries {
        let path = entry.expect("printings dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let file: PrintingsToml =
            toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));

        // A card prints in a set once however many printings it has there, and coverage reads the
        // list in a stable order.
        let mut codes = BTreeSet::new();
        for printing in file.printings {
            codes.insert(intern(printing.set));
            let Some(words) = printing.flavor else {
                continue;
            };
            flavor.insert(printing.id, intern(words));
        }
        sets.insert(file.card, Arc::from(codes.into_iter().collect::<Vec<_>>()));
    }
    Printings { flavor, sets }
}

/// Load-once strings outlive the process; the pool is bounded and never reloaded.
fn intern(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
