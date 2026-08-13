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
                    .map(|player| self.life_gain(player, i64::from(amount), source))
                    .collect()
            }
            LifeEffect::Lose { who, amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.players_in(who, controller, target)
                    .into_iter()
                    .map(|player| Event::LifeChanged {
                        player,
                        amount: -i64::from(amount),
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
                    true => i64::from(amount).saturating_mul(losers.len() as i64),
                    false => i64::from(amount),
                };
                let mut events: Vec<Event> = losers
                    .into_iter()
                    .map(|player| Event::LifeChanged {
                        player,
                        amount: -i64::from(amount),
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
                        let delta = i64::from(highest) - i64::from(self.life(player));
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
                    let delta = i64::from(self.life(other)) - i64::from(self.life(controller));
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
                    amount: -(i64::from(self.life(owner).max(0)) + 1) / 2,
                    source: Some(source),
                }]
            }
        }
    }

    /// A life *gain* event, sized after the recipient's own gain replacements (CR 614) — the
    /// choke every gain in this family goes through, so none of them can skip a Rest for the
    /// Weary-style rider.
    fn life_gain(&self, player: PlayerId, amount: i64, source: ObjectId) -> Event {
        Event::LifeChanged {
            player,
            amount: self.life_gain_after_replacements(player, amount),
            source: Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    fn source(game: &mut Game) -> ObjectId {
        game.spawn_on_battlefield(
            P0,
            cards::get_by_name("Grizzly Bears").expect("fixture card"),
        )
    }

    fn apply_life_effect(
        game: &mut Game,
        effect: LifeEffect,
        source: ObjectId,
        target: Option<Target>,
    ) -> Vec<Event> {
        let minted = game.mint_life(effect, P0, source, target, 0);
        let mut events = Vec::new();
        game.apply_effect_events_with_replacements(minted, &mut events);
        events
    }

    #[test]
    fn each_player_becomes_highest_sets_minimum_life_to_maximum_exactly() {
        let mut game = Game::with_players(2, 0);
        let source = source(&mut game);
        game.players[P0.0 as usize].life = i32::MIN;
        game.players[P1.0 as usize].life = i32::MAX;

        let events = apply_life_effect(
            &mut game,
            LifeEffect::EachPlayerBecomesHighest,
            source,
            None,
        );

        assert_eq!(game.life(P0), i32::MAX);
        assert_eq!(game.life(P1), i32::MAX);
        assert_eq!(game.players[P0.0 as usize].life_gained_this_turn, u32::MAX);
        assert!(matches!(
            events.as_slice(),
            [Event::LifeChanged { player, amount, .. }]
                if *player == P0 && *amount == i64::from(u32::MAX)
        ));
    }

    #[test]
    fn exchange_swaps_minimum_and_maximum_life_exactly() {
        let mut game = Game::with_players(2, 0);
        let source = source(&mut game);
        game.players[P0.0 as usize].life = i32::MIN;
        game.players[P1.0 as usize].life = i32::MAX;

        let events = apply_life_effect(
            &mut game,
            LifeEffect::Exchange {
                who: PlayerSet::TargetOpponent,
            },
            source,
            Some(Target::Player(P1)),
        );

        assert_eq!(game.life(P0), i32::MAX);
        assert_eq!(game.life(P1), i32::MIN);
        assert_eq!(game.players[P0.0 as usize].life_gained_this_turn, u32::MAX);
        assert_eq!(game.players[P1.0 as usize].life_losses_this_turn, 1);
        assert_eq!(events.len(), 2, "one logical operation for each player");
        assert!(events.iter().any(|event| matches!(
            event,
            Event::LifeChanged { player, amount, .. }
                if *player == P0 && *amount == i64::from(u32::MAX)
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::LifeChanged { player, amount, .. }
                if *player == P1 && *amount == -i64::from(u32::MAX)
        )));
    }

    #[test]
    fn summed_multiplayer_drain_keeps_the_full_wide_gain() {
        let mut game = Game::with_players(3, 0);
        let source = source(&mut game);
        game.players[P0.0 as usize].life = i32::MIN;

        let events = apply_life_effect(
            &mut game,
            LifeEffect::Drain {
                who: PlayerSet::EachOpponent,
                amount: Amount::Fixed(i32::MAX),
                sum_gain: true,
            },
            source,
            None,
        );

        assert_eq!(game.life(P0), i32::MAX - 1);
        assert!(events.iter().any(|event| matches!(
            event,
            Event::LifeChanged { player, amount, .. }
                if *player == P0 && *amount == 2 * i64::from(i32::MAX)
        )));
    }

    #[test]
    fn source_owner_loses_half_maximum_life_rounded_up() {
        let mut game = Game::with_players(2, 0);
        let source = source(&mut game);
        game.players[P0.0 as usize].life = i32::MAX;

        let events = apply_life_effect(
            &mut game,
            LifeEffect::SourceOwnerLosesHalfTheirLife,
            source,
            None,
        );

        assert_eq!(game.life(P0), 1_073_741_823);
        assert!(matches!(
            events.as_slice(),
            [Event::LifeChanged { player, amount, .. }]
                if *player == P0 && *amount == -1_073_741_824
        ));
    }
}
