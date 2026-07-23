//! Combat declaration, damage, and combat keywords.
//!
//! Primary: CR 506–511 (combat phases/steps), CR 702 (evergreen combat keywords).
//! Also: CR 701.38 (goad), CR 508.1g (attack costs / pillow-fort).
//! Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill); layers (CR 613) per engine-core-and-event-model spec.

use crate::*;

impl Game {
    /// Whether `blocker` may legally block `attacker` on behalf of the attacked player
    /// `player` (CR 509.1a): an untapped creature of `player`'s, against a declared attacker
    /// that's attacking `player`; a flyer can only be blocked by a flyer/reacher (CR 702.9c);
    /// protection stops blockers of the protected-from color (CR 702.16c). Menace is a rule
    /// about the *whole* declaration (two or more blockers) and lives in
    /// [`Game::declare_blockers`]. Shared by that validation and by
    /// [`Game::meaningful_actions`], so they can't disagree.
    pub(crate) fn can_block(
        &self,
        player: PlayerId,
        blocker: ObjectId,
        attacker: ObjectId,
    ) -> bool {
        let Some(b) = self.as_permanent(blocker) else {
            return false;
        };
        // A phased-out creature can't block (CR 702.26e — treated as though it doesn't exist).
        if b.phased_out {
            return false;
        }
        // CR 509.1a: blockers are the defending *controller*'s creatures — a creature you've
        // stolen blocks for you, not for its owner. Creature-ness via the CR 613.4 type layer
        // (an animated manland can block too).
        if !self.is_creature_on_battlefield(blocker)
            || self.controller_of(blocker) != player
            || b.tapped
        {
            return false;
        }
        // "This creature can't block" (CR 509.1a — Bloodghast): never a legal blocker.
        if self.has_keyword(blocker, Keyword::CantBlock) {
            return false;
        }
        // "Enchanted permanent/creature can't … block" (Faith's Fetters, Prison Term): a live
        // attached Aura's continuous `cant_block` grant.
        if self.host_cant_block(blocker) {
            return false;
        }
        // Decayed (CR 702.148b): "A creature with decayed can't block."
        if self.has_keyword(blocker, Keyword::Decayed) {
            return false;
        }
        // Brazen Borrower's "can block only creatures with flying": legal only against a flyer.
        if self.has_keyword(blocker, Keyword::CanBlockOnlyFlyers)
            && !self.has_keyword(attacker, Keyword::Flying)
        {
            return false;
        }
        // A blocker can only block a declared attacker that's attacking its controller — or one of
        // that controller's planeswalkers (CR 509.1a: the defending player declares the blocks).
        if !self.combat.attackers.contains(&attacker)
            || self.defending_player_of(attacker) != Some(player)
        {
            return false;
        }
        if self.has_keyword(attacker, Keyword::Flying) && !self.can_block_flyers(blocker) {
            return false;
        }
        // Unblockable (Rogue's Passage): no creature may block it at all (CR 702.10b).
        if self.has_keyword(attacker, Keyword::Unblockable) {
            return false;
        }
        // Skulk (CR 702.72a): can't be blocked by creatures with greater power.
        if self.has_keyword(attacker, Keyword::Skulk) && self.power(blocker) > self.power(attacker)
        {
            return false;
        }
        // ponytail: Elusive Otter's printed evasion static, riding the shared block-legality
        // check as a card-specific keyword tag rather than new DSL surface for one card.
        if self.has_keyword(attacker, Keyword::LesserPowerCantBlock)
            && self.power(blocker) < self.power(attacker)
        {
            return false;
        }
        // Shadow (CR 702.28b/c): a Shadow creature can only block/be blocked by other Shadow
        // creatures — the restriction runs both directions.
        if self.has_keyword(attacker, Keyword::Shadow) != self.has_keyword(blocker, Keyword::Shadow)
        {
            return false;
        }
        // Fear (CR 702.36b): can be blocked only by artifact creatures and/or black creatures.
        if self.has_keyword(attacker, Keyword::Fear)
            && !self.effective_types(blocker).intersects(TypeSet::ARTIFACT)
            && !self.colors_of(blocker)[Color::Black.index()]
        {
            return false;
        }
        !self.protection_blocks_source(attacker, blocker)
    }

    /// What a declared attacker is attacking (CR 508.1a), or `None` if the defending player has
    /// since been eliminated (CR 800.4a drops the target pair but leaves the attacker in combat).
    pub(crate) fn defender_of(&self, attacker: ObjectId) -> Option<Defender> {
        self.combat
            .attack_targets
            .iter()
            .find(|(a, _)| *a == attacker)
            .map(|(_, d)| *d)
    }

    /// The player defending against `defender` — itself for a player, the controller for a
    /// planeswalker (CR 508.1a). `None` once the planeswalker has left the battlefield, which is
    /// exactly when its attacker assigns no combat damage (CR 510.1a).
    pub fn defender_controller(&self, defender: Defender) -> Option<PlayerId> {
        match defender {
            Defender::Player(player) => Some(player),
            Defender::Planeswalker(pw) => self.as_permanent(pw).map(|_| self.controller_of(pw)),
        }
    }

    /// The defending *player* a declared attacker is attacking — the single choke every rule
    /// scoped to "the player being attacked" (block legality, attack taxes, the Vow cycle's "you
    /// or planeswalkers you control", attack triggers) reads, so a planeswalker defender behaves
    /// like its controller without each rule knowing about planeswalkers.
    pub(crate) fn defending_player_of(&self, attacker: ObjectId) -> Option<PlayerId> {
        self.defender_of(attacker)
            .and_then(|d| self.defender_controller(d))
    }

    /// Whether `player` is being attacked by at least one creature this combat — directly, or
    /// through a planeswalker they control (CR 509.1a: either way they declare the blocks).
    pub(crate) fn is_attacked_player(&self, player: PlayerId) -> bool {
        self.combat
            .attack_targets
            .iter()
            .any(|&(_, d)| self.defender_controller(d) == Some(player))
    }

