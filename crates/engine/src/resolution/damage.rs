//! Damage-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_damage(
        &self,
        effect: DamageEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            DamageEffect::Target {
                amount, divided, ..
            } => {
                let chosen = target.expect("a targeted effect resolves with a chosen target");
                // A divided spell's per-target amount was already settled (CR 601.2d) right
                // after targets were chosen — see `Game::maybe_begin_damage_division` — and
                // recorded on the resolving spell (`source` is that spell's own object id;
                // `divided` only appears on `Timing::Spell` effects, so this always resolves
                // through the spell path, never a triggered/activated ability's). (CR 602, CR 601, CR 603)
                let amount = if divided {
                    // A divided target's share was recorded on the spell: object shares on
                    // `damage_division`, player shares on `damage_division_players` (CR 601.2d).
                    match chosen {
                        Target::Object(id) => self
                            .spell(source)
                            .damage_division
                            .pairs()
                            .into_iter()
                            .find_map(|(t, amt)| (t == id).then_some(amt))
                            .unwrap_or(0),
                        Target::Player(p) => self
                            .spell(source)
                            .damage_division_players
                            .into_iter()
                            .flatten()
                            .find_map(|(t, amt)| (t == p).then_some(amt))
                            .unwrap_or(0),
                    }
                } else {
                    self.resolve_amount(amount, controller, source, target, x)
                };
                match chosen {
                    // Damage to a creature is marked (an SBA later checks it against toughness), (CR 704, CR 120.3)
                    // unless protection from the source's color prevents it (CR 702.16d).
                    Target::Object(object) => {
                        if self.damage_prevented_by_protection(object, Some(source)) {
                            return Vec::new();
                        }
                        // Phantom Centaur's self-shield prevents this damage outright and
                        // removes one of its own +1/+1 counters instead (CR 615).
                        if self.phantom_shield_active(object) {
                            return self
                                .phantom_shield_counter_removal(object)
                                .into_iter()
                                .collect();
                        }
                        // Damage to a planeswalker removes that many loyalty counters instead of
                        // being marked (CR 120.3c/306.9) — checked ahead of Tajic's creature-only
                        // prevention below, since a planeswalker is never "another creature".
                        if matches!(self.def_of(object).kind, CardKind::Planeswalker { .. }) {
                            return vec![Event::LoyaltyChanged {
                                object,
                                amount: -amount,
                            }];
                        }
                        // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                        if self.noncombat_damage_prevented_to_creature(object) {
                            return Vec::new();
                        }
                        vec![Event::DamageMarked {
                            object,
                            amount,
                            source: Some(source),
                        }]
                    }
                    // Damage to a player is life loss. ponytail: the commander-damage tally is
                    // combat-only (CR 903.10a), so a burn spell never adds to it.
                    Target::Player(player) => {
                        let mut events = vec![Event::LifeChanged {
                            player,
                            amount: -amount,
                            source: Some(source),
                        }];
                        // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                        if amount > 0 {
                            events.push(Event::DamageDealtToPlayer {
                                source,
                                player,
                                amount,
                            });
                            // Lifelink (CR 702.15/119.3) triggers on ANY damage the source
                            // deals, not just combat damage (Brion Stoutarm's fling ability).
                            events.extend(self.lifelink_gain(source, amount));
                        }
                        events
                    }
                }
            }
            // Mass damage: mark `amount` on every creature; the SBA sweep clears the dead. (CR 704, CR 120.3)
            // `amount` is resolved *per creature*, with that creature substituted in as the
            // resolving `source` (Wave of Reckoning: "each creature deals damage to itself equal
            // to its power" — `Amount::SourcePower` then reads each creature's own power). A
            // shared value (`Fixed`, `PerCreatureOnBattlefield` — Blasphemous Act, Chain
            // Reaction) doesn't read `source` at all, so per-creature resolution is a no-op
            // change for those: same total, computed once per creature instead of once overall.
            // ponytail: the event's own `source` field stays the ability's source (not each
            // creature) — CR 609.7 would want each creature as the damage's true source for
            // this self-damage spell, but no pool card's protection/lifelink/replacement reads
            // that distinction here.
            DamageEffect::EachCreature {
                amount,
                opponents_only,
                filter,
                include_planeswalkers,
            } => {
                // `Amount::IfSpellKicked` (CR 702.33d) reads the resolving *spell's* own kicked
                // flag, not any one creature's — pick the kicked/unkicked branch once here
                // against the ability's true `source`, before the per-creature substitution
                // below stands a creature in for `source` (needed for `Amount::SourcePower`,
                // Wave of Reckoning's "equal to its power") and `spell_was_kicked` on a creature
                // id would silently read false (Breath of Darigaaz, kicked, would otherwise
                // always resolve its "else" branch). Every other `Amount` variant is unaffected.
                let amount = match amount {
                    Amount::IfSpellKicked { then, else_ } => {
                        if self.spell_was_kicked(source) {
                            *then
                        } else {
                            *else_
                        }
                    }
                    // Disaster Radius's "X is the revealed card's mana value" (CR 601.2g) reads
                    // the resolving *spell's* own reveal-cost record, not any one creature's —
                    // same "pick it once against the true source" reasoning as `IfSpellKicked`
                    // above, before the per-creature substitution below stands a creature in for
                    // `source`.
                    Amount::RevealedCreatureManaValue => {
                        Amount::Fixed(self.revealed_creature_mana_value(source) as i32)
                    }
                    other => other,
                };
                // Volcanic Torrent's "and planeswalker" (CR 120.3c/306.9) — `include_planeswalkers`
                // widens the sweep beyond creatures; `false` preserves every other consumer's
                // creature-only sweep unchanged.
                let is_planeswalker =
                    |id: ObjectId| matches!(self.def_of(id).kind, CardKind::Planeswalker { .. });
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.is_creature_on_battlefield(id)
                            || (include_planeswalkers && is_planeswalker(id))
                    })
                    .filter(|&id| !opponents_only || self.controller_of(id) != controller)
                    // Breath of Darigaaz's "without flying" (or any future filter axis) — `None`
                    // preserves every existing consumer's unfiltered "every creature" sweep.
                    .filter(|&id| {
                        filter.is_none_or(|f| {
                            self.permanent_matches(&f, id, controller, Some(source))
                        })
                    })
                    // Protection from the source's color prevents that permanent's share (CR 702.16d).
                    .filter(|&id| !self.damage_prevented_by_protection(id, Some(source)))
                    // Tajic prevents noncombat damage to its controller's OTHER CREATURES (CR
                    // 615) — a planeswalker is never "another creature", so it's exempt (mirrors
                    // the single-target ordering comment above).
                    .filter(|&id| {
                        is_planeswalker(id) || !self.noncombat_damage_prevented_to_creature(id)
                    })
                    .flat_map(|object| {
                        // Damage to a planeswalker removes that many loyalty counters instead of
                        // being marked (CR 120.3c/306.9), ahead of Phantom Centaur's shield below
                        // since a planeswalker can never carry that creature-only static.
                        if is_planeswalker(object) {
                            return vec![Event::LoyaltyChanged {
                                object,
                                amount: -self.resolve_amount(amount, controller, object, target, x),
                            }];
                        }
                        // Phantom Centaur's self-shield prevents its own share and removes one of
                        // its own +1/+1 counters instead (CR 615) — a shielded creature swaps its
                        // `DamageMarked` for that counter removal rather than being filtered out
                        // outright.
                        if self.phantom_shield_active(object) {
                            return self
                                .phantom_shield_counter_removal(object)
                                .into_iter()
                                .collect();
                        }
                        vec![Event::DamageMarked {
                            object,
                            amount: self.resolve_amount(amount, controller, object, target, x),
                            source: Some(source),
                        }]
                    })
                    .collect()
            }
            // Breath of Darigaaz's "... and each player": real damage to every player, the
            // ability's own controller included — mirrors `DealDamage`'s `Target::Player` arm
            // (life loss + `DamageDealtToPlayer` + lifelink), fanned out once per living player
            // instead of once for a single chosen target. `amount` doesn't vary per player (no
            // pool card reads player-specific state here), so it's resolved once, shared by
            // every player's share.
            DamageEffect::EachPlayer { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.living_players()
                    .flat_map(|player| {
                        let mut events = vec![Event::LifeChanged {
                            player,
                            amount: -amount,
                            source: Some(source),
                        }];
                        // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                        if amount > 0 {
                            events.push(Event::DamageDealtToPlayer {
                                source,
                                player,
                                amount,
                            });
                            // Lifelink (CR 702.15e): a source dealing damage to multiple players
                            // gains life separately for each.
                            events.extend(self.lifelink_gain(source, amount));
                        }
                        events
                    })
                    .collect()
            }
            // Hydra Omnivore's splash: same per-player damage events as `DamageEachPlayer` above,
            // but only to opponents of the ability's controller (CR 102.3) other than the one who
            // already took the combat damage — that player is baked in at trigger placement.
            DamageEffect::EachOtherOpponent { amount, damaged } => {
                let damaged = damaged.expect("the damaged opponent is filled in at placement");
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.living_players()
                    .filter(|&player| player != controller && player != damaged)
                    .flat_map(|player| {
                        let mut events = vec![Event::LifeChanged {
                            player,
                            amount: -amount,
                            source: Some(source),
                        }];
                        // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                        if amount > 0 {
                            events.push(Event::DamageDealtToPlayer {
                                source,
                                player,
                                amount,
                            });
                            // Lifelink (CR 702.15e): a source dealing damage to multiple players
                            // gains life separately for each.
                            events.extend(self.lifelink_gain(source, amount));
                        }
                        events
                    })
                    .collect()
            }
            // Marauding Raptor: 2 damage to the permanent that just entered (context), not a
            // chosen target. `then_if_subtype`/`then` (the Dinosaur pump rider) are handled by
            // the caller in `run` — this leaf only deals the damage.
            DamageEffect::ToEnteringPermanent {
                entering, amount, ..
            } => {
                let object = entering.expect("the entering permanent is filled in at placement");
                if self.damage_prevented_by_protection(object, Some(source)) {
                    return Vec::new();
                }
                // Phantom Centaur's self-shield prevents this damage outright and removes one
                // of its own +1/+1 counters instead (CR 615).
                if self.phantom_shield_active(object) {
                    return self
                        .phantom_shield_counter_removal(object)
                        .into_iter()
                        .collect();
                }
                // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                if self.noncombat_damage_prevented_to_creature(object) {
                    return Vec::new();
                }
                vec![Event::DamageMarked {
                    object,
                    amount,
                    source: Some(source),
                }]
            }

            // Real damage to the ability's own controller — mirrors `DealDamage`'s
            // `Target::Player` arm, substituting `controller` for the chosen target.
            DamageEffect::ToSelf { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let mut events = vec![Event::LifeChanged {
                    player: controller,
                    amount: -amount,
                    source: Some(source),
                }];
                // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                if amount > 0 {
                    events.push(Event::DamageDealtToPlayer {
                        source,
                        player: controller,
                        amount,
                    });
                    // Lifelink (CR 702.15/119.3) triggers on ANY damage the source deals.
                    events.extend(self.lifelink_gain(source, amount));
                }
                events
            }
            // Lash Out's win rider: real damage to the *target creature's* controller, not the
            // ability's own controller — the player twin of `DealDamageToSelf` (CR 120.1), routed
            // through the same `DamageDealtToPlayer` life-loss + damage-watch events. The target is
            // the enclosing `Sequence`'s shared creature; `controller_of` follows `Object::Moved`,
            // so it still resolves even after the preceding 4-damage step killed the creature.
            DamageEffect::ToTargetController { amount } => {
                let creature = expect_object_target(target, "deal damage to target's controller");
                let recipient = self.controller_of(creature);
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let mut events = vec![Event::LifeChanged {
                    player: recipient,
                    amount: -amount,
                    source: Some(source),
                }];
                // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                if amount > 0 {
                    events.push(Event::DamageDealtToPlayer {
                        source,
                        player: recipient,
                        amount,
                    });
                    // Lifelink (CR 702.15/119.3) triggers on ANY damage the source deals.
                    events.extend(self.lifelink_gain(source, amount));
                }
                events
            }
        }
    }

    /// Marauding Raptor's [`DamageEffect::ToEnteringPermanent`] choreography: run the
    /// damage step (unchanged via `execute_effect`), then — CR "if a Dinosaur is dealt damage
    /// this way, this creature gets +2/+0 until end of turn" — run `then` only if the entering
    /// permanent's subtypes intersect `then_if_subtype` AND the damage actually landed (a
    /// `DamageMarked` event was produced — none means a protection/prevention shield stopped
    /// it, CR 119.3 "is dealt damage").
    pub(crate) fn resolve_deal_damage_to_entering(
        &mut self,
        effect: DamageEffect,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        let DamageEffect::ToEnteringPermanent {
            entering,
            then_if_subtype,
            then,
            ..
        } = effect
        else {
            unreachable!("resolve_deal_damage_to_entering received a non-family effect")
        };
        let evs = self.execute_effect(Effect::Damage(effect), controller, source, target, x);
        let damage_landed = evs.iter().any(|e| matches!(e, Event::DamageMarked { .. }));
        self.apply_all(&evs);
        events.extend(evs);
        if !damage_landed {
            return;
        }
        let entering = entering.expect("the entering permanent is filled in at placement");
        let is_matching_subtype = self
            .def_of(entering)
            .subtypes
            .iter()
            .any(|s| then_if_subtype.contains(s));
        if !is_matching_subtype {
            return;
        }
        self.run_sequence(then, ctx, events);
    }

    /// Lifelink (CR 702.15): if `source` has lifelink and dealt `amount` (>0) damage to a
    /// player, its controller gains that much life — the pure-mint twin of
    /// [`Game::gain_lifelink`] (`combat.rs`), appended to this effect's own event batch instead
    /// of applied immediately (this module never mutates the board).
    fn lifelink_gain(&self, source: ObjectId, amount: i32) -> Option<Event> {
        if amount <= 0 || !self.has_keyword(source, Keyword::Lifelink) {
            return None;
        }
        let player = self.controller_of(source);
        Some(Event::LifeChanged {
            player,
            amount: self.life_gain_after_replacements(player, amount),
            source: Some(source),
        })
    }
}
