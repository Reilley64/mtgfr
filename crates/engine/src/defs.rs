//! Interned card definitions (`CardId` → `Arc<CardDef>`).
//!
//! Wave A of the engine refactor program: zone objects and events store [`CardId`]
//! instead of embedding a fat [`CardDef`]. See
//! `docs/superpowers/specs/2026-07-25-engine-refactor-program-design.md`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::CardDef;

/// Stable handle into the process-global card-definition intern table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardId(pub u32);

#[derive(Default)]
struct InternTable {
    defs: Vec<Arc<CardDef>>,
    by_oracle_id: HashMap<&'static str, CardId>,
}

fn table() -> &'static Mutex<InternTable> {
    static TABLE: OnceLock<Mutex<InternTable>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(InternTable::default()))
}

/// Intern `def` and return its stable [`CardId`].
///
/// Real card defs reuse the same handle whenever their non-empty Scryfall oracle id matches an
/// existing entry. Test stubs with an empty `id` still get a fresh handle on every call.
pub fn intern_card_def(def: CardDef) -> CardId {
    let mut guard = table().lock().expect("card def intern table poisoned");
    if !def.id.is_empty()
        && let Some(&id) = guard.by_oracle_id.get(def.id)
    {
        return id;
    }
    let id = CardId(guard.defs.len() as u32);
    if !def.id.is_empty() {
        guard.by_oracle_id.insert(def.id, id);
    }
    guard.defs.push(Arc::new(def));
    id
}

/// Shared definition for `id`. Panics if `id` was never returned by [`intern_card_def`].
pub fn card_def(id: CardId) -> Arc<CardDef> {
    let guard = table().lock().expect("card def intern table poisoned");
    guard
        .defs
        .get(id.0 as usize)
        .cloned()
        .unwrap_or_else(|| panic!("unknown CardId({})", id.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    const P0: PlayerId = PlayerId(0);

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
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            approximates: None,
            oracle: None,
            sets: empty_slice(),
            subtypes: empty_slice(),
            otags: empty_slice(),
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
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn vanilla_creature(name: &'static str, oracle_id: &'static str) -> CardDef {
        CardDef {
            name,
            id: oracle_id,
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Creature {
                power: 2,
                toughness: 2,
                also: TypeSet::NONE,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            approximates: None,
            oracle: None,
            sets: empty_slice(),
            subtypes: empty_slice(),
            otags: empty_slice(),
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
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn spell(name: &'static str) -> CardDef {
        CardDef {
            kind: CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            ..vanilla_creature(name, "")
        }
    }

    fn intern_halves(halves: Vec<CardDef>) -> Arc<[CardId]> {
        halves.into_iter().map(intern_card_def).collect()
    }

    fn interned_len() -> usize {
        table()
            .lock()
            .expect("card def intern table poisoned")
            .defs
            .len()
    }

    #[test]
    fn intern_card_def_round_trips_name() {
        let id = intern_card_def(forest());
        assert_eq!(card_def(id).name, "Forest");
    }

    #[test]
    fn same_non_empty_oracle_id_reuses_card_id() {
        let first = intern_card_def(vanilla_creature("Oracle Front", "oracle-dedupe-test"));
        let second = intern_card_def(vanilla_creature("Oracle Front Copy", "oracle-dedupe-test"));
        assert_eq!(first, second);
    }

    #[test]
    fn empty_id_stubs_still_get_distinct_ids() {
        let a = intern_card_def(forest());
        let b = intern_card_def(forest());
        assert_ne!(a, b);
    }

    #[test]
    fn flipped_permanent_def_id_is_stable_across_reads() {
        let back = intern_card_def(vanilla_creature("Flipper Back", ""));
        let front = CardDef {
            name: "Flipper Front",
            back: Some(back),
            ..vanilla_creature("Flipper Front", "")
        };
        let mut game = Game::new();
        let permanent = game.spawn_on_battlefield(P0, front);
        game.apply(&Event::Flipped { object: permanent });

        let before = interned_len();
        let first = game.def_id_of(permanent);
        let after_first = interned_len();
        let second = game.def_id_of(permanent);
        let after_second = interned_len();

        assert_eq!(
            first, second,
            "the flipped back-face CardId should be stable"
        );
        assert_eq!(
            after_first, after_second,
            "reading the flipped back face twice must not intern again"
        );
        assert!(
            after_first >= before,
            "the first read may intern once, but the second read must be pure"
        );
    }

    #[test]
    fn adventure_restore_reuses_front_face_card_id() {
        let adventure = intern_card_def(spell("Adventure Half"));
        let front = CardDef {
            name: "Adventure Front",
            adventure: Some(adventure),
            ..vanilla_creature("Adventure Front", "")
        };
        let mut game = Game::new();
        let source = game.spawn_in_hand(P0, front);
        let front_id = game.def_id_of(source);
        let spell = game.next_object_id();

        game.apply(&Event::AdventureSpellCast {
            spell,
            source,
            controller: P0,
            target: None,
            x: 0,
        });
        let after_cast = interned_len();
        let exiled = game.next_object_id();
        game.apply(&Event::ExiledOnAdventure {
            card: exiled,
            from: spell,
            owner: P0,
        });

        let Object::Card(exiled_card) = &game.objects[exiled as usize] else {
            panic!("adventure resolution should restore a card in exile");
        };
        assert_eq!(exiled_card.def, front_id);
        assert_eq!(
            interned_len(),
            after_cast,
            "restoring the creature front face from adventure must not reintern it"
        );
    }

    #[test]
    fn split_half_restore_reuses_fused_card_id() {
        let before_spawn = interned_len();
        let front = CardDef {
            name: "Fused Split",
            kind: CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            halves: intern_halves(vec![spell("Left Half"), spell("Right Half")]),
            ..vanilla_creature("Fused Split", "")
        };
        let mut game = Game::new();
        let source = game.spawn_in_hand(P0, front);
        let after_spawn = interned_len();
        let fused_id = game.def_id_of(source);
        let spell = game.next_object_id();

        assert_eq!(
            after_spawn,
            before_spawn + 3,
            "interning a split card should also intern both castable halves"
        );

        game.apply(&Event::SplitHalfSpellCast {
            spell,
            source,
            half: 0,
            controller: P0,
            target: None,
            x: 0,
        });
        let after_cast = interned_len();
        let graveyard_card = game.next_object_id();
        game.apply(&Event::MovedToGraveyard {
            card: graveyard_card,
            from: spell,
        });

        let Object::Card(restored_card) = &game.objects[graveyard_card as usize] else {
            panic!("a split-half spell leaving the stack should restore a card");
        };
        assert_eq!(restored_card.def, fused_id);
        assert_eq!(
            interned_len(),
            after_cast,
            "restoring the fused split card off the stack must not reintern it"
        );
    }
}