    /// The players who have goaded `creature` (CR 701.38); empty if it isn't goaded.
    // ponytail: unions the event-based one-shot goad (`goaded` vec, cleared at the goader's
    // next turn, CR 701.38b) with the continuous goad-on-attached static (Impetus cycle,
    // Redemption Arc) — the latter is recomputed live off the attachment scan, so it needs no
    // entry here and just stops applying the instant the Aura leaves.
    pub fn goaders_of(&self, creature: ObjectId) -> Vec<PlayerId> {
        self.combat_extras
            .goaded
            .iter()
            .filter(|&&(o, _, _)| o == creature)
            .map(|&(_, by, _)| by)
            .chain(self.goaded_by_attachment(creature))
            .collect()
    }

    /// Whether `player` has an active impulse-draw permission to play the exiled card `object`
    /// (CR 118.6). The permission gates casting/playing from exile; timing is checked separately.
    pub(crate) fn may_play_from_exile(&self, object: ObjectId, player: PlayerId) -> bool {
        self.play_permissions
            .play_from_exile
            .iter()
            .any(|&(card, p, _)| card == object && p == player)
            // An adventure card exiled "on an adventure" is castable from exile at normal cost
            // (CR 715.3d) — an open-ended permission with the same "cast from exile" gate.
            || self
                .play_permissions
                .on_adventure
                .iter()
                .any(|&(card, p)| card == object && p == player)
            || self.may_play_from_exile_free_while_source(object, player)
    }

    /// Whether `player` may play the exiled card `object` **without paying its mana cost** because
    /// the permanent that granted the permission is still on the battlefield (Intet, the Dreamer —
    /// CR 118.5). Deliberately *not* folded into
    /// [`Game::may_cast_from_exile_free`](Self::may_cast_from_exile_free): that permission also
    /// waives timing (CR 601.3e — it arrives mid-resolution), while Intet's grant is an ordinary
    /// "you may play that card" that keeps normal timing.
    pub(crate) fn may_play_from_exile_free_while_source(
        &self,
        object: ObjectId,
        player: PlayerId,
    ) -> bool {
        self.play_permissions
            .play_from_exile_free_while_source
            .iter()
            .any(|&(card, p, source)| {
                card == object && p == player && self.as_permanent(source).is_some()
            })
    }

    /// Whether `player` has an active free-cast permission (CR 118.5, "without paying its mana
    /// cost") for the exiled card `object` — Quintorius, Loremaster's activated ability.
    pub(crate) fn may_cast_from_exile_free(&self, object: ObjectId, player: PlayerId) -> bool {
        self.play_permissions
            .cast_from_exile_free
            .iter()
            .any(|&(card, p)| card == object && p == player)
    }

    /// Whether `creature` could legally be declared as an attacker this combat, ignoring goad:
    /// an untapped, non-Defender creature that isn't summoning-sick without haste and has at
    /// least one legal defending player. Drives the "attacks if able" half of goad.
    pub(crate) fn can_attack(&self, creature: ObjectId) -> bool {
        let Some(p) = self.as_permanent(creature) else {
            return false;
        };
        // A phased-out creature can't attack (CR 702.26e — treated as though it doesn't exist).
        // Creature-ness via the CR 613.4 type layer (`is_creature_on_battlefield`), so an animated
        // manland (Restless Spire) can attack.
        self.is_creature_on_battlefield(creature)
            && !p.tapped
            && !self.is_sick_without_haste(creature)
            && !self.has_keyword(creature, Keyword::Defender)
            // "Enchanted permanent/creature can't attack" (Faith's Fetters, Prison Term): the
            // reverse of goad's "must attack", a live attached Aura's continuous `cant_attack`
            // grant.
            && !self.host_cant_attack(creature)
            && self
                .living_players()
                .any(|d| d != self.controller_of(creature))
    }

    /// Test/setup helper: goad `creature` on behalf of `by` (routed through an event so state
    /// stays mutated only by [`Game::apply`]).
    pub fn goad(&mut self, creature: ObjectId, by: PlayerId) {
        self.apply(&Event::Goaded {
            object: creature,
            by,
            source_name: "Goad",
        });
    }

    /// Whether `creature` is currently goaded by anyone (CR 701.38): a one-shot `GoadTarget`
    /// still in effect, or a continuous goad-on-attached static Aura (see
    /// [`Game::goaded_by_attachment`]).
    pub fn is_goaded(&self, creature: ObjectId) -> bool {
        self.combat_extras
            .goaded
            .iter()
            .any(|&(o, _, _)| o == creature)
            || self.goaded_by_attachment(creature).next().is_some()
    }

    /// Creatures `player` controls that must attack this combat if able (CR 701.38a goad), each
    /// paired with a legal defender that satisfies the "attack a non-goader if able" rule.
    /// Used by the declare-attackers action projection so the client can seed staging instead of
    /// offering an empty confirm the engine would reject.
    pub fn required_attacks(&self, player: PlayerId) -> Vec<(ObjectId, Defender)> {
        let mut out = Vec::new();
        for id in self.controlled_battlefield(player) {
            let goaders = self.goaders_of(id);
            if goaders.is_empty() || !self.can_attack(id) {
                continue;
            }
            let Some(defender) = self
                .living_players()
                .filter(|&d| d != player)
                .find(|d| !goaders.contains(d))
                .or_else(|| self.living_players().find(|&d| d != player))
            else {
                continue;
            };
            out.push((id, Defender::Player(defender)));
        }
        for id in self.controlled_battlefield(player) {
            let Some(&(_, required)) = self
                .combat_extras
                .must_attack
                .iter()
                .find(|&&(o, _)| o == id)
            else {
                continue;
            };
            if !self.can_attack(id) {
                continue;
            }
            let required_legal = required != player
                && (required.0 as usize) < self.players.len()
                && !self.players[required.0 as usize].lost;
            if !required_legal {
                continue;
            }
            if out.iter().any(|&(a, _)| a == id) {
                continue;
            }
            out.push((id, Defender::Player(required)));
        }
        out
    }

