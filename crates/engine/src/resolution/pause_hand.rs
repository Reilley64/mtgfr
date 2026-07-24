//! Hand-pick pause family — discard / put-from-hand / face-down cast.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice.

use crate::*;

impl Game {
    /// Pause on Discard / PutFromHandOnTop / PutLandFromHand / PutCreatureFromHand /
    /// CastCreatureFaceDown for the matching effect.
    pub(crate) fn run_hand_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            source,
            target,
            spent_mana,
            ..
        } = ctx;
        match effect {
            // A discard pauses on a card-pick choice (the discarding player chooses which to
            // pitch): the ability's controller, or a chosen target player (Prismari Command).
            Effect::Choice(ChoiceEffect::Discard {
                count,
                target_player,
                or_one_matching,
            }) => {
                let discarder = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!("target-player discard resolves with a chosen player target");
                    };
                    player
                } else {
                    controller
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::Discard {
                        player: discarder,
                        count,
                        or_one_matching,
                    },
                )
            }
            // Brainstorm's "put two cards from your hand on top of your library in any order"
            // pauses on an ordered card-pick choice over the controller's own hand.
            Effect::Choice(ChoiceEffect::PutFromHandOnTop { count }) => pending::raise(
                self,
                pending::ChoiceRequest::PutFromHandOnTop {
                    player: controller,
                    count,
                },
            ),
            // "You may put a land from hand onto the battlefield" pauses on a card-pick choice
            // (up to one hand land, or decline).
            Effect::Choice(ChoiceEffect::PutLandFromHand { tapped }) => pending::raise(
                self,
                pending::ChoiceRequest::PutLandFromHand {
                    player: controller,
                    tapped,
                },
            ),
            // Cauldron Dance's "You may put a creature card from your hand onto the
            // battlefield" pauses on the creature sibling of `PutLandFromHand`'s card-pick
            // choice (up to one hand creature, or decline). `source` is threaded through so the
            // answer can later schedule the end-step sacrifice against this same ability.
            Effect::Choice(ChoiceEffect::PutCreatureFromHand) => pending::raise(
                self,
                pending::ChoiceRequest::PutCreatureFromHand {
                    player: controller,
                    source,
                },
            ),
            // Illusionary Mask's "you may cast a creature card in hand … face down as a 2/2"
            // pauses on a card-pick choice over the hand creatures whose mana cost the mana
            // spent on this ability's `{X}` could pay (`ctx.spent_mana`, CR 107.3).
            Effect::Choice(ChoiceEffect::CastCreatureFaceDown) => pending::raise(
                self,
                pending::ChoiceRequest::CastCreatureFaceDown {
                    player: controller,
                    spent_mana,
                },
            ),
            _ => unreachable!("hand pause family received a non-family effect"),
        }
    }
}
