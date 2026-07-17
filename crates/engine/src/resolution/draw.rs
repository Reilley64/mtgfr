//! Draw-family event mint — pure Event vectors for [`Effect::DrawCards`] and siblings.
//!
//! Dispatched via [`Game::mint_draw_family`] from the exhaustive mint match.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Draw Effect family.
    pub(crate) fn mint_draw_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        match effect {
            Effect::DrawCards { count } => {
                self.mint_draw_cards(controller, source, target, x, count)
            }
            Effect::TargetPlayerDraws { count, .. } => {
                self.mint_target_player_draws(controller, source, target, x, count)
            }
            Effect::EachPlayerDraws { count } => self.mint_each_player_draws(count),
            Effect::AttackingPlayerDraws { drawer, count } => {
                self.mint_attacking_player_draws(drawer, count)
            }
            _ => unreachable!("draw family mint received a non-family effect"),
        }
    }

    /// Mint draw events for the ability's controller ([`Effect::DrawCards`]).
    pub(crate) fn mint_draw_cards(
        &self,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        count: Amount,
    ) -> Vec<Event> {
        self.draw_events(
            controller,
            self.resolve_count(count, controller, source, target, x),
        )
    }

    /// Mint draw events for a chosen player target ([`Effect::TargetPlayerDraws`]).
    pub(crate) fn mint_target_player_draws(
        &self,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        count: Amount,
    ) -> Vec<Event> {
        let Some(Target::Player(player)) = target else {
            panic!("target-player-draws resolves with a chosen player target");
        };
        self.draw_events(
            player,
            self.resolve_count(count, controller, source, target, x),
        )
    }

    /// Mint draw events for every living player ([`Effect::EachPlayerDraws`]).
    ///
    /// Ids are minted sequentially across every player's batch in one pass — [`Game::draw_events`]
    /// can't be called once per player here since each call restarts from the same
    /// not-yet-applied `next_object_id` (see `DestroyAll`'s `next` for the same reason).
    pub(crate) fn mint_each_player_draws(&self, count: u32) -> Vec<Event> {
        let mut next = self.next_object_id();
        let mut events = Vec::new();
        for p in self.living_players() {
            let library = &self.players[p.0 as usize].library;
            for i in 0..count as usize {
                match library.get(i) {
                    Some(&from) => {
                        events.push(Event::CardDrawn {
                            player: p,
                            object: next,
                            from,
                            card: self.def_of(from),
                        });
                        next += 1;
                    }
                    None => events.push(Event::DrewFromEmptyLibrary { player: p }),
                }
            }
        }
        events
    }

    /// Mint draw events for the attacking player ([`Effect::AttackingPlayerDraws`]).
    pub(crate) fn mint_attacking_player_draws(
        &self,
        drawer: Option<PlayerId>,
        count: u32,
    ) -> Vec<Event> {
        let drawer = drawer.expect("the attacking player is filled in at placement");
        self.draw_events(drawer, count)
    }
}
