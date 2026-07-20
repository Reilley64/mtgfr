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
            Effect::ChooseCreatureType => pending::raise(
                self,
                pending::ChoiceRequest::ChooseCreatureType {
                    player: controller,
                    source,
                    options: CREATURE_TYPES,
                },
            ),
            // Flickering Ward's "As this Aura enters, choose a color": pause on a ChooseColor for (CR 702.21, CR 303.4)
            // the controller over the fixed five colors.
            Effect::ChooseColor => pending::raise(
                self,
                pending::ChoiceRequest::ChooseColor {
                    player: controller,
                    source,
                    until_end_of_turn: false,
                },
            ),
            // Wild Mongrel's "...and becomes the color of your choice until end of turn": the same (CR 613.3c)
            // ChooseColor picker as `ChooseColor` above, but the answer sets an until-end-of-turn
            // color-SET instead of the indefinite `chosen_color`.
            Effect::SetOwnColorUntilEndOfTurn => pending::raise(
                self,
                pending::ChoiceRequest::ChooseColor {
                    player: controller,
                    source,
                    until_end_of_turn: true,
                },
            ),
            // "Choose one —" on a triggered ability (CR 700.2): pause on a ChooseMode for the
            // controller. The chosen mode resolves later through this same pipeline (see
            // `answer_choose_mode`), carrying this ability's `source`/`target`/`x` context so a
            // mode that needs them still has them. An empty mode list is a defensive no-op.
            Effect::ChooseOne { modes } => {
                if modes.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseMode {
                        player: controller,
                        source,
                        target,
                        x,
                        modes,
                    },
                );
            }
            // Demonstrate (CR 702.147): pause on a MayYesNo "copy it?" over the cast spell
            // (`spell` baked in at placement, see `CardDef::demonstrate`). The spell may have
            // been countered in response before this trigger resolved (CR 707.10c guard, same
            // shape as `CopyTriggeringSpell`): nothing left to copy.
            Effect::Demonstrate { spell } => {
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::Demonstrate { spell },
                    },
                );
            }
            // Proliferate (CR 701.27) pauses on a Proliferate choice over every counter-bearing
            // permanent; `times` (Expansion Algorithm's {X}) may re-pause after this iteration.
            Effect::Proliferate { times } => {
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
            Effect::PhaseOut => pending::raise(
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
