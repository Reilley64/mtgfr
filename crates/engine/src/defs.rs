//! Interned card definitions (`CardId` → `Arc<CardDef>`).
//!
//! Wave A of the engine refactor program: zone objects and events store [`CardId`]
//! instead of embedding a fat [`CardDef`]. See
//! `docs/superpowers/specs/2026-07-25-engine-refactor-program-design.md`.

use std::sync::{Arc, Mutex, OnceLock};

use crate::CardDef;

/// Stable handle into the process-global card-definition intern table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardId(pub u32);

fn table() -> &'static Mutex<Vec<Arc<CardDef>>> {
    static TABLE: OnceLock<Mutex<Vec<Arc<CardDef>>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Intern `def` and return a fresh [`CardId`]. Each call allocates a distinct id
/// (test stubs that build equivalent defs intentionally get distinct handles).
pub fn intern_card_def(def: CardDef) -> CardId {
    let mut guard = table().lock().expect("card def intern table poisoned");
    let id = CardId(guard.len() as u32);
    guard.push(Arc::new(def));
    id
}

/// Shared definition for `id`. Panics if `id` was never returned by [`intern_card_def`].
pub fn card_def(id: CardId) -> Arc<CardDef> {
    let guard = table().lock().expect("card def intern table poisoned");
    guard
        .get(id.0 as usize)
        .cloned()
        .unwrap_or_else(|| panic!("unknown CardId({})", id.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn forest() -> CardDef {
        CardDef {
            name: "Forest",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            legendary: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            halves: &[],
            suspend: None,
            vanishing: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: &[],
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    #[test]
    fn intern_card_def_round_trips_name() {
        let id = intern_card_def(forest());
        assert_eq!(card_def(id).name, "Forest");
    }

    #[test]
    fn distinct_interns_get_distinct_ids() {
        let a = intern_card_def(forest());
        let b = intern_card_def(forest());
        assert_ne!(a, b);
    }
}
