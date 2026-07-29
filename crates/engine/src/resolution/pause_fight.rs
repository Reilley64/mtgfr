//! Fight / move-counters pause family — resolution-time second-target peels.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`].

use crate::*;

impl Game {
    /// Pause on Fight (Primal Might's shape) / MoveCounters for the matching effect — or, for a
    /// plain two-target fight, apply it outright.
    pub(crate) fn run_fight_pause(
        &mut self,
        effect: Effect,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            target,
            targets_second,
            x,
            ..
        } = ctx;
        match effect {
            // Fight (CR 701.12): both creatures were chosen at announcement in printed order
            // (CR 601.2c) — `target` is the ally (clause 0), `targets_second` the enemy (clause 1,
            // `Effect::second_target`). Nothing left to choose, so apply the damage here.
            Effect::Misc(MiscEffect::Fight {
                ally_is_shared_target: false,
                one_way,
                ..
            }) => {
                let ally = expect_object_target(target, "a fight's creature you control");
                // CR 608.2b: the caller only re-checks the step's own (clause 0) target, so the
                // enemy is checked here — gone from the battlefield, or never chosen at all
                // because no opponent controlled a creature, and the damage half does nothing.
                let Some(enemy) = targets_second.primary().and_then(|t| t.object_id()) else {
                    return;
                };
                if !self.is_creature_on_battlefield(enemy) {
                    return;
                }
                self.fight(ally, enemy, one_way, events);
            }
            // Primal Might's mirror shape (CR 701.12): `target` is already the ally (the pumped
            // creature you control, chosen at cast by a preceding Sequence step); pause on an
            // *optional* ChooseTarget for the enemy ("fights up to one target creature you don't
            // control"). Guard-returns with no pause if the ally has since left the battlefield
            // or stopped being a creature (CR 608.2b — a fizzled shared target) or there's no
            // legal enemy — the pump still stands either way.
            Effect::Misc(MiscEffect::Fight {
                ally_is_shared_target: true,
                ..
            }) => {
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
                        // `one_way: false` is deliberate, not a dropped field: this shape hands
                        // the *enemy* to `Game::fight`'s `a` slot and the ally to `b`, so a
                        // one-way half would point the wrong way. No card pairs
                        // `ally_is_shared_target` with `one_way` — swap the slots here first if
                        // one ever does.
                        effect: Effect::Misc(MiscEffect::Fight {
                            enemy: Some(Target::Object(ally)),
                            ally_is_shared_target: false,
                            one_way: false,
                        }),
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
            Effect::Counters(CountersEffect::MoveCounters {
                to_filter,
                all_kinds,
                distributed,
                ..
            }) => {
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
                        effect: Effect::Counters(CountersEffect::MoveCounters {
                            target: TargetSpec::None,
                            to_filter,
                            all_kinds,
                            distributed,
                            from: Some(Target::Object(from)),
                        }),
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
