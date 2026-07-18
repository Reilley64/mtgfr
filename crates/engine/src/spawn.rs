//! Test/setup object minting and commander tax helpers.
//!
//! Seeded objects for tests and lobby setup; commander tax (CR 903). Deferred / gaps:
//! see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

impl Game {
    /// Test/setup helper: create a card in `player`'s hand, returning its id. Invalidates
    /// `player`'s cached characteristics — a hand-count static (Empyrial Armor's
    /// `grant_to_attached`) reads live off the hand, so a battlefield permanent's cached P/T
    /// would otherwise go stale the instant a test drops a card in here after an earlier read;
    /// see [`Self::spawn_in_graveyard`]'s doc comment.
    pub fn spawn_in_hand(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Card(Card {
                def,
                owner: player,
                zone: Zone::Hand,
                commander: false,
                face_down: false,
            }),
        );
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Test/setup helper: create a card directly in `player`'s graveyard, returning its id.
    /// Invalidates `player`'s cached characteristics — a graveyard-count static (Wight of the
    /// Reliquary) reads live off the graveyard, so a battlefield permanent's cached P/T would
    /// otherwise go stale the instant a test drops a card in here after an earlier read.
    pub fn spawn_in_graveyard(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Card(Card {
                def,
                owner: player,
                zone: Zone::Graveyard,
                commander: false,
                face_down: false,
            }),
        );
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Test/setup helper: put a permanent directly onto `player`'s battlefield
    /// (not summoning sick, as if it had been there since before the turn). Invalidates
    /// `player`'s cached characteristics — see [`Self::spawn_in_graveyard`]'s doc comment.
    pub fn spawn_on_battlefield(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Permanent(Permanent {
                entered_this_turn: false,
                ..fresh_permanent(def, player, false, false)
            }),
        );
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Test/setup helper: put a token directly onto `player`'s battlefield (not summoning
    /// sick, as if it had been there since before the turn) — the token equivalent of
    /// [`Self::spawn_on_battlefield`]. Invalidates `player`'s cached characteristics — see
    /// [`Self::spawn_in_graveyard`]'s doc comment.
    pub fn spawn_token_on_battlefield(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Permanent(Permanent {
                summoning_sick: false,
                entered_this_turn: false,
                ..fresh_token(def, player)
            }),
        );
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Setup: create `player`'s commander in the command zone and set Commander life (40).
    pub fn designate_commander(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let id = self.create_object(
            None,
            Object::Card(Card {
                def,
                owner: player,
                zone: Zone::Command,
                commander: true,
                face_down: false,
            }),
        );
        self.set_life(player, COMMANDER_LIFE);
        id
    }

    /// Setup: mark an existing object as `player`'s commander and set Commander life (40).
    pub fn set_commander(&mut self, player: PlayerId, object: ObjectId) {
        match &mut self.objects[object as usize] {
            Object::Card(c) => c.commander = true,
            Object::Spell(s) => s.commander = true,
            Object::Permanent(p) => p.commander = true,
            Object::Moved { .. } | Object::Removed => {
                panic!("cannot make a moved-or-removed object a commander")
            }
        }
        self.set_life(player, COMMANDER_LIFE);
    }

    /// The additional generic mana it currently costs `player` to cast their commander
    /// from the command zone (2 per previous such cast).
    pub fn commander_tax(&self, player: PlayerId) -> u8 {
        2 * self.players[player.0 as usize].command_casts
    }

    /// The event for a permanent/spell leaving play to the graveyard — redirected to the
    /// command zone if it's a commander (a special-cased replacement effect, CR 903.9a).
    /// `new_id` is the id the resulting card will take.
    /// ponytail: the "may" (CR 903.9) always defaults to yes — no fixed soc-pool card needs
    ///   "no": none of the five deck commanders (Breena, Quintorius, Rootha, Beledros, Zimone)
    ///   has an implemented self Dies trigger to preserve (Atsushi's/Ao's modal Dies abilities
    ///   are dropped independently — see their card files), and since a diverted commander never
    ///   reaches a graveyard, Reanimate/Animate Dead/Karmic Guide simply never see it as a
    ///   candidate either way. Same rationale covers `exile_or_command`'s CR 903.9b diversion
    ///   below. Revisit if a target card ever needs to decline the command zone —
    ///   PendingChoice::MayYesNo won't reuse directly (it places a triggered ability, not a
    ///   mid-move zone redirect), so wire a new PendingChoice variant, thread it through both
    ///   diversions' callers (effects.rs, apply.rs, sacrifice_event), schema
    ///   (PendingChoiceView + an AnswerCommanderDivert-style intent), and the client's
    ///   PendingChoice Switch (Board.tsx) — the same pattern MayYesNo already used end-to-end.
    pub(crate) fn graveyard_or_command(&self, from: ObjectId, new_id: ObjectId) -> Event {
        // CR 614.12: a permanent with a finality counter that would die (be put into a graveyard
        // from the battlefield) is exiled instead. `from` is not always a battlefield permanent
        // here — `choices.rs` also routes a discarded hand card through this choke point — so use
        // the fallible `as_permanent` (never `self.permanent(from)`, which panics on a non-permanent).
        // ponytail: a commander with a finality counter is a CR 616 choice between two
        // replacements; we skip it (the `!is_commander` guard below lets the command-zone
        // diversion win) — no pool card is a commander with a finality counter.
        if self.as_permanent(from).is_some_and(|p| p.finality_counter) && !self.is_commander(from) {
            return Event::MovedToExile { card: new_id, from };
        }
        // Serra Paragon's granted rider (CR 118.9 — "When this permanent is put into a graveyard
        // from the battlefield, exile it and you gain 2 life.") is a real placed trigger, not a
        // zone redirect: the tagged permanent genuinely dies here (a commander still diverts to
        // the command zone below, same as any other death), and `Game::enqueue_triggers`
        // fabricates the exile-and-gain-2-life trigger off the real `Event::MovedToGraveyard` —
        // see `Effect::ExileGraveyardObjectGainLife`.
        if self.is_commander(from) {
            Event::MovedToCommandZone { card: new_id, from }
        } else {
            Event::MovedToGraveyard { card: new_id, from }
        }
    }

    /// The event for a permanent/spell being exiled — redirected to the command zone if it's a
    /// commander (CR 903.9b). `new_id` is the id the resulting card will take. See
    /// `graveyard_or_command`'s doc for the shared always-yes "may" rationale.
    pub(crate) fn exile_or_command(&self, from: ObjectId, new_id: ObjectId) -> Event {
        if self.is_commander(from) {
            Event::MovedToCommandZone { card: new_id, from }
        } else {
            Event::MovedToExile { card: new_id, from }
        }
    }

    /// The event for sacrificing the permanent at `id` (CR 701.16): it goes to the graveyard
    /// (or the command zone, for a commander), or ceases to exist if it's a token. Reuses the
    /// same death events as destruction, so "when this / a creature dies" triggers fire off it.
    pub(crate) fn sacrifice_event(&self, id: ObjectId) -> Event {
        let perm = self.permanent(id);
        if perm.token {
            return Event::TokenCeasedToExist {
                token: id,
                controller: perm.owner,
                def: perm.def,
            };
        }
        self.graveyard_or_command(id, self.next_object_id())
    }
}