    /// Test/setup helper: tap `object` (routed through an event so state stays mutated only
    /// by [`Game::apply`]). A masked Illusionary Mask creature (CR 615) is turned face up first.
    pub fn tap(&mut self, object: ObjectId) {
        let mut events = Vec::new();
        self.flip_masked(object, &mut events);
        self.apply(&Event::Tapped { object });
    }

    /// Test/setup helper: untap `object` (the [`Self::tap`] twin) — for a permanent that entered
    /// tapped (`enters_tapped`) but a test needs to activate its `{T}` ability right away.
    pub fn untap(&mut self, object: ObjectId) {
        self.apply(&Event::Untapped { object });
    }

    /// Test/setup helper: remove one counter of `kind` from `object` (routed through an event).
    pub fn remove_counter(&mut self, object: ObjectId, kind: CounterKind) {
        self.apply(&Event::KindCountersPlaced {
            object,
            kind,
            count: -1,
        });
    }

    /// The total generic mana `declarer` must pay to declare `attackers` (CR 508.1g), summed across
    /// every defending player who controls a "pillow-fort" attack-tax static. A flat
    /// [`Effect::Static(StaticEffect::AttackTax)`] (Ghostly Prison) charges its `amount` per attacker aimed at that
    /// defender; a [`Effect::Static(StaticEffect::CounterScaledAttackTax)`] (Nils, Discipline Enforcer) charges each such
    /// attacker its own counter count (0 — untaxed — when it has none). Several taxers a defender
    /// controls stack (their amounts add, per the Ghostly Prison / Propaganda stacking ruling).
    /// Zero when no attacker faces a taxing defender. Takes the *resolved* defending player of
    /// each attack (see [`Game::defender_controller`]) — a planeswalker's controller taxes an
    /// attack on that planeswalker exactly as they tax an attack on themself.
    pub(crate) fn attack_tax_owed(&self, attackers: &[(ObjectId, PlayerId)]) -> u32 {
        attackers
            .iter()
            .map(|&(attacker, defender)| self.attacker_tax_owed(attacker, defender))
            .sum()
    }

    /// The generic mana one `attacker` aimed at `defender` owes across `defender`'s attack-tax
    /// statics. A counter-scaled taxer reads `attacker`'s own counter count — so a counterless
    /// attacker owes it nothing (CR 122.1: only creatures with counters are taxed).
    fn attacker_tax_owed(&self, attacker: ObjectId, defender: PlayerId) -> u32 {
        let counters = self.total_counters(attacker);
        // Battlefield only — `controller_of` panics on Removed (ceased tokens), so never walk the
        // full object arena here.
        self.controlled_battlefield(defender)
            .into_iter()
            .flat_map(|id| self.def_of(id).abilities)
            .map(|ability| match (ability.timing, ability.effect) {
                (Timing::Static, Effect::Static(StaticEffect::AttackTax { amount })) => {
                    amount as u32
                }
                (Timing::Static, Effect::Static(StaticEffect::CounterScaledAttackTax)) => counters,
                _ => 0,
            })
            .sum()
    }

    /// The player `defender` resolves to when `declarer` declares an attack on it, or `None` when
    /// it isn't a legal attack target (CR 508.1a): the defending player must be a living opponent,
    /// and a planeswalker defender must be an un-phased planeswalker on the battlefield that
    /// opponent controls.
    fn legal_defending_player(&self, declarer: PlayerId, defender: Defender) -> Option<PlayerId> {
        let player = match defender {
            Defender::Player(player) => player,
            Defender::Planeswalker(pw) => {
                // A phased-out permanent is treated as though it doesn't exist (CR 702.26e).
                let live = self.as_permanent(pw).is_some_and(|p| !p.phased_out)
                    && self.effective_types(pw).intersects(TypeSet::PLANESWALKER);
                if !live {
                    return None;
                }
                self.controller_of(pw)
            }
        };
        let legal = player != declarer
            && (player.0 as usize) < self.players.len()
            && !self.players[player.0 as usize].lost;
        legal.then_some(player)
    }

