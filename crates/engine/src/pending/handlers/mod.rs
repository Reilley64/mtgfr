//! Pending-choice handlers and dig-loop kickoff helpers.
//!
//! Split by family (mirrors `raise/`) so raise + answer locality can deepen together.
//! External seam remains [`super::answer`] / [`super::forced`] / [`super::raise`].
//! Answer / forced routing is the choice-discriminant table in [`super::dispatch`].

mod combat;
mod common;
mod dig;
mod edict;
mod fanout;
mod library;
mod optional;
mod targets;

use crate::*;

/// Whether each permanent (given by its type `masks`) can be assigned a *distinct* `slot` type it
/// possesses — a system of distinct representatives for Tragic Arrogance's "one of each type" keep
/// (an artifact creature has two type bits but fills only one slot). Small brute-force recursion;
/// the pool never keeps more than three permanents (three reachable slots).
pub(crate) fn assign_to_distinct_slots(masks: &[TypeSet], slots: &[TypeSet], used: u32) -> bool {
    let Some((first, rest)) = masks.split_first() else {
        return true;
    };
    slots.iter().enumerate().any(|(i, &slot)| {
        let bit = 1 << i;
        used & bit == 0
            && first.intersects(slot)
            && assign_to_distinct_slots(rest, slots, used | bit)
    })
}

/// The maximum number of `slots` that distinct permanents (given by their type `masks`) can
/// simultaneously fill — Tragic Arrogance's mandatory keep count for a player (you must spare one
/// of every type you can reach). Each slot may take at most one permanent and each permanent at
/// most one slot (an artifact creature covers artifact *or* creature, not both). Brute-force
/// recursion over the ≤3 reachable slots.
pub(crate) fn max_distinct_slots(masks: &[TypeSet], slots: &[TypeSet]) -> usize {
    let Some((slot, rest)) = slots.split_first() else {
        return 0;
    };
    // Skip this slot, or fill it with any not-yet-used permanent that has its type.
    let skip = max_distinct_slots(masks, rest);
    masks
        .iter()
        .enumerate()
        .filter(|(_, mask)| mask.intersects(*slot))
        .map(|(i, _)| {
            let remaining: Vec<TypeSet> = masks
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, &m)| m)
                .collect();
            1 + max_distinct_slots(&remaining, rest)
        })
        .max()
        .unwrap_or(0)
        .max(skip)
}

#[cfg(test)]
mod tests {
    use crate::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    fn source_creature(game: &mut Game) -> ObjectId {
        game.spawn_on_battlefield(
            P0,
            CardDef {
                name: "Source",
                id: "",
                default_print: "",
                cost: Cost::FREE,
                kind: CardKind::Creature {
                    power: 1,
                    toughness: 1,
                    also: TypeSet::NONE,
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
            },
        )
    }

    #[test]
    fn choose_order_rejects_a_non_permutation() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: vec![
                    Effect::Draw(DrawEffect::Cards {
                        count: Amount::Fixed(1),
                    }),
                    Effect::Draw(DrawEffect::Cards {
                        count: Amount::Fixed(2),
                    }),
                ],
            },
        );
        assert_eq!(
            game.choose_order(P0, vec![0, 0]),
            Err(Reject::IllegalChoice)
        );
        assert!(
            game.pending_choice.is_some(),
            "invalid answer restores pause"
        );
    }

    #[test]
    fn choose_order_rejects_the_wrong_player() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: vec![Effect::Draw(DrawEffect::Cards {
                    count: Amount::Fixed(1),
                })],
            },
        );
        assert_eq!(game.choose_order(P1, vec![0]), Err(Reject::IllegalChoice));
        assert!(
            game.pending_choice.is_some(),
            "wrong player restores the pause"
        );
    }

    #[test]
    fn choose_order_accepts_a_valid_permutation() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        let effects = [
            Effect::Draw(DrawEffect::Cards {
                count: Amount::Fixed(1),
            }),
            Effect::Draw(DrawEffect::Cards {
                count: Amount::Fixed(2),
            }),
        ];
        // `choose_order` re-splits the still-queued group, so the queue has to hold it (that is
        // what `place_pending_triggers` leaves behind when it raises the ordering choice).
        game.pending_trigger_groups.push(TriggerGroup {
            controller: P0,
            source,
            abilities: effects
                .iter()
                .map(|&effect| Ability {
                    timing: Timing::Triggered(Trigger::Upkeep),
                    effect,
                    optional: false,
                    min_level: 0,
                    cost: Cost::FREE,
                    condition: None,
                    once_each_turn: false,
                })
                .collect(),
            expanded: true,
        });
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: effects.to_vec(),
            },
        );
        assert!(game.choose_order(P0, vec![1, 0]).is_ok());
        assert!(game.pending_choice.is_none());
        // order [1, 0] pushes effect 1 then effect 0 — bottom-first stack view.
        assert_eq!(
            game.stack(),
            vec![
                StackEntry::Ability {
                    controller: P0,
                    source,
                    effect: Effect::Draw(DrawEffect::Cards {
                        count: Amount::Fixed(2)
                    }),
                    target: None,
                },
                StackEntry::Ability {
                    controller: P0,
                    source,
                    effect: Effect::Draw(DrawEffect::Cards {
                        count: Amount::Fixed(1)
                    }),
                    target: None,
                },
            ]
        );
    }
}
