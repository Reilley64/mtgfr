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
                who,
                or_one_matching,
                ..
            }) => {
                let Some(player) = self.sole_player_in(who, controller, target) else {
                    return;
                };
                let count = self.resolve_amount(count, controller, source, target, ctx.x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::Discard {
                        player,
                        count: count.max(0) as u32,
                        or_one_matching,
                    },
                )
            }
            // Brainstorm's "put two cards from your hand on top of your library in any order"
            // pauses on an ordered card-pick choice over the controller's own hand.
            Effect::Choice(ChoiceEffect::PutFromHandOnTop {
                count,
                drawn_this_turn,
                life_per_declined,
            }) => pending::raise(
                self,
                pending::ChoiceRequest::PutFromHandOnTop {
                    player: controller,
                    count,
                    drawn_this_turn,
                    life_per_declined,
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
            // Kaalia restricts eligibility to `subtypes`, sets `keep`, and carries the threaded
            // `defender` so the put-in creature enters tapped and attacking that opponent.
            Effect::Choice(ChoiceEffect::PutCreatureFromHand {
                subtypes,
                keep,
                defender,
            }) => pending::raise(
                self,
                pending::ChoiceRequest::PutCreatureFromHand {
                    player: controller,
                    source,
                    subtypes,
                    keep,
                    defender,
                    round: None,
                    permanent_cards: false,
                },
            ),
            // Eureka: "Starting with you, each player may put a permanent card from their hand onto
            // the battlefield. Repeat this process until no one puts a card onto the battlefield."
            // The same offer as above, widened to permanent cards and carrying the lap queue —
            // `Game::offer_next_in_put_round` re-raises it for each seat in turn until a whole lap
            // declines. Nothing put out this way is sacrificed, so `keep`.
            Effect::Choice(ChoiceEffect::EachPlayerMayPutPermanentFromHandRepeating) => {
                let lap = self.turn_order_from(controller);
                let Some((&first, rest)) = lap.split_first() else {
                    return;
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::PutCreatureFromHand {
                        player: first,
                        source,
                        subtypes: &[],
                        keep: true,
                        defender: None,
                        round: Some(rest.to_vec()),
                        permanent_cards: true,
                    },
                )
            }
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
