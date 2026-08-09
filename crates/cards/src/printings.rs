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
    /// Named physical faces and the italic words printed on each face.
    #[serde(default)]
    faces: Vec<PrintingFaceToml>,
}

#[derive(Debug, Deserialize)]
struct PrintingFaceToml {
    name: String,
    #[serde(default)]
    flavor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PrintFaceKey {
    print_id: String,
    face_name: String,
}

pub(crate) struct Printings {
    /// Printing UUID → the flavor that printing prints. Only flavored printings are keyed.
    flavor: HashMap<String, &'static str>,
    /// Printing UUID + authoritative face name → that face's flavor.
    face_flavor: HashMap<PrintFaceKey, Option<&'static str>>,
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

    fn flavor_for_face(&self, print_id: &str, face_name: &str) -> Option<&'static str> {
        match self.face_flavor.get(&PrintFaceKey {
            print_id: print_id.to_owned(),
            face_name: face_name.to_owned(),
        }) {
            Some(flavor) => *flavor,
            None => self.flavor.get(print_id).copied(),
        }
    }
}

/// The flavor text of one printing, by Scryfall card UUID. `None` when the printing prints none
/// (most cards) or is not recorded.
pub fn print_flavor(print_id: &str) -> Option<&'static str> {
    loaded().flavor.get(print_id).copied()
}

/// Flavor printed on the named active face of one physical printing. Falls back to the
/// printing-level flavor for legacy single-face records.
pub fn print_face_flavor(print_id: &str, face_name: &str) -> Option<&'static str> {
    loaded().flavor_for_face(print_id, face_name)
}

pub(crate) fn load(data_dir: &Path) -> Printings {
    let dir = data_dir.join("prints");
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading printings dir {}: {e}", dir.display()));

    let mut files = Vec::new();
    for entry in entries {
        let path = entry.expect("printings dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let file: PrintingsToml =
            toml::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));

        files.push(file);
    }
    registry_from_files(files)
}

fn registry_from_files(files: impl IntoIterator<Item = PrintingsToml>) -> Printings {
    let mut flavor = HashMap::new();
    let mut face_flavor = HashMap::new();
    let mut sets = HashMap::new();

    for file in files {
        // A card prints in a set once however many printings it has there, and coverage reads the
        // list in a stable order.
        let mut codes = BTreeSet::new();
        for printing in file.printings {
            codes.insert(intern(printing.set));
            if let Some(words) = printing.flavor {
                flavor.insert(printing.id.clone(), intern(words));
            }
            for face in printing.faces {
                let words = face.flavor.map(intern);
                face_flavor.insert(
                    PrintFaceKey {
                        print_id: printing.id.clone(),
                        face_name: face.name,
                    },
                    words,
                );
            }
        }
        sets.insert(file.card, Arc::from(codes.into_iter().collect::<Vec<_>>()));
    }
    Printings {
        flavor,
        face_flavor,
        sets,
    }
}

/// Load-once strings outlive the process; the pool is bounded and never reloaded.
fn intern(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_distinct_flavor_for_each_named_face_of_one_printing() {
        let file: PrintingsToml = toml::from_str(
            r#"card = "card-id"
[[printings]]
id = "print-id"
set = "tst"

[[printings.faces]]
name = "Front Face"
flavor = "front words"

[[printings.faces]]
name = "Back Face"
flavor = "back words"
"#,
        )
        .expect("face-aware printing TOML parses");
        let registry = registry_from_files([file]);

        assert_eq!(
            registry.flavor_for_face("print-id", "Front Face"),
            Some("front words")
        );
        assert_eq!(
            registry.flavor_for_face("print-id", "Back Face"),
            Some("back words")
        );
    }

    #[test]
    fn known_unflavored_face_does_not_fall_back_to_front_flavor() {
        let file: PrintingsToml = toml::from_str(
            r#"card = "card-id"
[[printings]]
id = "print-id"
set = "tst"
flavor = "front words"

[[printings.faces]]
name = "Front Face"
flavor = "front words"

[[printings.faces]]
name = "Back Face"
"#,
        )
        .expect("an explicitly unflavored named face parses");
        let registry = registry_from_files([file]);

        assert_eq!(registry.flavor_for_face("print-id", "Back Face"), None);
        assert_eq!(
            registry.flavor_for_face("print-id", "Unknown Face"),
            Some("front words")
        );
    }

    #[test]
    fn checked_in_multiface_printing_records_name_their_faces() {
        let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        let registry = load(&data_dir);
        assert_eq!(
            registry.face_flavor.get(&PrintFaceKey {
                print_id: "676ba521-66e4-42cf-a315-70d03cb7334e".into(),
                face_name: "Pack a Punch".into(),
            }),
            Some(&None),
            "checked-in Kirol printing data must preserve its named unflavored back face"
        );
        assert!(
            registry
                .flavor_for_face("407d6723-bf58-403e-b2ac-ba52c51d356f", "Kyren Flamewright")
                .is_some_and(|words| words.starts_with("Inspired by tales")),
            "checked-in Invasion of Mercadia data must preserve its flavored back face"
        );
    }
}
