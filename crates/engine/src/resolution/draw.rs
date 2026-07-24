//! Draw-family event mint — pure Event vectors for [`DrawEffect::Cards`] and siblings.
//!
//! Dispatched via [`Game::mint_draw`] from the exhaustive mint match.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Draw Effect family.
    pub(crate) fn mint_draw(
        &self,
        effect: DrawEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        match effect {
            DrawEffect::Cards { count } => {
                self.mint_draw_cards(controller, source, target, x, count)
            }
            DrawEffect::TargetPlayer { count, .. } => {
                self.mint_target_player_draws(controller, source, target, x, count)
            }
            DrawEffect::EachPlayer { count } => {
                self.mint_each_player_draws(controller, source, target, x, count)
            }
            DrawEffect::AttackingPlayer { drawer, count } => {
                self.mint_attacking_player_draws(drawer, count)
            }
            DrawEffect::EachDrawStepPlayer { drawer, count } => {
                self.mint_each_draw_step_player_draws(drawer, count)
            }
            DrawEffect::TargetOwner {
                count,
                controller: to_controller,
            } => self.mint_target_owner_draws(controller, source, target, x, count, to_controller),
        }
    }

    /// Mint draw events for the ability's controller ([`DrawEffect::Cards`]).
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

    /// Mint draw events for a chosen player target ([`DrawEffect::TargetPlayer`]).
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

    /// Mint draw events for every living player ([`DrawEffect::EachPlayer`]).
    ///
    /// Ids are minted sequentially across every player's batch in one pass — [`Game::draw_events`]
    /// can't be called once per player here since each call restarts from the same
    /// not-yet-applied `next_object_id` (see `DestroyAll`'s `next` for the same reason).
    pub(crate) fn mint_each_player_draws(
        &self,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        count: Amount,
    ) -> Vec<Event> {
        let count = self.resolve_count(count, controller, source, target, x);
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

    /// Mint draw events for the attacking player ([`DrawEffect::AttackingPlayer`]).
    pub(crate) fn mint_attacking_player_draws(
        &self,
        drawer: Option<PlayerId>,
        count: u32,
    ) -> Vec<Event> {
        let drawer = drawer.expect("the attacking player is filled in at placement");
        self.draw_events(drawer, count)
    }

    /// Mint draw events for the player whose draw step it is
    /// ([`DrawEffect::EachDrawStepPlayer`] — Howling Mine).
    pub(crate) fn mint_each_draw_step_player_draws(
        &self,
        drawer: Option<PlayerId>,
        count: u32,
    ) -> Vec<Event> {
        let drawer = drawer.expect("the active player is filled in at placement");
        self.draw_events(drawer, count)
    }

    /// Mint draw events for the enclosing [`Sequence`](Effect::Sequence)'s shared target's owner
    /// or controller ([`DrawEffect::TargetOwner`] — Oblation's "then draws two cards" rider).
    pub(crate) fn mint_target_owner_draws(
        &self,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        count: Amount,
        to_controller: bool,
    ) -> Vec<Event> {
        let object = expect_object_target(target, "an owner/controller-draws amount");
        let drawer = self.owner_of_shared_target(object, to_controller);
        self.draw_events(
            drawer,
            self.resolve_count(count, controller, source, target, x),
        )
    }
}
