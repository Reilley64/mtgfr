//! Life-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// The seats a [`PlayerSet`] names, in turn order — the one place a card's "you" / "target
    /// player" / "each opponent" is turned into actual players.
    ///
    /// Dead seats are never included: [`Game::living_players`] is the roster, so an effect aimed
    /// at "each opponent" skips anyone who has already lost (CR 104.2a).
    pub(crate) fn players_in(
        &self,
        who: PlayerSet,
        controller: PlayerId,
        target: Option<Target>,
    ) -> Vec<PlayerId> {
        match who {
            PlayerSet::You => vec![controller],
            PlayerSet::EachPlayer => self.living_players().collect(),
            PlayerSet::EachOpponent => self.living_players().filter(|&p| p != controller).collect(),
            // CR 601.2f's alternative-cost rider names no target, so there is nothing to read back
            // — see the variant's ponytail note on the deterministic pick.
            PlayerSet::AnOpponent => self
                .living_players()
                .find(|&p| p != controller)
                .into_iter()
                .collect(),
            // The targeting machinery already picked and legality-checked the seat; a resolution
            // that finds no player target has lost it (CR 608.2b) and touches no one.
            PlayerSet::TargetPlayer | PlayerSet::TargetOpponent => match target {
                Some(Target::Player(player)) => vec![player],
                _ => Vec::new(),
            },
            // Swords to Plowshares: the *target's* controller (its owner, per the engine's
            // control/ownership conflation), not this ability's controller.
            PlayerSet::TargetsController => match target {
                Some(Target::Object(object)) => vec![self.owner_of(object)],
                _ => Vec::new(),
            },
            PlayerSet::AttackingPlayer { player } => {
                vec![player.expect("the attacking player is filled in at placement")]
            }
        }
    }

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
