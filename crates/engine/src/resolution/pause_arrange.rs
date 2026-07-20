//! ArrangeTop pause family — [`Effect::Scry`] / [`Effect::Surveil`] (CR 701.42 / 701.43).
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
            Effect::Scry { count } => {
                let count = self.resolve_count(count, controller, source, target, x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::ArrangeTop {
                        player: controller,
                        count,
                        to_graveyard: false,
                    },
                )
            }
            Effect::Surveil { count } => pending::raise(
                self,
                pending::ChoiceRequest::ArrangeTop {
                    player: controller,
                    count,
                    to_graveyard: true,
                },
            ),
            _ => unreachable!("arrange-top pause family received a non-family effect"),
        }
    }
}
