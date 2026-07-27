//! ArrangeTop pause family — [`Effect::Dig(DigEffect::Scry)`] / [`Effect::Dig(DigEffect::Surveil)`] (CR 701.42 / 701.43).
//!
//! First pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on [`pending::ChoiceRequest::ArrangeTop`] for scry (bottom) or surveil (graveyard).
    pub(crate) fn run_arrange_top(
        &mut self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) {
        match effect {
            Effect::Dig(DigEffect::Scry { count }) => {
                let count = self.resolve_count(count, controller, source, target, x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::ArrangeTop {
                        player: controller,
                        library: controller,
                        count,
                        rest: ArrangeRest::Bottom,
                    },
                )
            }
            Effect::Dig(DigEffect::Surveil { count }) => pending::raise(
                self,
                pending::ChoiceRequest::ArrangeTop {
                    player: controller,
                    library: controller,
                    count,
                    rest: ArrangeRest::Graveyard,
                },
            ),
            // Natural Selection sorts somebody else's library: the caster answers, the target's
            // library is the one shown, and every card goes back on top.
            Effect::Dig(DigEffect::RearrangeTargetPlayersTop { count }) => {
                let Some(Target::Player(owner)) = target else {
                    return;
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::ArrangeTop {
                        player: controller,
                        library: owner,
                        count,
                        rest: ArrangeRest::Nowhere,
                    },
                )
            }
            _ => unreachable!("arrange-top pause family received a non-family effect"),
        }
    }
}
