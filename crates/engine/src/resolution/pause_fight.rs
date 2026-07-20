//! Fight / move-counters pause family — resolution-time second-target peels.
//!
//! Pause peel behind [`Game::run`] (ADR 0002 deepen). Pause bookkeeping stays in
//! [`crate::pending`].

use crate::*;

impl Game {
    /// Pause on Fight (both shapes) / MoveCounters for the matching effect.
    pub(crate) fn run_fight_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        match effect {
            // Fight (CR 701.12): `target` is already the opponent's creature (chosen at cast);
            // pause on a ChooseTarget for the controller's own creature (mirrors
            // `place_targeted_ability`). No legal creature you control: the fight fizzles
            // (CR 601.2c — no damage, no pause) rather than picking an illegal target.
            Effect::Fight {
                ally_is_shared_target: false,
                ..
            } => {
                let legal = self.legal_targets_for(
                    TargetSpec::CreatureYouControl,
                    source,
                    controller,
                    [false; Color::COUNT],
                    x,
                );
                if legal.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::Fight {
                            enemy: target,
                            ally_is_shared_target: false,
                        },
                        legal,
                        count: TargetCount::default(),
                        x: 0,
                        activated: false,
                    },
                );
            }
            // Primal Might's mirror shape (CR 701.12): `target` is already the ally (the pumped
            // creature you control, chosen at cast by a preceding Sequence step); pause on an
            // *optional* ChooseTarget for the enemy ("fights up to one target creature you don't
            // control"). Guard-returns with no pause if the ally has since left the battlefield
            // or stopped being a creature (CR 608.2b — a fizzled shared target) or there's no
            // legal enemy — the pump still stands either way.
            Effect::Fight {
                ally_is_shared_target: true,
                ..
            } => {
                let ally = expect_object_target(target, "primal might's pumped ally");
                if !self.is_creature_on_battlefield(ally) {
                    return;
                }
                let legal = self.legal_targets_for(
                    TargetSpec::Permanent(PermanentFilter {
                        controller: FilterController::Opponent,
                        ..PermanentFilter::of(TypeSet::CREATURE)
                    }),
                    source,
                    controller,
                    [false; Color::COUNT],
                    x,
                );
                if legal.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::Fight {
                            enemy: Some(Target::Object(ally)),
                            ally_is_shared_target: false,
                        },
                        legal,
                        count: TargetCount {
                            min: 0,
                            max: 1,
                            ..TargetCount::default()
                        },
                        x: 0,
                        activated: false,
                    },
                );
            }
            // Move all counters of a kind (Nexus Mentality / Forgotten Ancient): `target` is
            // already resolved (the moved-from permanent); pause on a ChooseTarget for the
            // second permanent, mirroring `Fight`'s cast/resolution split.
            Effect::MoveCounters {
                to_filter,
                all_kinds,
                distributed,
                ..
            } => {
                let from = expect_object_target(target, "a move-counters effect's source");
                let legal: Vec<ObjectId> = self
                    .legal_targets_for(
                        TargetSpec::Permanent(to_filter),
                        source,
                        controller,
                        [false; Color::COUNT],
                        x,
                    )
                    .into_iter()
                    .filter_map(|t| (t != Target::Object(from)).then_some(t.object_id()?))
                    .collect();
                if legal.is_empty() {
                    return;
                }
                // Forgotten Ancient's "distributed as you choose among any number of target
                // creatures" (CR 601.2d): pause on a target→amount map capped at `from`'s live
                // +1/+1 count, rather than choosing one destination for the whole pile.
                if distributed {
                    let cap = self.permanent(from).plus_counters;
                    if cap <= 0 {
                        return; // nothing to move — "any number" tops out at zero.
                    }
                    crate::pending::raise_choice(
                        self,
                        PendingChoice::DivideMovedCounters {
                            player: controller,
                            from,
                            legal,
                            cap,
                        },
                    );
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::MoveCounters {
                            target: TargetSpec::None,
                            to_filter,
                            all_kinds,
                            distributed,
                            from: Some(Target::Object(from)),
                        },
                        legal: legal.into_iter().map(Target::Object).collect(),
                        count: TargetCount::default(),
                        x: 0,
                        activated: false,
                    },
                );
            }
            _ => unreachable!("fight pause family received a non-family effect"),
        }
    }
}
