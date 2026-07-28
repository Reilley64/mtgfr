//! Damage-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    /// The event that lands `amount` damage from `source` on creature `object` — the single choke
    /// every creature-damage mint routes through, so infect (CR 702.90b) reshapes all of them at
    /// once. An infect source's damage is dealt in the form of that many -1/-1 counters instead of
    /// being marked; everything else marks damage as usual (CR 120.3/506).
    ///
    /// Returns `(the events, the damage actually dealt)` — the second half is what's left of
    /// `amount` after this creature's [`spend_prevention_shields`](Self::spend_prevention_shields)
    /// took its bite (CR 615), so a caller that also mints damage *markers* (the combat-damage
    /// watch, lifelink, deathtouch) sizes them off damage that was really dealt.
    ///
    /// All-or-nothing prevention and protection stay the caller's job and run ahead of this call —
    /// a fully prevented hit never reaches here, so it places no counters either.
    ///
    /// ponytail: an infect hit emits no [`Event::DamageMarked`], so the two watchers that ride
    /// that event alone — Armadillo Cloak's enchanted-host damage trigger and Vampiric Dragon's
    /// `damaged_this_turn` tally (`triggers.rs`) — don't see it, though CR 120.3 says the damage
    /// was dealt. Upgrade path: a source-carrying `Event::DamageDealtToCreature` marker pushed
    /// alongside both forms (the creature twin of [`Event::DamageDealtToPlayer`]), with those
    /// watchers moved onto it.
    pub(crate) fn creature_damage_events(
        &self,
        source: ObjectId,
        object: ObjectId,
        amount: i32,
    ) -> (Vec<Event>, i32) {
        self.creature_damage_events_with_riders(source, object, amount, false, false)
    }

    /// Spend the "prevent the next N damage that would be dealt to `target` this turn" shields
    /// (CR 615) standing between `amount` damage and `target`, returning `(the event recording the
    /// spend, the damage left to deal)`. Both damage chokes call this, so every damage path in the
    /// game — combat, burn, fight, mass sweeps — is covered by construction.
    ///
    /// The shields themselves are only decremented when [`Event::DamagePrevented`] is applied.
    /// That's fine for a batch minted from one pre-apply snapshot as long as no two damage events
    /// in it share a target — the mass-damage sweeps deal to each creature or player once.
    ///
    /// ponytail: two *sources* hitting one shielded target in a single batch (several blockers
    /// dealing combat damage to the same shielded creature) would each see the full shield and
    /// double-spend it. Combat's own path applies each event as it pushes it (`push_apply`), so
    /// the case that actually arises today is already correct; the general fix is a spend ledger
    /// threaded through the mint, which no pool card yet needs.
    fn spend_prevention_shields(&self, target: Target, amount: i32) -> (Vec<Event>, i32) {
        let available: i32 = self
            .damage_prevention_shields
            .iter()
            .filter(|&&(shielded, _)| shielded == target)
            .map(|&(_, points)| points)
            .sum();
        let prevented = available.min(amount);
        if prevented <= 0 {
            return (Vec::new(), amount);
        }
        (
            vec![Event::DamagePrevented {
                target,
                amount: prevented,
            }],
            amount - prevented,
        )
    }

    /// [`creature_damage_events`](Self::creature_damage_events) plus Disintegrate's two riders on
    /// the damaged creature — "it can't be regenerated this turn, and if it would die this turn,
    /// exile it instead." Only [`Effect::Damage(DamageEffect::Target)`] carries them; every other
    /// damage path takes the three-argument form above.
    ///
    /// ponytail: the riders travel on the damage event, so a hit that never marks damage never
    /// marks the creature either — an infect Disintegrate (counters instead of damage), or one
    /// whose damage is prevented outright. CR reads "if it's a creature" off the *target*, not off
    /// the damage actually landing. No pool card creates either case; the upgrade path is a
    /// separate marking event emitted alongside the guards in the `Target` arm.
    pub(crate) fn creature_damage_events_with_riders(
        &self,
        source: ObjectId,
        object: ObjectId,
        amount: i32,
        cant_be_regenerated: bool,
        exile_instead_of_dying: bool,
    ) -> (Vec<Event>, i32) {
        let (mut events, amount) = self.spend_prevention_shields(Target::Object(object), amount);
        // CR 615: damage a shield ate entirely was never dealt, so it marks nothing and feeds no
        // damage watch. (An *unshielded* 0 still emits its `DamageMarked`, as it always has —
        // callers guard 0 amounts themselves where it matters.)
        if !events.is_empty() && amount <= 0 {
            return (events, 0);
        }
        if !self.has_keyword(source, Keyword::Infect) {
            events.push(Event::DamageMarked {
                object,
                amount,
                source: Some(source),
                cant_be_regenerated,
                exile_instead_of_dying,
            });
            return (events, amount);
        }
        // 0 or less damage is never dealt (CR 120.8) — and never placed as counters.
        if amount <= 0 {
            return (events, amount);
        }
        events.push(Event::KindCountersPlaced {
            object,
            kind: CounterKind::MinusOneMinusOne,
            count: amount,
        });
        (events, amount)
    }

    /// The event that lands `amount` damage from `source` on `player` — the player twin of
    /// [`creature_damage_events`](Self::creature_damage_events). An infect source's damage is dealt
    /// in the form of that many poison counters (CR 702.90c) instead of as life loss.
    ///
    /// Only the life-loss event itself is swapped: the caller's [`Event::DamageDealtToPlayer`] /
    /// [`Event::CombatDamageDealtToPlayer`] marker, commander-damage tally and lifelink gain all
    /// still fire off the original amount, because infect changes the form of the damage, not the
    /// fact that it was dealt (CR 120.3).
    ///
    /// Returns `(the events, the damage actually dealt)`, the second half reduced by this player's
    /// [`spend_prevention_shields`](Self::spend_prevention_shields) bite — see the creature twin.
    pub(crate) fn player_damage_events(
        &self,
        source: ObjectId,
        player: PlayerId,
        amount: i32,
    ) -> (Vec<Event>, i32) {
        let (mut events, amount) = self.spend_prevention_shields(Target::Player(player), amount);
        // CR 615: damage a shield ate entirely was never dealt — no life loss, and the caller's
        // `amount > 0` guard drops the `DamageDealtToPlayer` marker and lifelink with it.
        if !events.is_empty() && amount <= 0 {
            return (events, 0);
        }
        if !self.has_keyword(source, Keyword::Infect) {
            events.push(Event::LifeChanged {
                player,
                amount: -amount,
                source: Some(source),
            });
            return (events, amount);
        }
        // 0 or less damage is never dealt (CR 120.8) — and never placed as counters.
        if amount <= 0 {
            return (events, amount);
        }
        events.push(Event::PlayerCountersPlaced {
            player,
            kind: PlayerCounterKind::Poison,
            count: amount,
        });
        (events, amount)
    }

    /// The events for one targeted damage effect landing on `chosen`, plus the damage actually
    /// dealt (0 when a shield, protection, or a prevention effect ate it) — the second half is
    /// what Drain Life's "equal to the damage dealt" reads.
    fn single_target_damage_events(
        &self,
        source: ObjectId,
        chosen: Target,
        amount: i32,
        cant_be_regenerated: bool,
        exile_instead_of_dying: bool,
    ) -> (Vec<Event>, i32) {
        match chosen {
            // Damage to a creature is marked (an SBA later checks it against toughness), (CR 704, CR 120.3)
            // unless protection from the source's color prevents it (CR 702.16d).
            Target::Object(object) => {
                if self.damage_prevented_by_protection(object, Some(source)) {
                    return (Vec::new(), 0);
                }
                // Phantom Centaur's self-shield (or Bloatfly Swarm's scaling variant)
                // prevents this damage outright and removes +1/+1 counters instead (CR 615).
                if self.phantom_shield_active(object) {
                    return (self.phantom_shield_counter_removal(object, amount), 0);
                }
                // Damage to a planeswalker removes that many loyalty counters instead of
                // being marked (CR 120.3c/306.9) — checked ahead of Tajic's creature-only
                // prevention below, since a planeswalker is never "another creature".
                if matches!(self.def_of(object).kind, CardKind::Planeswalker { .. }) {
                    return (
                        vec![Event::LoyaltyChanged {
                            object,
                            amount: -amount,
                        }],
                        amount,
                    );
                }
                // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                if self.noncombat_damage_prevented_to_creature(object) {
                    return (Vec::new(), 0);
                }
                self.creature_damage_events_with_riders(
                    source,
                    object,
                    amount,
                    cant_be_regenerated,
                    exile_instead_of_dying,
                )
            }
            // Damage to a player is life loss. ponytail: the commander-damage tally is
            // combat-only (CR 903.10a), so a burn spell never adds to it.
            Target::Player(player) => {
                let (mut events, amount) = self.player_damage_events(source, player, amount);
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
                (events, amount)
            }
        }
    }

    /// Drain Life's "You gain life equal to the damage dealt, but not more life than the player's
    /// life total before the damage was dealt, the planeswalker's loyalty before the damage was
    /// dealt, or the creature's toughness." The cap is the target's own capacity to absorb damage,
    /// read before any of it lands (nothing in this module has mutated the board yet).
    fn drain_gain(
        &self,
        controller: PlayerId,
        source: ObjectId,
        chosen: Target,
        dealt: i32,
    ) -> Option<Event> {
        let capacity = match chosen {
            Target::Player(player) => self.life(player),
            Target::Object(object)
                if matches!(self.def_of(object).kind, CardKind::Planeswalker { .. }) =>
            {
                self.loyalty(object)
            }
            Target::Object(object) => self.toughness(object),
        };
        let gain = dealt.min(capacity);
        if gain <= 0 {
            return None;
        }
        Some(Event::LifeChanged {
            player: controller,
            amount: self.life_gain_after_replacements(controller, gain),
            source: Some(source),
        })
    }

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
                amount,
                divided,
                cant_be_regenerated,
                exile_instead_of_dying,
                gain_life_equal_to_damage,
                ..
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
                let (mut events, dealt) = self.single_target_damage_events(
                    source,
                    chosen,
                    amount,
                    cant_be_regenerated,
                    exile_instead_of_dying,
                );
                if gain_life_equal_to_damage {
                    events.extend(self.drain_gain(controller, source, chosen, dealt));
                }
                events
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
                    // Sulfurous Blast's "If you cast this spell during your main phase... instead"
                    // reads the resolving *spell's* own cast-timing flag, not any one creature's —
                    // same "pick it once against the true source" reasoning as `IfSpellKicked`
                    // above.
                    Amount::IfSpellCastDuringMainPhase { then, else_ } => {
                        if self.spell_cast_during_main_phase(source) {
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
                        let object_amount =
                            self.resolve_amount(amount, controller, object, target, x);
                        // Phantom Centaur's self-shield (or Bloatfly Swarm's scaling variant)
                        // prevents its own share and removes +1/+1 counters instead (CR 615) — a
                        // shielded creature swaps its `DamageMarked` for that counter removal
                        // rather than being filtered out outright.
                        if self.phantom_shield_active(object) {
                            return self.phantom_shield_counter_removal(object, object_amount);
                        }
                        self.creature_damage_events(source, object, object_amount).0
                    })
                    .collect()
            }
            // The old "Radiance" keyword action (Cleansing Beam): "deals `amount` damage to
            // target creature and each other creature that shares a color with it" (CR 105.2).
            // `target` is the one real target — already re-checked by `target_still_legal` for
            // legality/protection/hexproof before this runs — and expands into
            // `Game::radiance_batch`; `amount` doesn't vary per creature (no pool Radiance card
            // reads `Amount::SourcePower`), so it's resolved once, like `DamageEffect::Target`.
            // Each swept creature still gets its own protection/Phantom Centaur/Tajic check,
            // same as `EachCreature`'s per-creature checks above — only the chosen target was a
            // real target, the rest of the batch is untargeted.
            DamageEffect::Radiance { amount, .. } => {
                let chosen = expect_object_target(target, "a radiance target");
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.radiance_batch(chosen)
                    .into_iter()
                    .filter(|&id| !self.damage_prevented_by_protection(id, Some(source)))
                    .filter(|&id| !self.noncombat_damage_prevented_to_creature(id))
                    .flat_map(|object| {
                        // Phantom Centaur's self-shield prevents its own share and removes one
                        // of its own +1/+1 counters instead (CR 615).
                        if self.phantom_shield_active(object) {
                            return self.phantom_shield_counter_removal(object, amount);
                        }
                        vec![Event::DamageMarked {
                            object,
                            amount,
                            source: Some(source),
                            cant_be_regenerated: false,
                            exile_instead_of_dying: false,
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
                        let (mut events, amount) =
                            self.player_damage_events(source, player, amount);
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
            // Advanced Reconstruction / Fateful Tempest: same per-player damage events as
            // `EachPlayer` above, but only to living opponents (CR 102.3) — the controller is
            // carved out.
            DamageEffect::EachOpponent { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.living_players()
                    .filter(|&player| player != controller)
                    .flat_map(|player| {
                        let (mut events, amount) =
                            self.player_damage_events(source, player, amount);
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
                        let (mut events, amount) =
                            self.player_damage_events(source, player, amount);
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
                // Phantom Centaur's self-shield (or Bloatfly Swarm's scaling variant) prevents
                // this damage outright and removes +1/+1 counters instead (CR 615).
                if self.phantom_shield_active(object) {
                    return self.phantom_shield_counter_removal(object, amount);
                }
                // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                if self.noncombat_damage_prevented_to_creature(object) {
                    return Vec::new();
                }
                self.creature_damage_events(source, object, amount).0
            }

            // Ankh of Mishra: 2 damage to the controller of the land that just entered — the
            // player twin of `ToEnteringPermanent` above, off the same context slot. `controller_of`
            // (not `owner_of`) is the printed word, so a land under a Confiscate bills the thief.
            DamageEffect::ToEnteringPermanentController { entering, amount } => {
                let object = entering.expect("the entering permanent is filled in at placement");
                let recipient = self.controller_of(object);
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let (mut events, amount) = self.player_damage_events(source, recipient, amount);
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

            // Copper Tablet: 1 damage to the player whose upkeep this is, baked in at trigger
            // placement off `TriggerContext::active_player` — same shape as the arm above, with
            // the recipient arriving as a player rather than as a permanent to ask.
            DamageEffect::ToTriggeringPlayer { player, amount } => {
                let recipient = player.expect("the triggering player is filled in at placement");
                // Karma's "damage to that player equal to the number of Swamps *they* control" and
                // Power Surge's "lands *they* controlled": a player-relative amount on this effect
                // reads the recipient, not the source's controller.
                // ponytail: no `who` axis on `Amount` — every "deals damage to that player equal
                // to …" the pool prints counts that same player's things. Add one if a card ever
                // bills the triggering player for something *you* control.
                let amount = self.resolve_amount(amount, recipient, source, target, x);
                let (mut events, amount) = self.player_damage_events(source, recipient, amount);
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

            // Creature Bond: damage to whoever controlled the Aura's host when it died, both the
            // recipient and (via `Amount::DyingEnchantedCreatureToughness`) the amount baked in at
            // trigger placement — same shape as the arm above.
            DamageEffect::ToDyingEnchantedCreaturesController { player, amount } => {
                let recipient =
                    player.expect("the dying host's controller is filled in at placement");
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let (mut events, amount) = self.player_damage_events(source, recipient, amount);
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

            // Real damage to the ability's own controller — mirrors `DealDamage`'s
            // `Target::Player` arm, substituting `controller` for the chosen target.
            DamageEffect::ToSelf { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let (mut events, amount) = self.player_damage_events(source, controller, amount);
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
                let (mut events, amount) = self.player_damage_events(source, recipient, amount);
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
        // Either form of dealt damage counts (CR 119.3): an infect source's hit lands as -1/-1
        // counters rather than marked damage (CR 702.90b), and still satisfies "is dealt damage".
        let damage_landed = evs.iter().any(|e| {
            matches!(
                e,
                Event::DamageMarked { .. }
                    | Event::KindCountersPlaced {
                        kind: CounterKind::MinusOneMinusOne,
                        ..
                    }
            )
        });
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
