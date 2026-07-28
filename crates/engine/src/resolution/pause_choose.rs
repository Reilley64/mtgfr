//! Choose-* / proliferate / phase-out / demonstrate pause family.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on ChooseCreatureType / ChooseColor / SetOwnColorUntilEndOfTurn / ChooseOne /
    /// Demonstrate / Proliferate / PhaseOut for the matching effect.
    pub(crate) fn run_choose_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        match effect {
            // Patchwork Banner's "As this artifact enters, choose a creature type": pause on a
            // ChooseCreatureType for the controller, over the pool's known creature types.
            Effect::Choice(ChoiceEffect::ChooseCreatureType) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseCreatureType {
                    player: controller,
                    source,
                    options: CREATURE_TYPES,
                },
            ),
            // Phantasmal Terrain's "As this Aura enters, choose a basic land type": the same
            // picker, narrowed to the five basic land types.
            Effect::Choice(ChoiceEffect::ChooseBasicLandType) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseCreatureType {
                    player: controller,
                    source,
                    options: BASIC_LAND_TYPES,
                },
            ),
            // Flickering Ward's "As this Aura enters, choose a color": pause on a ChooseColor for (CR 702.21, CR 303.4)
            // the controller over the fixed five colors.
            Effect::Choice(ChoiceEffect::ChooseColor) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseColor {
                    player: controller,
                    source,
                    until_end_of_turn: false,
                },
            ),
            // Black Vise's "As this artifact enters, choose an opponent": the shared "an opponent
            // ..." picker, which collapses on its own when only one opponent is alive.
            Effect::Choice(ChoiceEffect::ChooseOpponent) => self.choose_splitting_opponent(
                controller,
                source,
                SplittingContinuation::RememberAsChosenOpponent,
            ),
            // Wild Mongrel's "...and becomes the color of your choice until end of turn": the same (CR 613.3c)
            // ChooseColor picker as `ChooseColor` above, but the answer sets an until-end-of-turn
            // color-SET instead of the indefinite `chosen_color`.
            Effect::Choice(ChoiceEffect::SetOwnColorUntilEndOfTurn) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseColor {
                    player: controller,
                    source,
                    until_end_of_turn: true,
                },
            ),
            // "Choose one —" reached mid-resolution — a modal spell's own resolution step (CR
            // 608.2, Zimone's Hypothesis), not a triggered ability (those choose their mode at
            // placement, see `place_pending_triggers`). Pause on a ChooseMode; the chosen mode
            // runs immediately through this same pipeline (see `answer_choose_mode`), carrying
            // this effect's `source`/`target`/`x` context. An empty mode list is a defensive no-op.
            Effect::ChooseOne { options } => {
                if options.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseMode {
                        player: controller,
                        source,
                        target,
                        x,
                        modes: options,
                        at_placement: false,
                        activated: false,
                    },
                );
            }
            // Demonstrate (CR 702.147): pause on a MayYesNo "copy it?" over the cast spell
            // (`spell` baked in at placement, see `CardDef::demonstrate`). The spell may have
            // been countered in response before this trigger resolved (CR 707.10c guard, same
            // shape as `CopyTriggeringSpell`): nothing left to copy.
            Effect::Copy(CopyEffect::Demonstrate { spell }) => {
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::Copy(CopyEffect::Demonstrate { spell }),
                        resume: crate::MayYesNoResume::Default,
                    },
                );
            }
            // Proliferate (CR 701.27) pauses on a Proliferate choice over every counter-bearing
            // permanent; `times` (Expansion Algorithm's {X}) may re-pause after this iteration.
            Effect::Choice(ChoiceEffect::Proliferate { times }) => {
                let n = self.resolve_count(times, controller, source, target, x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::Proliferate {
                        player: controller,
                        source,
                        remaining: n as u8,
                    },
                );
            }
            // Guardian of Faith's ETB (CR 702.26): pause to choose any number of the *other*
            // creatures its controller controls to phase out. Nothing to choose with no other
            // creature — skip past (like Proliferate's empty board).
            Effect::Choice(ChoiceEffect::PhaseOut) => pending::raise(
                self,
                pending::ChoiceRequest::PhaseOut {
                    player: controller,
                    source,
                },
            ),
            _ => unreachable!("choose pause family received a non-family effect"),
        }
    }
}
