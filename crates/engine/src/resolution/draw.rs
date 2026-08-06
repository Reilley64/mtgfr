//! Draw-family event mint — pure Event vectors for [`DrawEffect::Cards`].
//!
//! Dispatched via [`Game::mint_draw`] from the exhaustive mint match.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// Mint events for the Draw Effect family.
    ///
    /// ponytail: [`Game::run`] intercepts every `Effect::Draw` and routes it through
    /// [`Game::draw_with_replacements`] instead, so the only caller that can still land here is
    /// `Game::cast`'s alternative-cost rider — no printed rider draws, and one that did would
    /// bypass dredge and Chains of Mephistopheles. Route that rider through the funnel if a card
    /// ever prints one.
    pub(crate) fn mint_draw(
        &self,
        effect: DrawEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let DrawEffect::Cards { who, count } = effect;
        let count = self.resolve_count(count, controller, source, target, x);
        self.mint_draws_for(&self.players_in(who, controller, target), count)
    }

    /// Mint draw events for `players` in the order given, `count` cards each.
    ///
    /// Ids are minted sequentially across every player's batch in one pass — [`Game::draw_events`]
    /// can't be called once per player here since each call restarts from the same
    /// not-yet-applied `next_object_id` (see `DestroyAll`'s `next` for the same reason).
    pub(crate) fn mint_draws_for(&self, players: &[PlayerId], count: u32) -> Vec<Event> {
        let mut next = self.next_object_id();
        let mut events = Vec::new();
        for &player in players {
            let library = &self.players[player.0 as usize].library;
            for i in 0..count as usize {
                let Some(&from) = library.get(i) else {
                    events.push(Event::DrewFromEmptyLibrary { player });
                    continue;
                };
                events.push(Event::CardDrawn {
                    player,
                    object: next,
                    from,
                    card: self.def_id_of(from),
                });
                next += 1;
            }
        }
        events
    }
}
