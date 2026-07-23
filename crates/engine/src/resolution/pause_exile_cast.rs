//! Exile-linked / graveyard-shuffle pause family — cash-out, free-cast, shuffle-from-graveyard.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on CashOutExiledWithThis / CastExiledWithThisFree /
    /// ShuffleTargetCardsFromGraveyardIntoLibrary for the matching effect.
    pub(crate) fn run_exile_cast_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            source,
            target,
            ..
        } = ctx;
        match effect {
            // "Put a card exiled with this" pauses on a card-pick choice over this source's
            // exiled-with pile (up to one, or decline).
            Effect::Dig(DigEffect::CashOutExiledWithThis) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseExiledWithCard {
                    player: controller,
                    source,
                },
            ),
            // Quintorius's activated ability pauses on a card-pick choice over this source's (CR 602, CR 113)
            // linked exile pile, granting the free-cast permission for the chosen card instead
            // of cashing it out.
            Effect::Dig(DigEffect::CastExiledWithThisFree) => pending::raise(
                self,
                pending::ChoiceRequest::ChooseExiledWithCardToCast {
                    player: controller,
                    source,
                },
            ),
            // Perpetual Timepiece ("Shuffle any number of target cards from your graveyard into
            // your library", `target_player = false`) and Quandrix Command mode 3 ("Target
            // player shuffles up to three target cards from their graveyard into their
            // library", `target_player = true`) both pause on a ShuffleFromGraveyard choice —
            // the graveyard owner is the ability's controller or the targeted player.
            Effect::Dig(DigEffect::ShuffleTargetCardsFromGraveyardIntoLibrary {
                max,
                target_player,
            }) => {
                let owner = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!("target-player shuffle resolves with a chosen player target");
                    };
                    player
                } else {
                    controller
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::ShuffleFromGraveyard {
                        answerer: controller,
                        owner,
                        source,
                        max,
                    },
                )
            }
            _ => unreachable!("exile-cast pause family received a non-family effect"),
        }
    }
}
