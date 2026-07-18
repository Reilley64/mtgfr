//! Zone queries, draw/shuffle, and mana-pool helpers.
//! Primary: CR 400 (zones), CR 121 (drawing a card), CR 106.4 (mana pool).
//!
//! Zone membership and library/hand operations; mana pool empties as turn-based
//! actions elsewhere. Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

impl Game {
    /// The amount of `color` mana currently in `player`'s pool.
    pub fn mana_in_pool(&self, player: PlayerId, color: Color) -> u8 {
        self.players[player.0 as usize].mana_pool.colored[color.index()]
    }

    /// Total mana floating in `player`'s pool, of every kind.
    pub fn floating_mana(&self, player: PlayerId) -> u32 {
        self.players[player.0 as usize].mana_pool.total()
    }

    /// The player's current mana pool.
    pub fn mana_pool(&self, player: PlayerId) -> &ManaPool {
        &self.players[player.0 as usize].mana_pool
    }

    /// The amount of colorless `{C}` mana currently in `player`'s pool.
    pub fn colorless_in_pool(&self, player: PlayerId) -> u8 {
        self.players[player.0 as usize].mana_pool.colorless
    }

    /// Test/setup helper: add a comfortable amount of every color plus colorless to `player`'s
    /// pool so cost-agnostic tests can cast without arranging lands.
    pub fn fund_mana(&mut self, player: PlayerId) {
        for mana in [
            Mana::Color(Color::White),
            Mana::Color(Color::Blue),
            Mana::Color(Color::Black),
            Mana::Color(Color::Red),
            Mana::Color(Color::Green),
            Mana::Colorless,
        ] {
            self.apply(&Event::ManaAdded {
                player,
                mana,
                amount: 20,
                persist: false,
            });
        }
    }

    /// The zone an object currently occupies — following its lineage if the id has since
    /// moved on (so an old id still reports where the card ended up).
    pub fn zone_of(&self, object: ObjectId) -> Zone {
        match self.objects[object as usize] {
            Object::Card(c) => c.zone,
            Object::Spell(_) => Zone::Stack,
            Object::Permanent(_) => Zone::Battlefield,
            Object::Moved { to } => self.zone_of(to),
            Object::Removed => panic!("object {object} has left the game"),
        }
    }

