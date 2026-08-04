//! Test/setup object minting and commander tax helpers.
//!
//! Seeded objects for tests and lobby setup; commander tax (CR 903). Deferred / gaps:
//! see per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;

impl Game {
    /// Test/setup helper: create a card in `player`'s hand, returning its id. Invalidates
    /// `player`'s cached characteristics — a hand-count static (Empyrial Armor's
    /// `grant_to_attached`) reads live off the hand, so a battlefield permanent's cached P/T
    /// would otherwise go stale the instant a test drops a card in here after an earlier read;
    /// see [`Self::spawn_in_graveyard`]'s doc comment.
    pub fn spawn_in_hand(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let def = intern_card_def(def);
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
        let def = intern_card_def(def);
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
        let def = intern_card_def(def);
        let id = self.create_object(
            None,
            Object::Permanent(Permanent {
                entered_this_turn: false,
                ..fresh_permanent(def, player, false, false)
            }),
        );
        self.permanent_mut(id).continuous_timestamp = self.stamp_continuous_timestamp();
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Test/setup helper: put a token directly onto `player`'s battlefield (not summoning
    /// sick, as if it had been there since before the turn) — the token equivalent of
    /// [`Self::spawn_on_battlefield`]. Invalidates `player`'s cached characteristics — see
    /// [`Self::spawn_in_graveyard`]'s doc comment.
    pub fn spawn_token_on_battlefield(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let def = intern_card_def(def);
        let id = self.create_object(
            None,
            Object::Permanent(Permanent {
                summoning_sick: false,
                entered_this_turn: false,
                ..fresh_token(def, player)
            }),
        );
        self.permanent_mut(id).continuous_timestamp = self.stamp_continuous_timestamp();
        self.characteristics_cache
            .write(|cache| cache.invalidate_owner(self, player));
        id
    }

    /// Setup: create `player`'s commander in the command zone and set Commander life (40).
    pub fn designate_commander(&mut self, player: PlayerId, def: CardDef) -> ObjectId {
        let def = intern_card_def(def);
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
            Object::Moved { .. } | Object::Removed { .. } => {
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
        // Disintegrate's "if it would die this turn, exile it instead" is the same replacement
        // under a different name, so it rides the same guard and inherits the same commander
        // ponytail above.
        if self
            .as_permanent(from)
            .is_some_and(|p| p.finality_counter || p.exile_instead_of_dying_this_turn)
            && !self.is_commander(from)
        {
            return Event::MovedToExile { card: new_id, from };
        }
        // Serra Paragon's granted rider (CR 118.9 — "When this permanent is put into a graveyard
        // from the battlefield, exile it and you gain 2 life.") is a real placed trigger, not a
        // zone redirect: the tagged permanent genuinely dies here (a commander still diverts to
        // the command zone below, same as any other death), and `Game::enqueue_triggers`
        // fabricates the exile-and-gain-2-life trigger off the real `Event::MovedToGraveyard` —
        // see `Effect::Zone(ZoneEffect::ExileGraveyardObjectGainLife)`.
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::*;

    const P0: PlayerId = PlayerId(0);

    fn forest() -> CardDef {
        CardDef {
            name: "Forest",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            enters_tapped_unless_you_pay_life: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
            approximates: None,
            oracle: None,
            sets: empty_slice(),
            subtypes: empty_slice(),
            otags: empty_slice(),
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn vanilla_creature(name: &'static str, oracle_id: &'static str) -> CardDef {
        CardDef {
            name,
            id: oracle_id,
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Creature {
                power: 2,
                toughness: 2,
                also: TypeSet::NONE,
            },
            legendary: false,
            snow: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            enters_tapped_unless_you_pay_life: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
            approximates: None,
            oracle: None,
            sets: empty_slice(),
            subtypes: empty_slice(),
            otags: empty_slice(),
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn spell(name: &'static str) -> CardDef {
        CardDef {
            kind: CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            ..vanilla_creature(name, "")
        }
    }

    fn intern_halves(halves: Vec<CardDef>) -> Arc<[CardId]> {
        halves.into_iter().map(intern_card_def).collect()
    }

    #[test]
    fn intern_card_def_round_trips_name() {
        let id = intern_card_def(forest());
        assert_eq!(card_def(id).name, "Forest");
    }

    #[test]
    fn same_non_empty_oracle_id_reuses_card_id() {
        let first = intern_card_def(vanilla_creature("Oracle Front", "oracle-dedupe-test"));
        let second = intern_card_def(vanilla_creature("Oracle Front Copy", "oracle-dedupe-test"));
        assert_eq!(first, second);
    }

    #[test]
    fn empty_id_stubs_still_get_distinct_ids() {
        let a = intern_card_def(forest());
        let b = intern_card_def(forest());
        assert_ne!(a, b);
    }

    #[test]
    fn flipped_permanent_def_id_is_stable_across_reads() {
        let back = intern_card_def(vanilla_creature("Flipper Back", ""));
        let front = CardDef {
            name: "Flipper Front",
            back: Some(back),
            ..vanilla_creature("Flipper Front", "")
        };
        let mut game = Game::new();
        let permanent = game.spawn_on_battlefield(P0, front);
        game.apply(&Event::Flipped { object: permanent });

        let before = interned_len();
        let first = game.def_id_of(permanent);
        let after_first = interned_len();
        let second = game.def_id_of(permanent);
        let after_second = interned_len();

        assert_eq!(
            first, second,
            "the flipped back-face CardId should be stable"
        );
        assert_eq!(
            after_first, after_second,
            "reading the flipped back face twice must not intern again"
        );
        assert!(
            after_first >= before,
            "the first read may intern once, but the second read must be pure"
        );
    }

    #[test]
    fn adventure_restore_reuses_front_face_card_id() {
        let adventure = intern_card_def(spell("Adventure Half"));
        let front = CardDef {
            name: "Adventure Front",
            adventure: Some(adventure),
            ..vanilla_creature("Adventure Front", "")
        };
        let mut game = Game::new();
        let source = game.spawn_in_hand(P0, front);
        let front_id = game.def_id_of(source);
        let spell = game.next_object_id();

        game.apply(&Event::AdventureSpellCast {
            spell,
            source,
            controller: P0,
            target: None,
            x: 0,
        });
        let after_cast = interned_len();
        let exiled = game.next_object_id();
        game.apply(&Event::ExiledOnAdventure {
            card: exiled,
            from: spell,
            owner: P0,
        });

        let Object::Card(exiled_card) = &game.objects[exiled as usize] else {
            panic!("adventure resolution should restore a card in exile");
        };
        assert_eq!(exiled_card.def, front_id);
        assert_eq!(
            interned_len(),
            after_cast,
            "restoring the creature front face from adventure must not reintern it"
        );
    }

    #[test]
    fn split_half_restore_reuses_fused_card_id() {
        let before_spawn = interned_len();
        let front = CardDef {
            name: "Fused Split",
            kind: CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            halves: intern_halves(vec![spell("Left Half"), spell("Right Half")]),
            ..vanilla_creature("Fused Split", "")
        };
        let mut game = Game::new();
        let source = game.spawn_in_hand(P0, front);
        let after_spawn = interned_len();
        let fused_id = game.def_id_of(source);
        let spell = game.next_object_id();

        assert_eq!(
            after_spawn,
            before_spawn + 3,
            "interning a split card should also intern both castable halves"
        );

        game.apply(&Event::SplitHalfSpellCast {
            spell,
            source,
            half: 0,
            controller: P0,
            target: None,
            x: 0,
        });
        let after_cast = interned_len();
        let graveyard_card = game.next_object_id();
        game.apply(&Event::MovedToGraveyard {
            card: graveyard_card,
            from: spell,
        });

        let Object::Card(restored_card) = &game.objects[graveyard_card as usize] else {
            panic!("a split-half spell leaving the stack should restore a card");
        };
        assert_eq!(restored_card.def, fused_id);
        assert_eq!(
            interned_len(),
            after_cast,
            "restoring the fused split card off the stack must not reintern it"
        );
    }
}
