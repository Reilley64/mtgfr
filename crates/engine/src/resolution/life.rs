//! Life-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Life Effect family, or [`None`] if `effect` is not in this family.
    pub(crate) fn try_mint_life(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Option<Vec<Event>> {
        if !matches!(
            effect,
            Effect::AttackerLosesLifeYouDraw { .. }
                | Effect::AttackerLosesLifeYouGain { .. }
                | Effect::DrainTarget { .. }
                | Effect::EachOpponentDrain { .. }
                | Effect::EachOpponentLosesLife { .. }
                | Effect::GainLife { .. }
                | Effect::GainLifeTargetController { .. }
                | Effect::LoseLife { .. }
                | Effect::TargetPlayerGainsLife { .. }
                | Effect::TargetPlayerLosesLife { .. }
        ) {
            return None;
        }
        Some(self.mint_life_family(effect, controller, source, target, x))
    }

    fn mint_life_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            Effect::GainLife { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                vec![Event::LifeChanged {
                    player: controller,
                    amount: self.life_gain_after_replacements(controller, amount),
                    source: Some(source),
                }]
            }
            Effect::LoseLife { amount } => vec![Event::LifeChanged {
                player: controller,
                amount: -self.resolve_amount(amount, controller, source, target, x),
                source: Some(source),
            }],
            // Swords to Plowshares' rider: the *target's* controller (its owner, per the
            // engine's control/ownership conflation) gains life, not this ability's controller.
            Effect::GainLifeTargetController { amount } => {
                let object = expect_object_target(target, "a controller-gains-life amount");
                let gainer = self.owner_of(object);
                let amount = self.resolve_amount(amount, controller, source, target, x);
                vec![Event::LifeChanged {
                    player: gainer,
                    amount: self.life_gain_after_replacements(gainer, amount),
                    source: Some(source),
                }]
            }
            // Parasitic Impetus: the enchanted creature's controller (context) loses `amount`
            // life; this ability's controller (the Aura's controller) gains the same.
            Effect::AttackerLosesLifeYouGain { attacker, amount } => {
                let loser = attacker.expect("the attacking player is filled in at placement");
                let amount = amount as i32;
                vec![
                    Event::LifeChanged {
                        player: loser,
                        amount: -amount,
                        source: Some(source),
                    },
                    Event::LifeChanged {
                        player: controller,
                        amount: self.life_gain_after_replacements(controller, amount),
                        source: Some(source),
                    },
                ]
            }
            // Tomik: the attacking opponent (context) loses `life_loss` life; this ability's
            // controller draws a card.
            Effect::AttackerLosesLifeYouDraw {
                attacker,
                life_loss,
            } => {
                let loser = attacker.expect("the attacking player is filled in at placement");
                let mut events = vec![Event::LifeChanged {
                    player: loser,
                    amount: -(life_loss as i32),
                    source: Some(source),
                }];
                events.extend(self.draw_events(controller, 1));
                events
            }
            // Blood Artist: the target player loses life, the controller gains the same.
            Effect::DrainTarget { amount, .. } => {
                let Some(Target::Player(loser)) = target else {
                    panic!("a targeted drain resolves with a chosen player target");
                };
                vec![
                    Event::LifeChanged {
                        player: loser,
                        amount: -amount,
                        source: Some(source),
                    },
                    Event::LifeChanged {
                        player: controller,
                        amount: self.life_gain_after_replacements(controller, amount),
                        source: Some(source),
                    },
                ]
            }
            // Questing Phelddagrif: the target player gains life, with no matching loss.
            Effect::TargetPlayerGainsLife { amount, .. } => {
                let Some(Target::Player(gainer)) = target else {
                    panic!("target-player-gains-life resolves with a chosen player target");
                };
                vec![Event::LifeChanged {
                    player: gainer,
                    amount: self.life_gain_after_replacements(gainer, amount),
                    source: Some(source),
                }]
            }
            // Zulaport Cutthroat: each opponent loses life; the controller gains a flat
            // `amount` — or, for Exsanguinate's "life lost this way", the summed total.
            Effect::EachOpponentDrain { amount, sum_gain } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let opponents: Vec<PlayerId> =
                    self.living_players().filter(|&p| p != controller).collect();
                let mut events: Vec<Event> = opponents
                    .iter()
                    .map(|&opponent| Event::LifeChanged {
                        player: opponent,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect();
                let gain = if sum_gain {
                    amount * opponents.len() as i32
                } else {
                    amount
                };
                events.push(Event::LifeChanged {
                    player: controller,
                    amount: self.life_gain_after_replacements(controller, gain),
                    source: Some(source),
                });
                events
            }
            // Dina, Soul Steeper: each opponent loses life, with no lifegain half (a gain would
            // re-trigger her "whenever you gain life" ability into a loop).
            Effect::EachOpponentLosesLife { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.living_players()
                    .filter(|&p| p != controller)
                    .map(|opponent| Event::LifeChanged {
                        player: opponent,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect()
            }
            // Ominous Harvest: the target player loses life, with no matching gain.
            Effect::TargetPlayerLosesLife { amount } => {
                let Some(Target::Player(player)) = target else {
                    panic!("target-player-loses-life resolves with a chosen player target");
                };
                vec![Event::LifeChanged {
                    player,
                    amount: -amount,
                    source: Some(source),
                }]
            }

            _ => unreachable!("life family mint received a non-family effect"),
        }
    }
}
