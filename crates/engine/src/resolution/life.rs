//! Life-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_life(
        &self,
        effect: LifeEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        match effect {
            LifeEffect::Gain { who, amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.players_in(who, controller, target)
                    .into_iter()
                    .map(|player| self.life_gain(player, amount, source))
                    .collect()
            }
            LifeEffect::Lose { who, amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.players_in(who, controller, target)
                    .into_iter()
                    .map(|player| Event::LifeChanged {
                        player,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect()
            }
            LifeEffect::Drain {
                who,
                amount,
                sum_gain,
            } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let losers = self.players_in(who, controller, target);
                // Exsanguinate gains the total lost across every victim; Zulaport Cutthroat gains
                // the flat printed amount however many seats it drained.
                let gain = match sum_gain {
                    true => amount * losers.len() as i32,
                    false => amount,
                };
                let mut events: Vec<Event> = losers
                    .into_iter()
                    .map(|player| Event::LifeChanged {
                        player,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect();
                events.push(self.life_gain(controller, gain, source));
                events
            }
            // Arbiter of Knollridge: each player's life total becomes the highest life total
            // among all players (CR 118.5 — a set is a gain/loss of the difference). A player
            // already at the highest gets no event; every other living player's delta is routed
            // through the same gain/lose choke so lifegain watchers/replacements fire correctly.
            LifeEffect::EachPlayerBecomesHighest => {
                let highest = self
                    .living_players()
                    .map(|p| self.life(p))
                    .max()
                    .expect("at least one living player resolves this trigger");
                self.living_players()
                    .filter_map(|player| {
                        let delta = highest - self.life(player);
                        match delta.cmp(&0) {
                            std::cmp::Ordering::Equal => None,
                            std::cmp::Ordering::Greater => {
                                Some(self.life_gain(player, delta, source))
                            }
                            std::cmp::Ordering::Less => Some(Event::LifeChanged {
                                player,
                                amount: delta,
                                source: Some(source),
                            }),
                        }
                    })
                    .collect()
            }
            // Mirror Universe: "Exchange life totals with target opponent" (CR 118.7). Sized as
            // a delta per player, same pairwise shape as `EachPlayerBecomesHighest` above — one
            // `LifeChanged` for the controller, one for the opponent, so a life-gain trigger on
            // either side sees exactly one change, not a gain/loss pair per point exchanged.
            // Glyph of Life arms a delayed watch on `Game`, which needs `&mut self`.
            LifeEffect::GainWhenTargetIsDamagedByAttackerThisTurn { .. } => {
                unreachable!("a pausing/composite effect resolves via Game::run")
            }
            LifeEffect::Exchange { who } => self
                .players_in(who, controller, target)
                .into_iter()
                .flat_map(|other| {
                    let delta = self.life(other) - self.life(controller);
                    match delta.cmp(&0) {
                        std::cmp::Ordering::Equal => vec![],
                        std::cmp::Ordering::Greater => vec![
                            self.life_gain(controller, delta, source),
                            Event::LifeChanged {
                                player: other,
                                amount: -delta,
                                source: Some(source),
                            },
                        ],
                        std::cmp::Ordering::Less => vec![
                            Event::LifeChanged {
                                player: controller,
                                amount: delta,
                                source: Some(source),
                            },
                            self.life_gain(other, -delta, source),
                        ],
                    }
                })
                .collect(),
            LifeEffect::SourceOwnerLosesHalfTheirLife => {
                let owner = self.owner_of(source);
                // Rounded *up*, so an odd life total costs the extra point. A player already at
                // or below zero has nothing left to halve.
                vec![Event::LifeChanged {
                    player: owner,
                    amount: -(self.life(owner).max(0) + 1) / 2,
                    source: Some(source),
                }]
            }
        }
    }

    /// A life *gain* event, sized after the recipient's own gain replacements (CR 614) — the
    /// choke every gain in this family goes through, so none of them can skip a Rest for the
    /// Weary-style rider.
    fn life_gain(&self, player: PlayerId, amount: i32, source: ObjectId) -> Event {
        Event::LifeChanged {
            player,
            amount: self.life_gain_after_replacements(player, amount),
            source: Some(source),
        }
    }
}
