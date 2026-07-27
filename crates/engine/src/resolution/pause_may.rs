//! Optional / may-* pause family — may-sacrifice, may-draw, may-discard, sacrifice-unless-pay.
//!
//! Pause peel behind [`Game::run`] (card-dsl-and-card-pool spec deepen). Pause bookkeeping stays in
//! [`crate::pending`]; this module only raises the choice (plus guard-returns that skip the pause).

use crate::*;

impl Game {
    /// Pause on the matching may-* / PayOrElse effect.
    pub(crate) fn run_may_pause(&mut self, effect: Effect, ctx: ResolveCtx) {
        let ResolveCtx {
            controller,
            source,
            target,
            ..
        } = ctx;
        match effect {
            // A resolution-time optional sacrifice (Witherbloom Charm mode 0) pauses on a
            // MaySacrifice choice; declining runs nothing.
            Effect::Choice(ChoiceEffect::MaySacrifice { filter, then }) => pending::raise(
                self,
                pending::ChoiceRequest::MaySacrifice {
                    player: controller,
                    source,
                    filter,
                    then,
                },
            ),
            // A resolution-time optional graveyard return (Deadly Brew's rider) pauses on a
            // MayReturnFromGraveyard choice; declining runs nothing. "If you sacrificed a
            // permanent this way" (Deadly Brew) gates the whole rider on the edict's own
            // controller having actually sacrificed — unmet, it's the same "runs nothing" as
            // declining, no pause at all.
            Effect::Choice(ChoiceEffect::MayReturnFromGraveyard {
                filter,
                if_you_sacrificed_this_way,
                mandatory,
            }) => {
                if if_you_sacrificed_this_way
                    && !self.resolution_frame.sacrificed_by_edict_controller
                {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayReturnFromGraveyard {
                        player: controller,
                        source,
                        filter,
                        mandatory,
                    },
                )
            }
            // A resolution-time optional "put a +1/+1 counter on a creature" (Zimone's Hypothesis'
            // primer) pauses on a MayPutCounterOnCreature choice over every battlefield creature;
            // declining runs nothing. No creature to offer skips the pause outright.
            Effect::Choice(ChoiceEffect::MayPutCounterOnCreature) => pending::raise(
                self,
                pending::ChoiceRequest::MayPutCounterOnCreature {
                    player: controller,
                    source,
                },
            ),
            // Conspiracy Theorist's batch nonland-discard payoff: "you may exile one of them from
            // your graveyard." Pauses on a MayExileDiscardedToPlay choice over the discarded
            // nonland cards still in the graveyard; declining (or none still there) runs nothing.
            Effect::Choice(ChoiceEffect::MayExileDiscardedNonlandMayPlay { cards }) => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayExileDiscardedNonlandMayPlay {
                        player: controller,
                        source,
                        cards,
                    },
                )
            }
            // A resolution-time optional discard (Quintorius, History Chaser's +1) pauses on a
            // MayDiscard choice; declining runs nothing.
            Effect::Choice(ChoiceEffect::MayDiscard { then }) => pending::raise(
                self,
                pending::ChoiceRequest::MayDiscard {
                    player: controller,
                    source,
                    then,
                },
            ),
            // Natural Selection's tail: "You may have that player shuffle." The caster decides —
            // they just ordered that library and may throw their own ordering away. The targeted
            // player is baked into the effect so the answer knows whose library to shuffle.
            Effect::Dig(DigEffect::MayShuffleTargetPlayersLibrary { .. }) => {
                let Some(Target::Player(owner)) = target else {
                    return;
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::Dig(DigEffect::MayShuffleTargetPlayersLibrary {
                            owner: Some(owner),
                        }),
                        resume: crate::MayYesNoResume::Default,
                    },
                );
            }
            // Rhystic Study's "you may draw a card unless that player pays {1}": pause the
            // ability's own controller on whether they want to draw at all (the card's ruling —
            // declining is quiet, no pay window is ever offered). Only a "yes" here raises the
            // triggering opponent's own pay-or-let-it-happen pause (`Game::answer_may`).
            Effect::Choice(ChoiceEffect::MayDrawUnlessPays { cost, caster }) => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::Choice(ChoiceEffect::MayDrawUnlessPays { cost, caster }),
                        resume: crate::MayYesNoResume::Default,
                    },
                );
            }
            // Questing Phelddagrif's blue rider: "Target opponent may draw a card." Unlike
            // `MayDrawUnlessPays` above, the *targeted* player answers (no pay window rides
            // behind it) — see `Game::answer_may`.
            Effect::Choice(ChoiceEffect::TargetPlayerMayDraw { count, opponent }) => {
                let Some(Target::Player(player)) = target else {
                    panic!("target-player-may-draw resolves with a chosen player target");
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player,
                        source,
                        effect: Effect::Choice(ChoiceEffect::TargetPlayerMayDraw {
                            count,
                            opponent,
                        }),
                        resume: crate::MayYesNoResume::Default,
                    },
                );
            }
            // Edric, Spymaster of Trest: "its controller may draw a card." Like
            // `TargetPlayerMayDraw` above, the drawing player answers — here the controller of the
            // creature that dealt the combat damage, baked in at trigger placement.
            Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw { count, drawer }) => {
                let player = drawer.expect(
                    "the damaging creature's controller is baked in by contextualize_effect at \
                     combat-damage trigger placement",
                );
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player,
                        source,
                        effect: Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw {
                            count,
                            drawer,
                        }),
                        resume: crate::MayYesNoResume::Default,
                    },
                );
            }
            // Arcane Denial's countered-spell rider: "Its controller may draw up to two cards"
            // (CR 120.4 / 601.2c). Pause the resolving controller on a count choice `0..=max`;
            // the answer (`Game::answer_may_draw_up_to`) draws exactly the chosen number.
            Effect::Choice(ChoiceEffect::MayDrawUpTo { count }) => {
                let max = self
                    .resolve_count(count, controller, source, None, 0)
                    .min(u8::MAX as u32) as u8;
                pending::raise_choice(
                    self,
                    PendingChoice::MayDrawUpTo {
                        player: controller,
                        max,
                        effect: Effect::Choice(ChoiceEffect::MayDrawUpTo {
                            count: Amount::Fixed(i32::from(max)),
                        }),
                        resume: MayDrawUpToResume::Default,
                    },
                );
            }
            // Trade Secrets: "target opponent draws two cards, then you draw up to four cards"
            // (CR 120.4 / 601.2c). The mandatory opponent draw is a preceding `TargetPlayerDraws`
            // step sharing this Sequence's target; this step pauses the caster on a count choice
            // `0..=count` (`Game::answer_trade_secrets_caster_draw` chains to the opponent's
            // repeat-or-stop pause once answered).
            Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat { count }) => {
                let Some(Target::Player(opponent)) = target else {
                    panic!(
                        "may-draw-up-to-then-opponent-may-repeat resolves with a chosen opponent target"
                    );
                };
                let max = self
                    .resolve_count(count, controller, source, None, 0)
                    .min(u8::MAX as u32) as u8;
                pending::raise_choice(
                    self,
                    PendingChoice::MayDrawUpTo {
                        player: controller,
                        max,
                        effect: Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat {
                            count: Amount::Fixed(i32::from(max)),
                        }),
                        resume: MayDrawUpToResume::TradeSecretsRepeat { opponent, source },
                    },
                );
            }
            // "…unless you pay {cost}" (Rupture Spire's ETB, Phantasmal Forces' and Force of
            // Nature's upkeeps). Pauses on the same pay-or-decline shape Echo's
            // `PayEchoOrSacrifice` uses, under its own variant (these are real triggered
            // abilities, not Echo — CR 603.3b, not CR 702.31).
            Effect::Choice(ChoiceEffect::PayOrElse { cost, otherwise }) => pending::raise(
                self,
                pending::ChoiceRequest::PayOrElse {
                    player: controller,
                    source,
                    cost,
                    otherwise,
                },
            ),
            _ => unreachable!("may pause family received a non-family effect"),
        }
    }
}
