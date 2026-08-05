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
    fn spend_prevention_shields(
        &self,
        target: Target,
        source: ObjectId,
        amount: i32,
        allow_redirect: bool,
    ) -> (Vec<Event>, i32) {
        let mut prevented = 0;
        let mut life_gained = 0;
        let mut redirects: Vec<(Target, i32)> = Vec::new();
        for shield in self.shields_against(target, source) {
            if prevented >= amount {
                break;
            }
            // A redirection is a replacement, not a prevention (CR 615.10) — it still spends the
            // shield, but the damage goes on to be dealt somewhere else. Damage that arrived here
            // by an earlier redirect passes them by rather than bouncing again (CR 616.1).
            if shield.redirect_to.is_some() && !allow_redirect {
                continue;
            }
            // "Prevent all but 1 of that damage" (Forcefield) subtracts from the other end:
            // everything still coming *except* the points it lets through. "Prevent that damage"
            // (no point total) eats everything still coming; a point shield eats what it has left.
            let bite = match (shield.keep, shield.amount) {
                (Some(keep), _) => (amount - prevented - keep).max(0),
                (None, None) => amount - prevented,
                (None, Some(points)) => points.min(amount - prevented),
            };
            prevented += bite;
            if shield.gain_life {
                life_gained += bite;
            }
            if let Some(to) = shield.redirect_to {
                redirects.push((to, bite));
            }
        }
        if prevented <= 0 {
            return (Vec::new(), amount);
        }
        // ponytail: a redirected bite is recorded as `DamagePrevented` at the original target
        // plus ordinary damage at the new one, so the log reads "prevented … and then dealt"
        // where the card reads "dealt … instead". The board lands in the right place either way;
        // upgrade path is an `Event::DamageRedirected { from, to, .. }` carrying both ends.
        let mut events = vec![Event::DamagePrevented {
            target,
            amount: prevented,
            source,
        }];
        // Reverse Damage's "you gain life equal to the damage prevented this way" — paid to the
        // shielded player, who is the only thing any pool card with this rider protects.
        if life_gained > 0
            && let Target::Player(player) = target
        {
            events.push(Event::LifeChanged {
                player,
                amount: life_gained,
                source: Some(source),
            });
        }
        for (to, moved) in redirects {
            events.extend(self.redirected_damage_events(source, to, moved));
        }
        (events, amount - prevented)
    }

    /// "That source deals that damage to `to` instead" (Jade Monolith, CR 615.10). The moved
    /// damage is dealt for real at its new home — the recipient's own shields still get their
    /// bite — but it cannot be moved a second time.
    fn redirected_damage_events(&self, source: ObjectId, to: Target, amount: i32) -> Vec<Event> {
        match to {
            Target::Player(player) => {
                // The moved damage is dealt to this player for real (CR 615.10), so it carries the
                // same "damage was dealt to a player" marker an ordinary hit does — Pit Scorpion's
                // watch and the turn's damage ledger both read it. `player_damage_events_inner`
                // mints only the life loss; every other caller pushes the marker itself.
                let (mut events, dealt) =
                    self.player_damage_events_inner(source, player, amount, false);
                if dealt > 0 {
                    events.push(Event::DamageDealtToPlayer {
                        source,
                        player,
                        amount: dealt,
                    });
                }
                events
            }
            Target::Object(object) => {
                self.creature_damage_events_inner(source, object, amount, false, false, false)
                    .0
            }
        }
    }

    /// The shields standing between `source` and `target`, oldest first — the one order both the
    /// mint above and [`Game::apply`]'s [`Event::DamagePrevented`] arm walk, so the two never
    /// disagree about which shields paid.
    pub(crate) fn shields_against(
        &self,
        target: Target,
        source: ObjectId,
    ) -> impl Iterator<Item = &crate::state::PreventionShield> {
        self.damage_prevention_shields
            .iter()
            .filter(move |shield| self.shield_stands_between(shield, target, source))
    }

    /// Whether `shield` stands between `source` and `target` at this moment — the one predicate
    /// the mint above and [`Game::apply`]'s [`Event::DamagePrevented`] arm both ask, so the two
    /// can't drift about which shields paid.
    pub(crate) fn shield_stands_between(
        &self,
        shield: &crate::state::PreventionShield,
        target: Target,
        source: ObjectId,
    ) -> bool {
        // A source-keyed shield (Lady Evangela's "dealt **by** target creature this turn") names
        // no recipient at all, so it stands in front of whoever the named source is hitting.
        if !shield.any_recipient && shield.target != target {
            return false;
        }
        // A named source (Forcefield's chosen creature) replaces the colour gate rather than
        // joining it — the card names the one thing it stops, not a class of them.
        if let Some(named) = shield.from_source {
            return named == source && (!shield.combat_only || self.in_combat_damage_step());
        }
        if shield.combat_only && !self.in_combat_damage_step() {
            return false;
        }
        // "… by attacking creatures without flying" (Al-abara's Carpet) — the turn-scoped twin of
        // a `StaticEffect::PreventDamage` source filter, read from the shielded side's own
        // perspective. A non-permanent source (a spell) never matches one.
        if let Some(filter) = shield.from_filter {
            let you = match target {
                Target::Player(player) => player,
                Target::Object(object) => self.controller_of(object),
            };
            if !self.permanent_matches(&filter, source, you, target.object_id()) {
                return false;
            }
        }
        // "… a spell or ability that targets that creature" (Silhouette) — only a shield standing
        // in front of a *permanent* can read a relationship to the source.
        if let Some(relation) = shield.from_relation {
            let Target::Object(object) = target else {
                return false;
            };
            if !self.source_relates(relation, source, object) {
                return false;
            }
        }
        self.color_matches(shield.from_color, source)
    }

    /// Whether the damage's `source` stands in the named relationship to the shielded permanent
    /// `object` (CR 615) — the two "by …" clauses that read the shielded object rather than the
    /// source's own characteristics. Shared by the permanent statics (Wall of Vapor, Bronze Horse)
    /// and the turn-scoped shields (Silhouette), so both answer the same question the same way.
    pub(crate) fn source_relates(
        &self,
        relation: SourceRelation,
        source: ObjectId,
        object: ObjectId,
    ) -> bool {
        match relation {
            // "creatures it's blocking" — per damage source, not per combat: a creature blocking
            // two attackers (CR 509.1) shields against each of them independently.
            SourceRelation::BlockedByThis => self.combat.blocks.contains(&(object, source)),
            // "spells that target it" — the source is still a spell on the stack while it
            // resolves, so its chosen targets are readable here (CR 608.2). Both target clauses
            // count: the shield asks whether the spell targets the creature at all.
            SourceRelation::SpellTargetingThis => {
                let Object::Spell(spell) = &self.objects[source as usize] else {
                    return false;
                };
                let targeted =
                    |list: &crate::TargetList| list.iter().any(|t| t == Target::Object(object));
                targeted(&spell.targets) || targeted(&spell.targets_second)
            }
            // "a spell or ability that targets that creature would *cause a source* to deal
            // damage to it" — the question is about the item that caused the damage, not about
            // the source it named, so it is answered from the resolution frame rather than from
            // `source`'s own characteristics. `resolving_targets` is armed only while a stack
            // item's effects are running, so damage minted outside a resolution (combat) reads
            // an empty list and is never caught.
            SourceRelation::SpellOrAbilityTargetingThis => self
                .resolution_frame
                .resolving_targets
                .contains(&Target::Object(object)),
        }
    }

    /// Whether a permanent's own [`StaticEffect::PreventDamage`] shield prevents this whole hit
    /// (CR 615) — Guard Gomazoa's "prevent all combat damage that would be dealt to" and the
    /// Legends family that narrows it with a source gate. Read at
    /// [`creature_damage_events_inner`](Self::creature_damage_events_inner), the one choke every
    /// creature-damage path routes through, so combat, fight, burn and sweeps are all covered.
    pub(crate) fn static_damage_prevented(&self, target: ObjectId, source: ObjectId) -> bool {
        self.replacement_registry().damage_prevented_to_permanent(
            self,
            target,
            source,
            self.in_combat_damage_step(),
        )
    }

    /// Whether damage being dealt right now is combat damage — read off the step rather than
    /// carried on the event, since combat damage is the only damage either combat damage step
    /// deals (CR 510.2).
    fn in_combat_damage_step(&self) -> bool {
        matches!(
            self.current_step(),
            crate::Step::FirstStrikeCombatDamage | crate::Step::CombatDamage
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
        self.creature_damage_events_inner(
            source,
            object,
            amount,
            cant_be_regenerated,
            exile_instead_of_dying,
            true,
        )
    }

    /// [`creature_damage_events_with_riders`](Self::creature_damage_events_with_riders) plus the
    /// one-bounce guard: `allow_redirect` is false for damage that a redirection already moved
    /// here, so it can be prevented but not moved on (CR 616.1).
    fn creature_damage_events_inner(
        &self,
        source: ObjectId,
        object: ObjectId,
        amount: i32,
        cant_be_regenerated: bool,
        exile_instead_of_dying: bool,
        allow_redirect: bool,
    ) -> (Vec<Event>, i32) {
        // Guard Gomazoa / Wall of Vapor (CR 615): the shielded permanent's own "prevent all
        // damage that would be dealt to this creature by …" static eats the whole hit before any
        // spendable shield pays for it. Silent, like the other whole-event shields — nothing in
        // the pool reads a prevented total on the creature side.
        if self.static_damage_prevented(object, source) {
            return (Vec::new(), 0);
        }
        // Rock Hydra's per-point shield (CR 615) is spent first and only covers as many points as
        // it has counters; whatever it can't pay for falls through to the ordinary shields below
        // and is dealt for real.
        let (mut events, amount) = self.per_point_counter_shield(object, amount);
        let (shield_events, amount) =
            self.spend_prevention_shields(Target::Object(object), source, amount, allow_redirect);
        events.extend(shield_events);
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
        self.player_damage_events_inner(source, player, amount, true)
    }

    /// [`player_damage_events`](Self::player_damage_events) plus the one-bounce guard — see the
    /// creature twin.
    fn player_damage_events_inner(
        &self,
        source: ObjectId,
        player: PlayerId,
        amount: i32,
        allow_redirect: bool,
    ) -> (Vec<Event>, i32) {
        let (mut events, amount) =
            self.spend_prevention_shields(Target::Player(player), source, amount, allow_redirect);
        // Forethought Amulet's "it deals 2 damage to you instead" (CR 615.9) — a replacement that
        // rewrites the amount rather than subtracting from it, so it is read *after* the shields
        // have taken their bite. CR 615.9 lets the affected player order the two; with one
        // rewrite in the pool and prevention only ever shrinking the hit, the order is unobservable
        // except when a shield drops a hit below the rewrite's threshold, which is the reading
        // that leaves the player better off.
        let amount = self
            .replacement_registry()
            .replaced_damage_to_player(self, player, source, amount);
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
                let amount = if divided != Division::None {
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
                // An `Amount::IfCondition` here reads the resolving *spell's* own record — its
                // kicked flag (CR 702.33d, Breath of Darigaaz) or its cast timing (Sulfurous
                // Blast) — not any one creature's, so the branch is picked once here against the
                // ability's true `source`, before the per-creature substitution below stands a
                // creature in for `source` (needed for `Amount::SourcePower`, Wave of Reckoning's
                // "equal to its power") and the condition would silently read false. The chosen
                // arm still resolves per creature, so `Fixed` and `SourcePower` inside it behave
                // as they would anywhere else.
                let amount = match amount {
                    Amount::IfCondition {
                        condition,
                        then,
                        else_,
                    } => {
                        let ctx = TriggerContext::of(self.controller_of(source));
                        match self.ability_condition_holds(condition, source, ctx) {
                            true => *then,
                            false => *else_,
                        }
                    }
                    // Disaster Radius's "X is the revealed card's mana value" (CR 601.2g) reads
                    // the resolving *spell's* own reveal-cost record, not any one creature's —
                    // same "pick it once against the true source" reasoning as the branch above.
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
            // Every damage the pool aims at seats: Psionic Blast's "2 damage to you", Pestilence's
            // "each player", Ankh of Mishra's "that land's controller", Hydra Omnivore's "each
            // other opponent" — one fan-out, with `who` naming the seats. Mirrors `DealDamage`'s
            // `Target::Player` arm (life loss + `DamageDealtToPlayer` + lifelink) once per
            // recipient.
            DamageEffect::ToPlayers { who, amount } => self
                .players_in(who, controller, target)
                .into_iter()
                .flat_map(|player| {
                    // A player-relative amount counts the seat being damaged: Karma's "Swamps
                    // *they* control", Black Vise's "cards in *their* hand", Power Surge's "lands
                    // *they* controlled". Resolved per recipient, so a fan-out bills each seat for
                    // its own things.
                    let amount = self.resolve_amount(amount, player, source, target, x);
                    let (mut events, amount) = self.player_damage_events(source, player, amount);
                    // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                    if amount > 0 {
                        events.push(Event::DamageDealtToPlayer {
                            source,
                            player,
                            amount,
                        });
                        // Lifelink (CR 702.15e): a source dealing damage to multiple players gains
                        // life separately for each.
                        events.extend(self.lifelink_gain(source, amount));
                    }
                    events
                })
                .collect(),
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