    /// Create a card on the bottom of `player`'s library, returning its id.
    pub(crate) fn spawn_in_library(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Card(Card {
                def,
                owner: player,
                zone: Zone::Library,
                commander: false,
                face_down: false,
            }),
        );
        self.players[player.0 as usize].library.push(id);
        id
    }

    /// Test/setup helper: replace `player`'s library with `defs` in order — index 0
    /// becomes the top of the library (drawn first). Returns the object ids in order.
    pub fn stack_library(&mut self, player: PlayerId, defs: &[CardDef]) -> Vec<ObjectId> {
        self.players[player.0 as usize].library.clear();
        defs.iter()
            .map(|&def| self.spawn_in_library(player, def))
            .collect()
    }

    /// Shuffle `player`'s library with the injected PRNG (Fisher-Yates).
    pub fn shuffle(&mut self, player: PlayerId) {
        let len = self.players[player.0 as usize].library.len();
        for i in (1..len).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            self.players[player.0 as usize].library.swap(i, j);
        }
    }

    /// Shuffle `player`'s library, then put `card` on top (CR 701.19 — Enlightened Tutor/Sterling
    /// Grove's "reveal it, then shuffle and put that card on top"). Pulls `card` out first so the
    /// shuffle can't relocate it — equivalent to shuffling everything and then overriding its
    /// position, since a uniform shuffle treats every card symmetrically before the override.
    /// Same-zone reorder, not a zone change (CR 400.7) — `card` keeps its object id.
    pub(crate) fn shuffle_then_put_on_top(&mut self, player: PlayerId, card: ObjectId) {
        self.players[player.0 as usize]
            .library
            .retain(|&o| o != card);
        self.shuffle(player);
        self.players[player.0 as usize].library.insert(0, card);
    }

    /// Draw the top card of `player`'s library into their hand. Drawing from an
    /// empty library flags the player to lose on the next SBA sweep (rule 104.3c).
    pub fn draw_card(&mut self, player: PlayerId) -> Vec<Event> {
        let events = self.draw_events(player, 1);
        self.apply_all(&events);
        events
    }

    /// The dredgers `player` may use to replace a single draw (CR 702.52): each card in their own
    /// graveyard carrying `dredge = Some(n)` whose N does not exceed the library size (CR 702.52a
    /// makes dredge illegal when the library is too small). Returns `(graveyard object id, N)` per
    /// eligible dredger; empty when none qualify, so the single-draw choke draws normally.
    pub(crate) fn dredge_options(&self, player: PlayerId) -> Vec<(ObjectId, u8)> {
        let library = self.players[player.0 as usize].library.len();
        self.graveyard_of(player)
            .into_iter()
            .filter_map(|id| {
                let n = self.def_of(id).dredge?;
                (library >= n as usize).then_some((id, n))
            })
            .collect()
    }

    /// Draw `remaining` cards for `player` one at a time, each individually replaceable by dredge
    /// (CR 702.52 — every draw is its own event, CR 121.2). Pauses on the first draw for which
    /// `player` has an eligible dredger, raising [`PendingChoice::ChooseDredge`] carrying `remaining`;
    /// [`Game::answer_choose_dredge`] resolves that one draw and re-enters here for `remaining - 1`,
    /// re-checking eligibility against the now-live graveyard/library. When no dredger qualifies the
    /// rest draw in one batch — no un-milled draw can newly create a dredger, so batching there is
    /// equivalent to drawing them singly. `from_draw_step` is threaded onto the pause to pick the
    /// answer handler's resume path.
    pub(crate) fn draw_with_dredge(
        &mut self,
        player: PlayerId,
        remaining: u32,
        from_draw_step: bool,
        events: &mut Vec<Event>,
    ) {
        if remaining == 0 {
            return;
        }
        let eligible = self.dredge_options(player);
        if eligible.is_empty() {
            let evs = self.draw_events(player, remaining);
            self.apply_all(&evs);
            events.extend(evs);
            return;
        }
        crate::pending::raise_choice(
            self,
            PendingChoice::ChooseDredge {
                player,
                eligible,
                remaining: remaining as u8,
                from_draw_step,
            },
        );
    }

    /// The events for `player` drawing `count` cards — pure (the caller applies them).
    /// Each successful draw mints a new hand-object id (`next + i`), matching the arena
    /// slots `apply` will push into.
    pub(crate) fn draw_events(&self, player: PlayerId, count: u32) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        (0..count as usize)
            .map(|i| match library.get(i) {
                Some(&from) => {
                    let event = Event::CardDrawn {
                        player,
                        object: next,
                        from,
                        card: self.def_of(from),
                    };
                    next += 1;
                    event
                }
                None => Event::DrewFromEmptyLibrary { player },
            })
            .collect()
    }

    /// The events for `player` milling the top `count` cards of their library into their
    /// graveyard — pure (the caller applies them). A library shorter than `count` mills only
    /// what's there; milling never sets the empty-draw flag, so it can't cause a loss. Each
    /// milled card mints a new graveyard-object id (`next + i`), matching the arena slots
    /// `apply` will push into.
    pub(crate) fn mill_events(&self, player: PlayerId, count: u32) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        library
            .iter()
            .take(count as usize)
            .map(|&from| {
                let event = Event::Milled {
                    player,
                    card: next,
                    from,
                };
                next += 1;
                event
            })
            .collect()
    }

    /// The events for `player` impulse-exiling the top `count` cards of their library face-up with
    /// permission to play them until end of turn (or until the end of their next turn, if
    /// `until_next_turn` — Atsushi's exile mode) — pure (the caller applies them). Mirrors
    /// [`Self::mill_events`]: a short library exiles only what's there; each mints a new exile id.
    pub(crate) fn exile_top_may_play_events(
        &self,
        player: PlayerId,
        count: u32,
        until_next_turn: bool,
    ) -> Vec<Event> {
        let library = self.players[player.0 as usize].library.clone();
        let mut next = self.next_object_id();
        library
            .iter()
            .take(count as usize)
            .map(|&from| {
                let event = Event::ExiledFromLibraryMayPlay {
                    player,
                    card: next,
                    from,
                    until_next_turn,
                };
                next += 1;
                event
            })
            .collect()
    }
}