    /// The active player declares attackers during their declare-attackers step. Each must be
    /// an untapped, non-sick creature they control, attacking a living opponent or one of that
    /// opponent's planeswalkers (CR 508.1a); each taps unless it has vigilance.
    pub(crate) fn declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: &[(ObjectId, Defender)],
    ) -> Result<Vec<Event>, Reject> {
        if player != self.active_player
            || self.step != Step::DeclareAttackers
            || self.combat.attackers_declared
        {
            return Err(Reject::IllegalDeclaration);
        }
        // Every attack resolved to its defending player (CR 508.1a) — the form every restriction
        // below reads, so a planeswalker defender is checked as its controller throughout.
        let mut resolved: Vec<(ObjectId, PlayerId)> = Vec::with_capacity(attackers.len());
        for &(a, defender) in attackers {
            // `can_attack` first: it is safe on any object id, and once it holds `a` is a live
            // permanent, so `controller_of` can't panic on untrusted input.
            if !self.can_attack(a) || self.controller_of(a) != player {
                return Err(Reject::IllegalDeclaration);
            }
            let Some(defending_player) = self.legal_defending_player(player, defender) else {
                return Err(Reject::IllegalDeclaration);
            };
            resolved.push((a, defending_player));
        }

        // Attack-restriction statics (CR 509.1a — Combat Calligrapher, Eriette of the Charmed
        // Apple): a defender may control an [`Effect::Static(StaticEffect::CantBeAttackedBy)`] static that forbids
        // matching attackers from attacking them. Scanned per declared (attacker, defender) pair,
        // mirroring `attack_tax_owed`'s defender-permanent enumeration.
        for &(attacker, defender) in &resolved {
            // Battlefield only — `controller_of` panics on Removed (ceased tokens), so never walk
            // the full object arena here (mirrors `attack_tax_owed`).
            let restricted = self
                .controlled_battlefield(defender)
                .into_iter()
                .flat_map(|id| {
                    self.def_of(id)
                        .abilities
                        .iter()
                        .map(move |ability| (id, ability))
                })
                .any(|(source, ability)| match (ability.timing, ability.effect) {
                    (Timing::Static, Effect::Static(StaticEffect::CantBeAttackedBy { filter })) => {
                        self.permanent_matches(&filter, attacker, defender, Some(source))
                    }
                    _ => false,
                });
            if restricted {
                return Err(Reject::IllegalDeclaration);
            }
        }

        // Vow counters (CR 122.1 — Promise of Loyalty): a creature marked with a vow counter
        // "can't attack" the player recorded on it, for as long as it has the counter. Read live
        // off `kind_counters[Vow]` + `vow_protected`, so removing the counter lifts the restriction.
        // The printed "or planeswalkers you control" half falls out of `resolved`: an attack on a
        // planeswalker is recorded against that planeswalker's controller.
        for &(attacker, defender) in &resolved {
            let vowed = self.as_permanent(attacker).is_some_and(|p| {
                p.kind_counters[CounterKind::Vow as usize] > 0 && p.vow_protected == Some(defender)
            });
            if vowed {
                return Err(Reject::IllegalDeclaration);
            }
        }

        // Vow auras (Vow of Duty/Flight/Lightning — CR 122.1 sibling): a live attached Aura can
        // restrict the host from attacking *that Aura's own controller*, sourced from the
        // attachment rather than a counter. Distinct from `host_cant_attack`'s blanket ban. Reads
        // `resolved`, so the printed "or planeswalkers you control" half is covered too.
        for &(attacker, defender) in &resolved {
            if self.host_cant_attack_controller(attacker, defender) {
                return Err(Reject::IllegalDeclaration);
            }
        }

        // Goad requirements (CR 701.38a): every goaded creature the active player controls that
        // *could* attack must be attacking, and must attack a non-goader if one is a legal
        // defender. A goaded creature that can't attack at all ("if able") is simply not required.
        for id in self.controlled_battlefield(player) {
            let goaders = self.goaders_of(id);
            if goaders.is_empty() || !self.can_attack(id) {
                continue;
            }
            let Some(&(_, defender)) = attackers.iter().find(|&&(a, _)| a == id) else {
                return Err(Reject::IllegalDeclaration); // a goaded able creature must attack
            };
            let nongoader_available = self
                .living_players()
                .any(|d| d != player && !goaders.contains(&d));
            // CR 701.38a: "attacks a *player* other than you if able" — attacking a goader's
            // planeswalker is not attacking a player at all, so it doesn't satisfy the
            // requirement either.
            let attacks_a_nongoader =
                matches!(defender, Defender::Player(d) if !goaders.contains(&d));
            if !attacks_a_nongoader && nongoader_available {
                return Err(Reject::IllegalDeclaration); // must attack a non-goader if able
            }
        }

        // Must-attack requirements (CR 508.1a — Furygale Flocking's minted tokens "attack that
        // opponent this turn if able"): every creature the active player controls under a
        // `must_attack` requirement that *could* attack must be attacking, and must attack its
        // recorded defender when that defender is still a legal target. A creature that can't
        // attack at all ("if able") is simply not required — the same escape hatch goad uses. (CR 702.19, CR 701.38, CR 508)
        for id in self.controlled_battlefield(player) {
            let Some(&(_, required)) = self
                .combat_extras
                .must_attack
                .iter()
                .find(|&&(o, _)| o == id)
            else {
                continue;
            };
            if !self.can_attack(id) {
                continue;
            }
            let Some(&(_, defender)) = attackers.iter().find(|&&(a, _)| a == id) else {
                return Err(Reject::IllegalDeclaration); // a required able creature must attack
            };
            let required_legal = required != player
                && (required.0 as usize) < self.players.len()
                && !self.players[required.0 as usize].lost;
            // "Attacks that opponent this turn if able" names the *player*, so attacking one of
            // their planeswalkers doesn't discharge the requirement (CR 508.1a).
            if required_legal && defender != Defender::Player(required) {
                return Err(Reject::IllegalDeclaration); // must attack the required opponent if able
            }
        }

        let mut events = Vec::new();
        // Pillow-fort attack taxes (CR 508.1g / CR 802, Ghostly Prison): the sum owed across the
        // defending players is an additional cost of the declaration, paid up front.
        // ponytail: the tax is charged as all-generic and auto-paid (pool first, then auto-tapped
        // lands via `settle_payment`) — the declaring player implicitly agrees to pay by
        // declaring; can't-afford ⇒ illegal declaration (CR 508.1g), rather than offered as an
        // explicit pay-or-decline choice. No pool card lets the tax be anything but generic, so
        // auto-planning is exact. ponytail: goad + an unpayable tax — a goaded creature that
        // "must attack" (CR 701.38) but whose controller can't pay is technically "not able"
        // (CR 701.38 "if able"); the goad loop above still forces it. Unmodeled residual; no pool
        // card exercises goad + a tax at once. (CR 701.38)
        let tax = self.attack_tax_owed(&resolved);
        if tax > 0 {
            let cost = Cost {
                generic: tax as u8,
                ..Default::default()
            };
            self.settle_payment(player, cost, None, None, &mut events)
                .map_err(|_| Reject::IllegalDeclaration)?;
        }
        for (&(a, defender), &(_, defending_player)) in attackers.iter().zip(&resolved) {
            self.push_apply(
                &mut events,
                Event::AttackerDeclared {
                    object: a,
                    defender: defending_player,
                    defender_planeswalker: defender.object_id(),
                },
            );
            if !self.has_keyword(a, Keyword::Vigilance) {
                // CR 615: a masked Illusionary Mask attacker becoming tapped is turned face up first.
                self.flip_masked(a, &mut events);
                self.push_apply(&mut events, Event::Tapped { object: a });
            }
            // Decayed (CR 702.148c): "When it attacks, sacrifice it at the beginning of the end
            // of combat step." Scheduled against this specific attacker, not a re-scan (mirrors
            // `CreateTokenCopy`'s `sacrifice_at_next_end_step`).
            // ponytail: decayed's "when it attacks" trigger is CR-defined on every creature that
            // has the keyword, not authored ability text — modeled as a schedule fired straight
            // from declare-attackers rather than a literal per-object `Trigger`.
            if self.has_keyword(a, Keyword::Decayed) {
                self.push_apply(
                    &mut events,
                    Event::DelayedTriggerScheduled {
                        controller: player,
                        source: a,
                        fire_at: Step::EndCombat,
                        effect: Effect::Destroy(DestroyEffect::SacrificeObject { object: Some(a) }),
                    },
                );
            }
        }
        // The whole attacker set is now known — scan it once for the batch attack-count
        // triggers (CR 508.1, "attack with two or more creatures"), rather than per single
        // `AttackerDeclared` event above (a per-event fire can't see "two or more").
        self.queue_batch_attack_triggers(player, &resolved);
        self.combat.attackers_declared = true; // even a zero-attacker declaration is final
        self.consecutive_passes = 0;
        self.priority = self.active_player;
        Ok(events)
    }

    /// An attacked player declares blocks. They may only block attackers aimed at them; each
    /// blocker must be an untapped creature they control; a flyer can only be blocked by a
    /// flyer. Each attacked player declares once, in priority (APNAP) order.
    pub(crate) fn declare_blockers(
        &mut self,
        player: PlayerId,
        blocks: &[(ObjectId, ObjectId)],
    ) -> Result<Vec<Event>, Reject> {
        if !self.is_attacked_player(player)
            || self.step != Step::DeclareBlockers
            || self.combat.blocked_by.contains(&player)
        {
            return Err(Reject::IllegalDeclaration);
        }
        for &(blocker, attacker) in blocks {
            if !self.can_block(player, blocker, attacker) {
                return Err(Reject::IllegalDeclaration);
            }
        }
        // Menace (CR 509.1b): an attacker with menace must be blocked by two or more creatures.
        for &attacker in &self.combat.attackers {
            let n = blocks.iter().filter(|&&(_, a)| a == attacker).count();
            if n == 1 && self.has_keyword(attacker, Keyword::Menace) {
                return Err(Reject::IllegalDeclaration);
            }
        }

        let mut events = Vec::new();
        for &(blocker, attacker) in blocks {
            self.push_apply(&mut events, Event::BlockerDeclared { blocker, attacker });
        }
        // Goblin Cadets' "whenever this creature blocks or becomes blocked" (CR 509): scan the
        // whole declaration once, not per `BlockerDeclared` event — a multiply-blocked attacker's
        // "becomes blocked" fires only once, same reasoning as the batch attack-count scan below.
        self.queue_blocks_or_becomes_blocked_triggers(blocks);
        self.combat.blocked_by.push(player); // this defender's block declaration is final
        // If an attacker is blocked by several creatures, its controller orders them.
        if let Some((attacker, blockers)) = self.next_undivided_multiblock() {
            crate::pending::raise_choice(
                self,
                PendingChoice::AssignCombatDamage {
                    player: self.active_player,
                    attacker,
                    blockers,
                },
            );
        }
        self.consecutive_passes = 0;
        self.priority = self.active_player;
        Ok(events)
    }

    /// The first multi-blocked attacker whose damage division hasn't been chosen yet, if any.
    pub(crate) fn next_undivided_multiblock(&self) -> Option<(ObjectId, Vec<ObjectId>)> {
        for &attacker in &self.combat.attackers {
            let blockers = self.blockers_of(attacker);
            let divided = self.combat.damage.iter().any(|(a, _)| *a == attacker);
            if blockers.len() >= 2 && !divided {
                return Some((attacker, blockers));
            }
        }
        None
    }

    /// The living attackers `blocker` is blocking, in declaration order (Gomazoa's "each creature
    /// it's blocking" — the reverse read of [`Self::blockers_of`]).
    pub(crate) fn attackers_blocked_by(&self, blocker: ObjectId) -> Vec<ObjectId> {
        let alive = |a: &ObjectId| self.as_permanent(*a).is_some();
        self.combat
            .blocks
            .iter()
            .filter(|(b, _)| *b == blocker)
            .map(|(_, a)| *a)
            .filter(alive)
            .collect()
    }

    /// The living creatures blocking `attacker`, in declaration order.
    pub(crate) fn blockers_of(&self, attacker: ObjectId) -> Vec<ObjectId> {
        let alive = |b: &ObjectId| self.as_permanent(*b).is_some();
        self.combat
            .blocks
            .iter()
            .filter(|(_, a)| *a == attacker)
            .map(|(b, _)| *b)
            .filter(alive)
            .collect()
    }

    /// Whether any attacking or blocking creature has first or double strike as the combat
    /// damage step begins (CR 510.5) — the condition for creating a separate first-strike
    /// combat damage step. When false, that step is skipped and only the normal one occurs.
    pub(crate) fn any_first_strike_in_combat(&self) -> bool {
        let strikes_first = |&o: &ObjectId| {
            self.as_permanent(o).is_some()
                && (self.has_keyword(o, Keyword::FirstStrike)
                    || self.has_keyword(o, Keyword::DoubleStrike))
        };
        self.combat.attackers.iter().any(strikes_first)
            || self.combat.blocks.iter().any(|(b, _)| strikes_first(b))
    }

    /// One combat-damage batch: creatures whose first-strike status matches this batch
    /// deal their damage (attackers to blockers/player, blockers to their attacker).
    pub(crate) fn combat_damage_substep(
        &mut self,
        first_strike_batch: bool,
        events: &mut Vec<Event>,
    ) {
        for attacker in self.combat.attackers.clone() {
            if self.as_permanent(attacker).is_none() {
                continue;
            }
            // CR 615: a masked attacker that would assign or deal combat damage is turned face up
            // first — before its power is read below, so it deals its real power. (An attacker is
            // normally revealed earlier by the declare-time tap; this covers a vigilant one.)
            if self.deals_this_batch(attacker, first_strike_batch) {
                self.flip_masked(attacker, events);
            }
            // The defender may have gone between substeps (CR 704 killed the defending player, or
            // first-strike damage killed the attacked planeswalker): the attacker stays in combat
            // but assigns no damage to it (CR 510.1a). Its *blockers* still deal theirs below. (CR 702.7, CR 506)
            let defender = self
                .defender_of(attacker)
                .filter(|&d| self.defender_controller(d).is_some());
            let blockers = self.blockers_of(attacker);

            if self.deals_this_batch(attacker, first_strike_batch) {
                match (blockers.is_empty(), defender) {
                    (true, Some(defender)) => {
                        self.damage_defender(attacker, defender, self.power(attacker), events)
                    }
                    (true, None) => {}
                    (false, _) => {
                        self.assign_attacker_damage(attacker, &blockers, defender, events)
                    }
                }
            }
            for blocker in blockers {
                if self.as_permanent(blocker).is_some()
                    && self.deals_this_batch(blocker, first_strike_batch)
                {
                    // CR 615: a masked blocker that would deal combat damage is turned face up
                    // first — before its power is read, so it deals its real power.
                    self.flip_masked(blocker, events);
                    self.damage_creature(blocker, attacker, events);
                }
            }
        }
    }

    /// Assign a blocked attacker's power across its blockers, then any leftover to the defender it
    /// is attacking if it tramples (CR 510.1c — the player *or* the planeswalker, whichever it
    /// attacked; `None` once that defender is gone). A multi-blocked attacker uses the
    /// controller's chosen division (stored in `combat.damage`); otherwise damage falls
    /// lethal-to-each in declaration order.
    pub(crate) fn assign_attacker_damage(
        &mut self,
        attacker: ObjectId,
        blockers: &[ObjectId],
        defender: Option<Defender>,
        events: &mut Vec<Event>,
    ) {
        // CR 615: a masked blocker that would be dealt combat damage is turned face up first, before
        // the lethal-damage split reads its (now real) toughness below.
        for &blocker in blockers {
            self.flip_masked(blocker, events);
        }
        // Moment's Peace (CR 615, #150): a this-turn table-wide "prevent all combat damage"
        // shield cancels the attacker's damage to every blocker before any is assigned, so no
        // trample overflow is computed either — same silent guard as `deal_creature_damage`'s.
        if self.combat_extras.prevent_all_combat_damage_this_turn {
            return;
        }
        // Fog Bank (CR 615, #220): a permanent "prevent all combat damage ... dealt by" static on
        // the attacker itself cancels every bit of damage it would deal this combat — its own
        // blocker shares below, and (since none is assigned) any trample overflow too.
        if self.combat_damage_prevented_by_source(attacker) {
            return;
        }
        let deathtouch = self.has_keyword(attacker, Keyword::Deathtouch);
        let power = self.power(attacker);

        // A chosen division wins; fall back to lethal-in-order for single blocks / no choice.
        let assignment: Vec<(ObjectId, i32)> = match self
            .combat
            .damage
            .iter()
            .find(|(a, _)| *a == attacker)
            .map(|(_, split)| split.clone())
        {
            Some(split) => split,
            None => {
                let mut remaining = power;
                let mut split = Vec::new();
                for &blocker in blockers {
                    if remaining <= 0 {
                        break;
                    }
                    let lethal = if deathtouch {
                        1
                    } else {
                        (self.toughness(blocker) - self.permanent(blocker).marked_damage).max(1)
                    };
                    let assign = remaining.min(lethal);
                    remaining -= assign;
                    split.push((blocker, assign));
                }
                split
            }
        };

        // `assigned` tracks every amount the split above committed to a blocker, whether or not
        // it's later prevented; `dealt` tracks only damage that actually lands (for lifelink).
        // CR 510.1c / 702.19e: trample overflow is power minus *assigned*, not minus *dealt* — a
        // blocker's protection or self-shield can't reroute its assigned share to the player.
        let mut dealt = 0;
        let mut assigned = 0;
        for (blocker, amount) in assignment {
            if amount <= 0 || self.as_permanent(blocker).is_none() {
                continue;
            }
            assigned += amount;
            // Protection prevents this blocker's share (CR 702.16d); it still counts as assigned.
            if self.damage_prevented_by_protection(blocker, Some(attacker)) {
                continue;
            }
            // Guard Gomazoa / Fog Bank (CR 615, #220): a permanent "prevent all combat damage ...
            // dealt to" static on the blocker itself prevents this share, same as protection
            // above — the prevented share still counts as assigned.
            if self.combat_damage_prevented_to_creature(blocker) {
                continue;
            }
            // A blocking Phantom Centaur prevents this share and removes one of its own +1/+1
            // counters instead (CR 615) — the same self-shield `deal_creature_damage` applies
            // on the blocker-to-attacker path. The prevented share still counts as assigned.
            if self.phantom_shield_active(blocker) {
                if let Some(removal) = self.phantom_shield_counter_removal(blocker) {
                    self.push_apply(events, removal);
                }
                continue;
            }
            dealt += amount;
            self.push_apply(
                events,
                Event::DamageMarked {
                    object: blocker,
                    amount,
                    source: Some(attacker),
                },
            );
            // CR 510.2: this is combat damage to a creature — a `DealsCombatDamageToCreature`
            // watch (Stinkweed Imp) fires off this marker, not the plain `DamageMarked` above.
            self.push_apply(
                events,
                Event::CombatDamageDealtToCreature {
                    source: attacker,
                    target: blocker,
                    amount,
                },
            );
            if deathtouch {
                self.push_apply(events, Event::DeathtouchMarked { object: blocker });
            }
        }
        self.gain_lifelink(attacker, dealt, events);
        let leftover = power - assigned;
        if leftover > 0
            && self.has_keyword(attacker, Keyword::Trample)
            && let Some(defender) = defender
        {
            self.damage_defender(attacker, defender, leftover, events);
        }
    }

    /// Deal `amount` combat damage from `source` to whatever it's attacking (CR 508.1a): the
    /// defending player's life, or the attacked planeswalker's loyalty.
    fn damage_defender(
        &mut self,
        source: ObjectId,
        defender: Defender,
        amount: i32,
        events: &mut Vec<Event>,
    ) {
        match defender {
            Defender::Player(player) => self.damage_player(source, player, amount, events),
            Defender::Planeswalker(pw) => self.damage_planeswalker(source, pw, amount, events),
        }
    }

    /// Deal `amount` combat damage from `source` to the planeswalker it's attacking: that many
    /// loyalty counters are removed (CR 120.3c/306.8), never marked damage — a planeswalker has no
    /// toughness. No commander-damage tally (CR 903.10a is player-only); lifelink still applies
    /// (CR 702.15b — damage dealt is damage dealt).
    fn damage_planeswalker(
        &mut self,
        source: ObjectId,
        planeswalker: ObjectId,
        amount: i32,
        events: &mut Vec<Event>,
    ) {
        if amount <= 0 || self.as_permanent(planeswalker).is_none() {
            return;
        }
        // Protection from the source's color prevents it entirely (CR 702.16d).
        if self.damage_prevented_by_protection(planeswalker, Some(source)) {
            return;
        }
        // Fog Bank's "prevent all combat damage ... dealt by" static on the attacker, and Moment's
        // Peace's table-wide this-turn shield (CR 615) — both silent, as on the creature path.
        if self.combat_damage_prevented_by_source(source)
            || self.combat_extras.prevent_all_combat_damage_this_turn
        {
            return;
        }
        self.push_apply(
            events,
            Event::LoyaltyChanged {
                object: planeswalker,
                amount: -amount,
            },
        );
        self.gain_lifelink(source, amount, events);
    }

    /// `source` deals its combat damage to a creature `target`.
    pub(crate) fn damage_creature(
        &mut self,
        source: ObjectId,
        target: ObjectId,
        events: &mut Vec<Event>,
    ) {
        self.deal_creature_damage(source, target, self.power(source), true, events);
    }

    /// `source` deals `amount` damage to creature `target`: marks it (unless protection prevents
    /// it entirely, CR 702.16d), notes deathtouch, and grants lifelink. The shared path behind
    /// combat damage ([`damage_creature`](Self::damage_creature), which reads `source`'s current
    /// power) and [`fight`](Self::fight) (which reads both sides' power up front, before either
    /// amount is applied). `combat` marks which of the two callers this is: only noncombat damage
    /// consults Tajic's [`noncombat_damage_prevented_to_creature`](Self::noncombat_damage_prevented_to_creature)
    /// prevention static (CR 615), so combat damage passes it straight through.
    fn deal_creature_damage(
        &mut self,
        source: ObjectId,
        target: ObjectId,
        amount: i32,
        combat: bool,
        events: &mut Vec<Event>,
    ) {
        if amount <= 0 {
            return;
        }
        // CR 615: a masked Illusionary Mask creature that would be dealt damage is turned face up
        // first, then the damage lands on the revealed creature (prevention still checks it).
        self.flip_masked(target, events);
        if self.damage_prevented_by_protection(target, Some(source)) {
            return;
        }
        // Phantom Centaur (CR 615): "If damage would be dealt to Phantom Centaur, prevent that
        // damage. Remove a +1/+1 counter from Phantom Centaur." Self-only, but unlike Tajic's
        // noncombat-only static, this applies to combat damage too — checked regardless of
        // `combat`.
        if self.phantom_shield_active(target) {
            if let Some(removal) = self.phantom_shield_counter_removal(target) {
                self.push_apply(events, removal);
            }
            return;
        }
        // ponytail: silent prevention — a prevented noncombat hit just produces no `DamageMarked`
        // (no event), since Tajic reads no prevented total. Emit an `Event::DamagePrevented` here
        // (mirror Inkshield's `Event::CombatDamagePrevented`) only if a later card must observe it.
        if !combat && self.noncombat_damage_prevented_to_creature(target) {
            return;
        }
        // Guard Gomazoa / Fog Bank (CR 615, #220): a permanent combat-damage-prevention static —
        // either `target`'s own "dealt to" half or `source`'s own "dealt by" half. Combat-only,
        // like the table-wide shield below (Tajic's noncombat static is checked above instead).
        if combat
            && (self.combat_damage_prevented_to_creature(target)
                || self.combat_damage_prevented_by_source(source))
        {
            return;
        }
        // Moment's Peace (CR 615, #150): a this-turn table-wide "prevent all combat damage"
        // shield silently cancels combat damage to a creature — same silent-prevention style as
        // the noncombat guard above (no event; nothing in the pool reads a prevented total here).
        if combat && self.combat_extras.prevent_all_combat_damage_this_turn {
            return;
        }
        self.push_apply(
            events,
            Event::DamageMarked {
                object: target,
                amount,
                source: Some(source),
            },
        );
        // CR 510.2: combat damage to a creature (blocker → attacker) also fires a
        // `DealsCombatDamageToCreature` watch (Stinkweed Imp) — `fight`'s noncombat call
        // (`combat = false`) does not.
        if combat {
            self.push_apply(
                events,
                Event::CombatDamageDealtToCreature {
                    source,
                    target,
                    amount,
                },
            );
        }
        if self.has_keyword(source, Keyword::Deathtouch) {
            self.push_apply(events, Event::DeathtouchMarked { object: target });
        }
        self.gain_lifelink(source, amount, events);
    }

    /// Resolve a fight (CR 701.12): `a` and `b` each deal damage equal to their power to the
    /// other, simultaneously — both powers are read before either amount is applied (CR
    /// 510.2/701.12c), so neither side's damage affects how much the other deals.
    pub(crate) fn fight(&mut self, a: ObjectId, b: ObjectId, events: &mut Vec<Event>) {
        // CR 615: a masked Illusionary Mask creature that would deal damage is turned face up first
        // — before its power is read, so it deals its real power (its being-dealt-damage flip rides
        // on `deal_creature_damage` below).
        self.flip_masked(a, events);
        self.flip_masked(b, events);
        let power_a = self.power(a);
        let power_b = self.power(b);
        // Fight damage is noncombat (CR 701.12), so it passes `combat = false` — Tajic's static
        // prevents it, unlike combat damage.
        self.deal_creature_damage(a, b, power_a, false, events);
        self.deal_creature_damage(b, a, power_b, false, events);
    }

    /// Deal `amount` combat damage from `source` to a player: a life loss, plus a
    /// commander-damage tally if the source is a commander.
    pub(crate) fn damage_player(
        &mut self,
        source: ObjectId,
        player: PlayerId,
        amount: i32,
        events: &mut Vec<Event>,
    ) {
        if amount <= 0 {
            return;
        }
        // Prevention shield (CR 615 — Inkshield): if `player` has a this-turn "prevent all combat
        // damage dealt to you" shield up, this damage deals nothing — no life loss, no commander
        // tally, and (CR 702.15e — no damage dealt) no lifelink. Instead each prevented point mints
        // one of the shield's tokens under `player`. Consulted before the life loss so the
        // prevention wholly replaces it.
        if self.prevent_combat_damage_to_player(player, amount, source, events) {
            return;
        }
        // Fog Bank (CR 615, #220): a permanent "prevent all combat damage ... dealt by" static on
        // the attacking `source` itself — the general form Fog Bank's Defender keeps it from ever
        // reaching (it can't attack), but the query is symmetric with `to_self`'s.
        if self.combat_damage_prevented_by_source(source) {
            self.push_apply(events, Event::CombatDamagePrevented { player, amount });
            return;
        }
        // Moment's Peace (CR 615, #150): the table-wide "prevent all combat damage" shield — like
        // Inkshield's above, but every player and no token mint. Still surfaced as the same
        // `Event::CombatDamagePrevented` for observability.
        if self.combat_extras.prevent_all_combat_damage_this_turn {
            self.push_apply(events, Event::CombatDamagePrevented { player, amount });
            return;
        }
        // Moment's Peace (CR 615, #150): the table-wide "prevent all combat damage" shield — like
        // Inkshield's above, but every player and no token mint. Still surfaced as the same
        // `Event::CombatDamagePrevented` for observability.
        if self.combat_extras.prevent_all_combat_damage_this_turn {
            self.push_apply(events, Event::CombatDamagePrevented { player, amount });
            return;
        }
        self.push_apply(
            events,
            Event::LifeChanged {
                player,
                amount: -amount,
                source: Some(source),
            },
        );
        if self.is_commander(source) {
            self.push_apply(
                events,
                Event::CommanderDamageDealt {
                    source,
                    player,
                    amount,
                },
            );
        }
        self.push_apply(
            events,
            Event::CombatDamageDealtToPlayer {
                source,
                player,
                amount,
            },
        );
        self.gain_lifelink(source, amount, events);
    }

    /// Consult `player`'s combat-damage prevention shields (CR 615 — Inkshield). Returns `false`
    /// (no shield, caller deals the damage normally) or, if `player` has a shield up, prevents the
    /// `amount` (>0) combat damage: emits [`Event::CombatDamagePrevented`] in place of the life
    /// loss and mints one of the shield's tokens per prevented point under `player`, returning
    /// `true`. Uses the first matching shield's token — a second Inkshield on the same player has
    /// nothing left to prevent ("for each 1 damage prevented *this way*"), so it mints nothing.
    fn prevent_combat_damage_to_player(
        &mut self,
        player: PlayerId,
        amount: i32,
        creator: ObjectId,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(&(_, token)) = self
            .combat_extras
            .combat_damage_prevention_shields
            .iter()
            .find(|(p, _)| *p == player)
        else {
            return false;
        };
        self.push_apply(events, Event::CombatDamagePrevented { player, amount });
        // One token per point prevented (CR 615 / Inkshield), routed through the token-creation
        // replacements (Doubling Season, CR 614) the same way `Effect::Token(TokenEffect::Create)`'s mint is.
        let count = self.token_count_after_replacements(player, amount as u32);
        for next in (self.next_object_id()..).take(count as usize) {
            self.push_apply(
                events,
                Event::TokenCreated {
                    token: next,
                    controller: player,
                    def: token,
                    creator,
                },
            );
        }
        true
    }

    /// Whether `object` deals combat damage in this batch (first-strike creatures in the
    /// first-strike batch, everyone else in the normal batch).
    pub(crate) fn deals_this_batch(&self, object: ObjectId, first_strike_batch: bool) -> bool {
        if self.has_keyword(object, Keyword::DoubleStrike) {
            return true; // deals in both the first-strike and the normal batch
        }
        self.has_keyword(object, Keyword::FirstStrike) == first_strike_batch
    }

    /// Lifelink (CR 702.15): if `source` has lifelink and dealt `amount` (>0) damage, its
    /// controller gains that much life. Call at each site that deals `source`'s damage.
    pub(crate) fn gain_lifelink(&mut self, source: ObjectId, amount: i32, events: &mut Vec<Event>) {
        if amount <= 0 || !self.has_keyword(source, Keyword::Lifelink) {
            return;
        }
        let player = self.controller_of(source);
        self.push_apply(
            events,
            Event::LifeChanged {
                player,
                amount: self.life_gain_after_replacements(player, amount),
                source: Some(source),
            },
        );
    }
}
