//! Library-look / search pause family — [`Effect::LookAtTop`], [`Effect::DistributeTop`],
//! [`Effect::SearchLibrary`].
//!
//! Pause peel behind [`Game::run`] (ADR 0002 deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on SelectFromTop / DistributeTop / SearchLibrary for the matching effect.
    pub(crate) fn run_look_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            target,
            ..
        } = ctx;
        match effect {
            // Look at the top N, select up to `up_to` matching cards into `dest`, rest to `rest`
            // (Quandrix Apprentice). Pauses on a SelectFromTop choice.
            Effect::LookAtTop {
                count,
                filter,
                up_to,
                min,
                dest,
                dest_tapped,
                rest,
                mv_budget,
            } => pending::raise(
                self,
                pending::ChoiceRequest::SelectFromTop {
                    player: controller,
                    count,
                    filter,
                    up_to,
                    min,
                    dest,
                    dest_tapped,
                    rest,
                    mv_budget,
                },
            ),
            // Look at the top N, route one card each to hand / bottom / exile-may-play
            // (Expressive Iteration). Pauses on a DistributeTop choice.
            Effect::DistributeTop {
                count,
                to_hand,
                to_bottom,
                to_exile_may_play,
            } => pending::raise(
                self,
                pending::ChoiceRequest::DistributeTop {
                    player: controller,
                    count,
                    to_hand,
                    to_bottom,
                    to_exile_may_play,
                },
            ),
            // A library search (fetchlands / tutors) pauses on a SearchLibrary choice. Usually
            // the ability's own controller searches; Path to Exile/Assassin's Trophy hand the
            // search to the exiled/destroyed permanent's controller instead (CR 701.19 doesn't
            // require the searcher to be the ability's controller).
            Effect::SearchLibrary {
                filter,
                to_zone,
                tapped,
                searcher,
                count,
                overflow,
            } => {
                let searching_player = match searcher {
                    SearchScope::You => controller,
                    SearchScope::TargetController => self.controller_of(expect_object_target(
                        target,
                        "a search effect's target-controller",
                    )),
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::SearchLibrary {
                        player: searching_player,
                        filter,
                        dest: to_zone,
                        tapped,
                        count,
                        overflow,
                    },
                )
            }
            _ => unreachable!("look pause family received a non-family effect"),
        }
    }
}
