//! Casting, activating, and paying costs.
//!
//! Primary: CR 601 (casting spells), CR 602 (activating abilities), CR 118 (costs / payments).
//! Also: alternative costs (CR 702.34 flashback, CR 702.19 escape), impulse play (CR 118.6).
//! Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::playable::{CastInputs, CastPlayKind, ValidatedCast};
use crate::*;

impl Game {
    /// The zone `player` may cast/play the card at `id` from right now, if any: their own
    /// hand, the command zone (their commander), or exile with an active impulse-play
    /// permission (CR 118.6). Shared by [`Game::cast`], [`Game::play_land`], and
    /// [`Game::meaningful_actions`], so they can't disagree.
    pub(crate) fn playable_zone(&self, id: ObjectId, player: PlayerId) -> Option<Zone> {
        let Object::Card(c) = &self.objects[id as usize] else {
            return None;
        };
        let playable = match c.zone {
            Zone::Hand => c.owner == player,
            Zone::Command => c.commander && c.owner == player,
            Zone::Exile => {
                self.may_play_from_exile(id, player) || self.may_cast_from_exile_free(id, player)
            }
            // Flashback (CR 702.34), escape (CR 702.19), retrace (CR 702.83), or a fixed
            // cast-from-graveyard alternative cost for a permanent (CR 118.9, Raffine's
            // Guidance): a card with any of the four may be cast from its owner's graveyard.
            // Serra Paragon (CR 118.9) also lets its controller play a land / cast a permanent
            // spell with mana value 3 or less from their graveyard, once during each of their
            // turns.
            Zone::Graveyard => {
                c.owner == player
                    && (c.def.flashback.is_some()
                        || c.def.escape.is_some()
                        || c.def.retrace
                        || c.def.graveyard_cast_cost.is_some()
                        || self.serra_graveyard_play_allowed(c.def, player))
            }
            _ => false,
        };
        playable.then_some(c.zone)
    }

    /// Whether Serra Paragon's permission (CR 118.9) lets `player` play/cast `def` from their
    /// graveyard right now: they control the granting permanent
    /// ([`Game::grants_graveyard_recursion`]), haven't already used the once-per-turn permission
    /// ([`Player::graveyard_play_used_this_turn`]), and `def` is a land or a permanent spell (a
    /// nonland card that isn't an instant/sorcery) with mana value 3 or less.
    fn serra_graveyard_play_allowed(&self, def: CardDef, player: PlayerId) -> bool {
        if self.players[player.0 as usize].graveyard_play_used_this_turn {
            return false;
        }
        if !self.grants_graveyard_recursion(player) {
            return false;
        }
        let is_land = matches!(def.kind, CardKind::Land { .. });
        let is_permanent_spell =
            !matches!(def.kind, CardKind::Spell { .. } | CardKind::Land { .. });
        is_land || (is_permanent_spell && def.mana_value() <= 3)
    }

    /// Input-independent additional-cost gates for casting `object` at `cost`/`x`: enough *other*
    /// hand cards for a discard rider, and enough life for a pay-life / pay-X-life rider (CR 119.4).
    /// Shared by [`Game::cast`] and [`Game::cast_listable`] so list and execute agree.
    /// Chosen discard/exile picks still re-validate in [`Game::validate_cast`].
    pub(crate) fn cast_additional_cost_gate(
        &self,
        player: PlayerId,
        object: ObjectId,
        cost: Cost,
        x: u32,
    ) -> Result<(), Reject> {
        let discard_n = cost.additional.discard as usize;
        if discard_n > 0 {
            let other = self
                .hand_of(player)
                .into_iter()
                .filter(|&id| id != object)
                .count();
            if other < discard_n {
                return Err(Reject::CannotPayCost);
            }
        }
        // Retrace (CR 702.83a): can't pay without a land card in hand to discard (CR 601.2f —
        // no land in hand means the additional cost can't be paid, so the cast is illegal).
        if cost.additional.discard_land {
            let has_land = self
                .hand_of(player)
                .into_iter()
                .any(|id| id != object && matches!(self.def_of(id).kind, CardKind::Land { .. }));
            if !has_land {
                return Err(Reject::CannotPayCost);
            }
        }
        if cost.additional.pay_life_x && self.life(player) < x as i32 {
            return Err(Reject::CannotPayCost);
        }
        if self.life(player) < cost.additional.pay_life as i32 {
            return Err(Reject::CannotPayCost);
        }
        Ok(())
    }

    /// The full cost to cast `def` right now: the chosen `{X}` folded into the generic
    /// component (a no-`{X}` cost ignores it), the command-zone tax of {2} per previous cast
    /// from there, static cost reducers (CR 118.9 — generic only, floored at 0), the spell's own
    /// board-derived reduction (CR 601.2f, e.g. Blasphemous Act), and ward (CR 702.21). Shared by
    /// [`Game::cast`] and [`Game::meaningful_actions`] (which prices with no target and `x = 0`),
    /// so a spell never reads as affordable to one and not the other for the same inputs.
    ///
    /// `object` is the card being priced, passed as the amount resolver's `source` — only
    /// [`Amount::SourcePower`]/[`Amount::PerCounterOnSource`]-shaped amounts would read it, and
    /// no pool card's `reduce_own_generic` is one of those (Blasphemous Act's is a board count
    /// that ignores `source`), but a real id is threaded through regardless of whether today's
    /// amount reads it.
    ///
    /// Ward: targeting an opponent's warded permanent taxes the cast {N} more.
    /// ponytail: modeled as a cast-time tax rather than a "countered on resolution unless you
    /// pay {N}" trigger — you simply can't cast without the extra mana. The visible difference
    /// (a warded creature can't be saved by responding to the ward trigger) doesn't matter for
    /// our single-pass casting; grow the trigger form if ward-on-the-stack interaction is needed. (CR 702.21, CR 601.2c, CR 405)
    /// ponytail: only a non-modal spell's `target` is taxed; a modal spell's per-mode targets
    /// aren't folded into ward/cost-reduction — no pool modal card targets a warded permanent. (CR 702.21, CR 700.2, CR 601.2c)
    /// `delve_count` is the number of graveyard cards the caster is exiling to pay delve (CR
    /// CR 702.66) — 0 for a non-delve spell, or when pricing without a chosen count (e.g. the
    /// list-affordability probe before a client picks exile cards).
    /// `kicked` folds [`AdditionalCost::kicker`]'s cost on top (CR 702.33d) — `false` for a
    /// spell with no kicker, or when pricing without picking one (list/one-click never offer it,
    /// matching the declinable default).
    /// `bought_back` folds [`AdditionalCost::buyback`]'s cost on top (CR 702.27c), mirroring
    /// `kicked`'s own fold — `false` for a spell with no buyback, or when pricing without picking
    /// one (list/one-click never offer it, matching kicker's own declinable default).
    /// `evoked` charges [`CardDef::evoke`] instead of the printed cost (CR 702.74a) — `false` for
    /// a spell with no evoke, or when pricing without declaring it (list/one-click never offer
    /// it, matching kicker's own declinable default above).
    /// `strive_count` folds [`AdditionalCost::strive`]'s cost on top, multiplied by
    /// `strive_count.saturating_sub(1)` (CR 702.42 — "for each target beyond the first") — 0 for
    /// a spell with no Strive, or when pricing without a declared count (list/one-click price at
    /// the base "zero extra targets" cost, matching kicker's declinable default).
    /// `replicate_count` folds [`AdditionalCost::replicate`]'s cost on top, multiplied by the
    /// count itself (CR 702.108 — each payment is a full extra instance of the cost, unlike
    /// Strive's "beyond the first") — 0 for a spell with no Replicate, or when pricing without a
    /// declared count.
    #[allow(clippy::too_many_arguments)]
    pub fn cast_cost(
        &self,
        player: PlayerId,
        object: ObjectId,
        def: CardDef,
        target: Option<Target>,
        x: u32,
        zone: Zone,
        delve_count: u8,
        kicked: bool,
        bought_back: bool,
        evoked: bool,
        strive_count: u8,
        replicate_count: u8,
    ) -> Cost {
        // Quintorius, Loremaster's free-cast permission (CR 118.5 "without paying its mana
        // cost"): the mana cost is zero. No pool card stacks an additional cost onto a
        // free-cast-eligible card, so `Cost::FREE` covers it; split mana from `additional` if one
        // ever does.
        if zone == Zone::Exile && self.may_cast_from_exile_free(object, player) {
            return Cost::FREE;
        }
        // A printed conditional free-cast permission (CR 118.5, `CardDef::free_cast_if` —
        // Massacre: "you may cast this spell without paying its mana cost"): the caster always
        // takes it once the gate holds, same "no reason to decline a strictly-better cost"
        // modeling as the exile permission above.
        if let Some(condition) = def.free_cast_if
            && self.condition_holds(condition, TriggerContext::of(player))
        {
            return Cost::FREE;
        }
        let from_command = zone == Zone::Command;
        // Flashback (CR 702.34), escape (CR 702.19), or a fixed cast-from-graveyard alternative
        // cost for a permanent (CR 118.9, Raffine's Guidance) casts for that alternative cost,
        // replacing the printed mana cost as the base the reduction pipeline below operates on.
        // Retrace (CR 702.83a) has no alternative cost — it pays the printed `[cost]` as normal,
        // falling through to `def.cost` below like any other graveyard-less cast.
        // ponytail: cost reducers still apply to the alternative cost, and no flashback/escape/
        // graveyard_cast_cost pool card interacts with one, so reusing the pipeline is harmless.
        // (CR 702.34, CR 702.19, CR 702.83, CR 118.9)
        let base = if zone == Zone::Graveyard {
            def.flashback
                .or(def.escape.map(|escape| escape.cost))
                .or(def.graveyard_cast_cost)
                .unwrap_or(def.cost)
        } else if evoked {
            // Evoke (CR 702.74a): the caster's declared evoke cost replaces the printed cost —
            // `validate_cast_cost_picks` already rejects `evoked` on a card with no evoke cost,
            // so this only sees a real one.
            def.evoke.unwrap_or(def.cost)
        } else {
            def.cost
        };
        // A pay-X-life additional cost (CR 601.2b/601.2f — Toxic Deluge) chooses `{X}` for the
        // effect and the life payment, but `{X}` itself is never mana: fold in 0 rather than the
        // real `x` here so the caster doesn't pay X twice (X mana *and* X life). `Game::cast`
        // pays the real `x` as life alongside this mana cost.
        let mut cost = if base.additional.pay_life_x {
            base.with_x(0)
        } else {
            base.with_x(x)
        };
        if from_command {
            cost.generic += self.commander_tax(player);
        }
        cost.generic = cost
            .generic
            .saturating_sub(self.cost_reduction(player, def, target, zone));
        if let Some(amount) = def.cost.reduce_own_generic {
            let discount = self
                .resolve_amount(amount, player, object, target, x)
                .max(0) as u8;
            cost.generic = cost.generic.saturating_sub(discount);
        }
        // Delve (CR 702.66): each exiled graveyard card pays for {1} of the generic cost.
        if def.delve {
            cost.generic = cost.generic.saturating_sub(delve_count);
        }
        // Kicker (CR 702.33d): the caster's chosen kicker cost, paid alongside the printed cost.
        // ponytail: sums generic/colored/colorless pips only — no pool kicker cost carries a
        // hybrid pip of its own; grow that if one ever does.
        if kicked && let Some(kicker) = base.additional.kicker {
            cost.generic = cost.generic.saturating_add(kicker.generic);
            for (pip, extra) in cost.colored.iter_mut().zip(kicker.colored.iter()) {
                *pip = pip.saturating_add(*extra);
            }
            cost.colorless = cost.colorless.saturating_add(kicker.colorless);
        }
        // Buyback (CR 702.27c): the caster's chosen buyback cost, paid alongside the printed
        // cost, mirroring kicker's own fold above.
        // ponytail: sums generic/colored/colorless pips only, mirroring kicker's own pip-only
        // fold — no pool buyback cost carries a hybrid pip.
        if bought_back && let Some(buyback) = base.additional.buyback {
            cost.generic = cost.generic.saturating_add(buyback.generic);
            for (pip, extra) in cost.colored.iter_mut().zip(buyback.colored.iter()) {
                *pip = pip.saturating_add(*extra);
            }
            cost.colorless = cost.colorless.saturating_add(buyback.colorless);
            // A non-mana buyback rider (CR 702.27f — Constant Mists' "Buyback—Sacrifice a
            // land"): the buyback cost's own `additional.sacrifice` rider (the same
            // `[[cost.additional.buyback.additional]]` sub-table `AdditionalCost::sacrifice`
            // uses for a plain spell) folds into the returned cost's sacrifice rider, so
            // `Game::validate_cast_cost_picks`/the `sacrifice_cost` pay loop in `Game::cast`
            // require and pay it exactly like an ordinary additional-sacrifice cost.
            // ponytail: overwrites rather than merges with a base-spell `additional.sacrifice`
            // — no pool card combines a buyback-sacrifice rider with the base spell's own
            // separate sacrifice cost; merge counts/filters if one ever does.
            if let Some(sacrifice) = buyback.additional.sacrifice {
                cost.additional.sacrifice = Some(sacrifice);
            }
        }
        // Strive (CR 601.2f/702.42): "{2}{R} more to cast for each target beyond the first" —
        // the caster's declared target count (settled pre-stack, see `Intent::Cast::strive_count`'s
        // own doc) scales the additional cost. `N = 0` or `N = 1` adds nothing (no targets beyond
        // the first).
        // ponytail: sums generic/colored/colorless pips only, mirroring kicker's own pip-only
        // fold above — no pool Strive cost carries a hybrid pip.
        if let Some(strive) = base.additional.strive {
            let extra = strive_count.saturating_sub(1);
            cost.generic = cost
                .generic
                .saturating_add(strive.generic.saturating_mul(extra));
            for (pip, per) in cost.colored.iter_mut().zip(strive.colored.iter()) {
                *pip = pip.saturating_add(per.saturating_mul(extra));
            }
            cost.colorless = cost
                .colorless
                .saturating_add(strive.colorless.saturating_mul(extra));
        }
        // Replicate (CR 702.108b): "You may pay [cost] any number of times as you cast this
        // spell" — each payment is a full extra instance of the cost (unlike Strive's "beyond the
        // first"), so the declared count multiplies straight through.
        // ponytail: sums generic/colored/colorless pips only, mirroring kicker/strive's own
        // pip-only fold above — Changing Loyalty's replicate cost carries none.
        if let Some(replicate) = base.additional.replicate {
            cost.generic = cost
                .generic
                .saturating_add(replicate.generic.saturating_mul(replicate_count));
            for (pip, per) in cost.colored.iter_mut().zip(replicate.colored.iter()) {
                *pip = pip.saturating_add(per.saturating_mul(replicate_count));
            }
            cost.colorless = cost
                .colorless
                .saturating_add(replicate.colorless.saturating_mul(replicate_count));
        }
        if let Some(Target::Object(id)) = target
            && self.controller_of(id) != player
            && let Some(n) = self.ward_amount(id)
        {
            cost.generic = cost.generic.saturating_add(n);
        }
        cost
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cast(
        &mut self,
        player: PlayerId,
        object: ObjectId,
        target: Option<Target>,
        x: u32,
        modes: Vec<(usize, Option<Target>)>,
        discard_cost: Vec<ObjectId>,
        graveyard_exile: Vec<ObjectId>,
        sacrifice_cost: Vec<ObjectId>,
        kicked: bool,
        bought_back: bool,
        evoked: bool,
        strive_count: u8,
        replicate_count: u8,
    ) -> Result<Vec<Event>, Reject> {
        self.cast_with_kind(
            player,
            object,
            target,
            x,
            &modes,
            &discard_cost,
            &graveyard_exile,
            &sacrifice_cost,
            kicked,
            bought_back,
            evoked,
            strive_count,
            replicate_count,
            playable::CastPlayKind::Full,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn cast_with_kind(
        &mut self,
        player: PlayerId,
        object: ObjectId,
        target: Option<Target>,
        x: u32,
        modes: &[(usize, Option<Target>)],
        discard_cost: &[ObjectId],
        graveyard_exile: &[ObjectId],
        sacrifice_cost: &[ObjectId],
        kicked: bool,
        bought_back: bool,
        evoked: bool,
        strive_count: u8,
        replicate_count: u8,
        kind: CastPlayKind,
    ) -> Result<Vec<Event>, Reject> {
        let validated = self.validate_cast(
            player,
            object,
            &CastInputs {
                target,
                x,
                modes,
                discard_cost,
                graveyard_exile,
                sacrifice_cost,
                kicked,
                bought_back,
                evoked,
                strive_count,
                replicate_count,
            },
            kind,
        )?;
        let ValidatedCast {
            zone: _,
            def,
            cost,
            from_command,
            cast_via_flashback,
            cast_via_escape,
            chosen_modes: chosen,
            multi_target,
            x,
            target,
        } = validated;

        // Pay first, then move the card onto the stack as a spell. The mana settles ahead of the
        // discards: settling is the last fallible step (an unpayable cost rejects here with
        // nothing tapped and nothing discarded), auto-tapping lands for whatever the pool lacks.
        let mut events = Vec::new();
        self.settle_payment(
            player,
            cost,
            None,
            Some(def.spell_characteristics()),
            &mut events,
        )?;
        // CR 106.9's "spent to cast" query (Court Hussar's "unless {W} was spent to cast it"):
        // read right off the `Event::ManaSpent` `settle_payment` just appended, before any later
        // event dilutes the `events` tail.
        let spent_colors = spent_colors_from(&events);
        // ponytail: the additional discard is a *cost* (CR 601.2h — paid pre-stack, before
        // SpellCast below), distinct from a resolution-time `Effect::Discard`. Applied
        // incrementally (not a single `apply_all`) so each `next_object_id()` — one per discarded
        // card, then the spell itself — sees the previous event already applied.
        for &id in discard_cost {
            let card = self.next_object_id();
            let def = self.def_of(id);
            self.push_apply(&mut events, Event::MovedToGraveyard { card, from: id });
            // CR 701.8/601.2h: a cost discard is still a discard — fires "whenever you discard"
            // watchers too.
            self.push_apply(
                &mut events,
                Event::Discarded {
                    card,
                    from: id,
                    def,
                    player,
                },
            );
        }
        // A delve or escape graveyard-exile payment (CR 601.2f/601.2h — paid pre-stack, before
        // SpellCast below). Applied incrementally, like the discard loop above, so each
        // `next_object_id()` sees the previous event already applied.
        for &id in graveyard_exile {
            let card = self.next_object_id();
            self.push_apply(&mut events, Event::MovedToExile { card, from: id });
        }
        // An optional additional sacrifice cost (CR 601.2f — Plumb the Forbidden's "you may
        // sacrifice one or more creatures"), paid pre-stack like the discard loop above. Routes
        // through the normal death events so "when this/a creature dies" watchers fire off it.
        for &id in sacrifice_cost {
            let def = self.def_of(id);
            let event = self.sacrifice_event(id);
            self.push_apply(&mut events, event);
            self.push_apply(
                &mut events,
                Event::Sacrificed {
                    object: id,
                    by: player,
                    def,
                },
            );
        }
        // Pay-X-life additional cost (CR 601.2f — Toxic Deluge): the chosen `{X}` funds this
        // life payment instead of mana (see `cast_cost`'s `pay_life_x` branch).
        if cost.additional.pay_life_x {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -(x as i32),
                    source: Some(object),
                },
            );
        }
        // Fixed pay-life additional cost (CR 601.2f — Deep Analysis's flashback "Pay 3 life"),
        // a flat amount that never touches mana.
        if cost.additional.pay_life > 0 {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -(cost.additional.pay_life as i32),
                    source: Some(object),
                },
            );
        }
        let spell_id = self.next_object_id();
        self.push_apply(
            &mut events,
            Event::SpellCast {
                spell: spell_id,
                from: object,
                controller: player,
                target,
                x,
                modes: chosen,
                flashback: cast_via_flashback,
                escape: cast_via_escape,
                sacrifice_count: sacrifice_cost.len() as u8,
                kicked,
                bought_back,
                strive_count,
                replicate_count,
                bestowed: false,
                face_down: false,
                masked: false,
                evoked,
                spent_colors,
            },
        );
        if from_command {
            self.push_apply(&mut events, Event::CommanderCastFromCommandZone { player });
        }
        // A multi-target spell now chooses its targets (CR 601.2c): auto-fill when the choice is
        // forced (a single legal set — take them all), otherwise pause on a ChooseSpellTargets.
        if let Some((spec, count)) = multi_target {
            self.choose_spell_targets(spell_id, spec, count, player, player, &mut events);
        }
        // Replicate (CR 702.108b-c): "copy it for each time you paid its replicate cost … This
        // all happens before any player has a chance to cast spells or activate abilities in
        // response" — minted right here at the cast choke (not a triggered ability, doesn't use
        // the stack), reusing the same `mint_spell_copies` rider `Effect::CopyThisSpell` uses for
        // its own per-copy CR 707.10c retarget.
        if replicate_count > 0 {
            self.mint_spell_copies(
                Amount::Fixed(replicate_count as i32),
                player,
                spell_id,
                target,
                x,
                &mut events,
            );
        }
        // Casting is an action: it resets the pass count and the caster keeps priority. (CR 117, CR 601)
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Whether `modes` is a legal modal choice for `def` (CR 700.2): between `modal_choose` and
    /// `modal_choose_max` entries (an exact count when `modal_choose_max` is `None` — CR 700.2d's
    /// "one or more" is the open-max case, further gated by `controller` controlling a commander
    /// when `modal_choose_max_if_commander` is set — see [`Game::modal_choose_max`]), each a
    /// distinct in-range printed mode whose target is legal for that mode. A mode whose own effect
    /// is multi-target (Prismari Charm's "one or two targets") takes no target here — like a
    /// non-modal multi-target spell, its targets are chosen after cast (CR 601.2c), so
    /// `modal_multi_target` below is where that mode's spec/count is read.
    pub(crate) fn validate_modes(
        &self,
        object: ObjectId,
        def: CardDef,
        modes: &[(usize, Option<Target>)],
        controller: PlayerId,
        x: u32,
    ) -> Result<(), Reject> {
        let max = self.modal_choose_max(def, controller);
        if !(def.modal_choose as usize..=max as usize).contains(&modes.len()) {
            return Err(Reject::IllegalMode);
        }
        let mut seen: Vec<usize> = Vec::new();
        for &(m, target) in modes {
            let Some(ability) = nth_mode(def, m) else {
                return Err(Reject::IllegalMode);
            };
            if m >= MAX_MODES || seen.contains(&m) {
                return Err(Reject::IllegalMode);
            }
            seen.push(m);
            if !ability.effect.target_count().is_single() {
                if target.is_some() {
                    return Err(Reject::IllegalTarget);
                }
                continue;
            }
            if !self.targets_are_legal(object, def, target, controller, Some(m), x) {
                return Err(Reject::IllegalTarget);
            }
        }
        Ok(())
    }

    /// A non-modal spell's multi-target effect (its target spec and count), if it has one —
    /// i.e. a `Timing::Spell` ability whose effect chooses more than one target (Aether Gale).
    /// `None` for the overwhelming single-target majority. The pool never sequences two
    /// multi-target spell effects under one spell, so the first is authoritative. Modal spells
    /// use [`Self::modal_multi_target`] instead, scoped to the chosen mode.
    pub(crate) fn spell_multi_target(&self, def: CardDef) -> Option<(TargetSpec, TargetCount)> {
        def.abilities
            .iter()
            .filter(|a| matches!(a.timing, Timing::Spell))
            .map(|a| (a.effect.target(), a.effect.target_count()))
            .find(|(_, count)| !count.is_single())
    }

    /// A modal spell's chosen mode's multi-target effect (its target spec and count), if the mode
    /// picked needs one (Prismari Charm mode 1's "deals 1 damage to each of one or two targets").
    /// Mirrors [`Self::spell_multi_target`] but scoped to just the *chosen* modes — a modal spell
    /// resolves only what was chosen, so at most one multi-target clause is ever in play.
    pub(crate) fn modal_multi_target(
        &self,
        def: CardDef,
        modes: &[(usize, Option<Target>)],
    ) -> Option<(TargetSpec, TargetCount)> {
        modes
            .iter()
            .filter_map(|&(m, _)| nth_mode(def, m))
            .map(|a| (a.effect.target(), a.effect.target_count()))
            .find(|(_, count)| !count.is_single())
    }

    /// The `clause`-th *independent* target clause of a non-modal spell — the spec/count of its
    /// `clause`-th `Timing::Spell` ability that chooses more than one target (CR 601.2c). `None`
    /// once every clause is exhausted (and always for a modal spell, whose per-mode targets go
    /// through [`Self::modal_multi_target`] — a modal spell resolves at most one multi-target
    /// clause). Clause 0 is what [`Self::spell_multi_target`] reports; Magma Opus adds clause 1 (its
    /// "Tap two target permanents").
    pub(crate) fn spell_target_clause(
        &self,
        def: CardDef,
        clause: usize,
    ) -> Option<(TargetSpec, TargetCount)> {
        if def.modal {
            return None;
        }
        def.abilities
            .iter()
            .filter(|a| matches!(a.timing, Timing::Spell))
            .map(|a| (a.effect.target(), a.effect.target_count()))
            .filter(|(_, count)| !count.is_single())
            .nth(clause)
    }

    /// `def`'s first `Timing::Spell` ability with a target — spec and count (CR 707.10c/CR
    /// 114.6a), single- or multi-target alike. The shared lookup [`Effect::CopyTargetSpell`]'s
    /// copy-retarget and [`Effect::ChangeTargetOfTargetSpellOrAbility`]'s optional (Wild Ricochet)
    /// bend both key off. `None` for a targetless spell (no retarget/copy-retarget offered). A
    /// later independent clause (Magma Opus) is reached separately once clause 0 settles, through
    /// [`Self::spell_target_clause`] via [`Self::advance_spell_target_clauses`].
    pub(crate) fn spell_primary_target(&self, def: CardDef) -> Option<(TargetSpec, TargetCount)> {
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.timing, Timing::Spell))?;
        let spec = ability.effect.target();
        (spec != TargetSpec::None).then(|| (spec, ability.effect.target_count()))
    }

    /// Record a just-cast multi-target spell's chosen targets (CR 601.2c), starting at its first
    /// independent target clause. Delegates to [`Self::choose_spell_target_clause`], which chains
    /// through every clause in printed order before the divided-damage/counter split runs. Reused by
    /// `Effect::CopyTargetSpell`/`ChangeTargetOfTargetSpellOrAbility` (in `effects.rs`) to offer a
    /// copy's or a bent original's CR 707.10c/114.6a retarget — same shape, just against an
    /// already-on-the-stack spell id instead of a just-cast one. `anchor` is whose perspective
    /// legality is evaluated from (the bent/copied spell's own controller — CR 114.6/707.10a);
    /// `chooser` is who actually answers the pause. The two always coincide for a fresh cast or a
    /// copy (a copy's controller *is* the chooser), but can differ when retargeting the ORIGINAL
    /// spell (Wild Ricochet may bend an opponent's spell without becoming its controller).
    pub(crate) fn choose_spell_targets(
        &mut self,
        spell: ObjectId,
        spec: TargetSpec,
        count: TargetCount,
        anchor: PlayerId,
        chooser: PlayerId,
        events: &mut Vec<Event>,
    ) {
        self.choose_spell_target_clause(spell, 0, spec, count, anchor, chooser, events);
    }

    /// After clause `clause` of `spell` is settled, choose the next independent clause (CR 601.2c —
    /// all a spell's targets are chosen at once, in printed order) or, once none remain, run the
    /// CR 601.2d divided-damage/counter split over the finished target sets. See
    /// [`Self::choose_spell_targets`] for `anchor`/`chooser`.
    pub(crate) fn advance_spell_target_clauses(
        &mut self,
        spell: ObjectId,
        clause: usize,
        anchor: PlayerId,
        chooser: PlayerId,
        events: &mut Vec<Event>,
    ) {
        let def = self.def_of(spell);
        if let Some((spec, count)) = self.spell_target_clause(def, clause) {
            self.choose_spell_target_clause(spell, clause, spec, count, anchor, chooser, events);
            return;
        }
        self.maybe_begin_damage_division(spell, events);
        self.maybe_begin_counter_division(spell, events);
    }

    /// Record one target clause's chosen targets. The caster must choose between `count.min` and
    /// `count.max` distinct legal targets — but capped at how many legal targets exist (CR 601.2c:
    /// "the maximum possible number"). When that's a single forced set (take every legal target, no
    /// room to choose fewer or which), it's auto-filled here with no pause and the next clause runs;
    /// otherwise the caster answers a [`PendingChoice::ChooseSpellTargets`] carrying this `clause`.
    /// See [`Self::choose_spell_targets`] for `anchor`/`chooser`.
    #[allow(clippy::too_many_arguments)]
    fn choose_spell_target_clause(
        &mut self,
        spell: ObjectId,
        clause: usize,
        spec: TargetSpec,
        count: TargetCount,
        anchor: PlayerId,
        chooser: PlayerId,
        events: &mut Vec<Event>,
    ) {
        let legal = self.legal_targets_for(
            spec,
            spell,
            anchor,
            color_identity(self.def_of(spell)),
            self.spell(spell).x,
        );
        let n = legal.len();
        // CR 601.2b: X is chosen before targets are chosen, so an `x_scaled` count (Curse of the
        // Swine's "exile X target creatures", Silkguard's "up to X") substitutes the spell's own
        // chosen X for its placeholder `min`/`max` here — the only choke point X needs to enter
        // multi-target selection.
        // ponytail: the substituted X is clamped to MAX_TARGETS (the fixed TargetList width, CR
        // 601.2c's "maximum possible" is itself capped there) — bump that const before a real
        // board needs an X-target spell targeting more than six.
        // CR 601.2f: the sacrifice-defined sibling of the above — Immoral Bargain's X is settled
        // by the additional cost paid before the spell was even put on the stack (see
        // `Game::cast_with_kind`'s `sacrifice_cost` loop, which runs before this is reached), so
        // `spell_sacrifice_count` is already known here. Always "exactly X" (no pool card sac-
        // scales a declinable "up to X"), unlike `x_scaled`'s `count.min == 0` case above.
        // CR 601.2c/601.2f/702.42: Strive's own sibling — Twinflame's target count is the
        // caster's declared pre-stack commitment (see `Intent::Cast::strive_count`'s own doc),
        // already recorded as `spell_strive_count` by the time this runs, same as sacrifice above.
        let (min, max) = if count.x_scaled {
            let x = (self.spell(spell).x as usize).min(MAX_TARGETS) as u8;
            (if count.min == 0 { 0 } else { x }, x)
        } else if count.sacrifice_scaled {
            let x = (self.spell_sacrifice_count(spell) as usize).min(MAX_TARGETS) as u8;
            (x, x)
        } else if count.strive_scaled {
            let x = (self.spell_strive_count(spell) as usize).min(MAX_TARGETS) as u8;
            (x, x)
        } else {
            (count.min, count.max)
        };
        let lo = (min as usize).min(n) as u8;
        let hi = (max as usize).min(n) as u8;
        // Forced: exactly one legal set (must take all `n`, no option to take fewer). Auto-fill,
        // then chain into the next clause (or the divided-damage split once every clause is set).
        if lo == hi && hi as usize == n {
            self.push_apply(
                events,
                Event::SpellTargetsChosen {
                    spell,
                    targets: TargetList::from_targets(&legal),
                    clause: clause as u8,
                },
            );
            self.advance_spell_target_clauses(spell, clause + 1, anchor, chooser, events);
            return;
        }
        crate::pending::raise_choice(
            self,
            PendingChoice::ChooseSpellTargets {
                player: chooser,
                spell,
                min: lo,
                max: hi,
                legal,
                clause: clause as u8,
            },
        );
    }

    /// After a multi-target spell's targets are finalized (CR 601.2c), also settle CR 601.2d's
    /// division for a `divided: true` `Effect::DealDamage` on that spell (Magma Opus's "4 damage
    /// divided as you choose among any number of targets"). A no-op for a spell with no divided
    /// effect, or whose divided effect has no chosen targets (a legal "any number... including
    /// none" of zero). A single chosen target needs no choice — the whole amount is auto-assigned
    /// to it, mirroring `next_undivided_multiblock`'s single-blocker skip for combat damage.
    /// Called from both `choose_spell_targets`'s forced-autofill branch above and
    /// `Game::choose_spell_targets_answer`'s player-chosen branch.
    pub(crate) fn maybe_begin_damage_division(&mut self, spell: ObjectId, events: &mut Vec<Event>) {
        let spell_obj = self.spell(spell);
        let def = spell_obj.def;
        let controller = spell_obj.controller;
        let x = spell_obj.x;
        let Some(ability) = def.abilities.iter().find(|a| {
            matches!(a.timing, Timing::Spell)
                && matches!(a.effect, Effect::DealDamage { divided: true, .. })
        }) else {
            return;
        };
        let Effect::DealDamage { amount, .. } = ability.effect else {
            unreachable!("guarded by the divided-DealDamage find above")
        };
        // "Any number of targets" (CR 601.2d) admits creatures *and* players — collect both.
        let targets: Vec<Target> = self.spell(spell).targets.iter().collect();
        if targets.is_empty() {
            return;
        }
        let total = self.resolve_amount(amount, controller, spell, None, x);
        if let [only] = targets[..] {
            self.push_apply(events, spell_damage_divided(spell, &[(only, total)]));
            return;
        }
        pending::raise(
            self,
            pending::ChoiceRequest::DivideSpellDamage {
                player: controller,
                spell,
                targets,
                total,
            },
        );
    }

    /// After a multi-target spell's targets are finalized (CR 601.2c), also settle CR 601.2d's
    /// division for a `divided: true` `Effect::PutCounters` on that spell (Grove's Bounty's
    /// "Distribute X +1/+1 counters among any number of target creatures you control"). Mirrors
    /// [`Self::maybe_begin_damage_division`] exactly — a no-op for a spell with no divided-
    /// counters effect or no chosen targets; a single chosen target auto-takes the whole total.
    /// Called from the same two sites as its damage twin.
    pub(crate) fn maybe_begin_counter_division(
        &mut self,
        spell: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let spell_obj = self.spell(spell);
        let def = spell_obj.def;
        let controller = spell_obj.controller;
        let x = spell_obj.x;
        let Some(ability) = def.abilities.iter().find(|a| {
            matches!(a.timing, Timing::Spell)
                && matches!(a.effect, Effect::PutCounters { divided: true, .. })
        }) else {
            return;
        };
        let Effect::PutCounters { count, .. } = ability.effect else {
            unreachable!("guarded by the divided-PutCounters find above")
        };
        // A divided effect's targets are always permanents (see `Effect::PutCounters`'s doc).
        let targets: Vec<ObjectId> = self
            .spell(spell)
            .targets
            .iter()
            .filter_map(Target::object_id)
            .collect();
        if targets.is_empty() {
            return;
        }
        let total = self.resolve_count(count, controller, spell, None, x) as i32;
        if let [only] = targets[..] {
            self.push_apply(
                events,
                Event::SpellCountersDivided {
                    spell,
                    assignment: DamageAssignment::from_pairs(&[(only, total)]),
                },
            );
            return;
        }
        pending::raise(
            self,
            pending::ChoiceRequest::DivideCounters {
                player: controller,
                spell,
                targets,
                total,
            },
        );
    }

    /// Play a land from hand: a special action (no stack), once per turn, during the
    /// player's own main phase with an empty stack.
    pub(crate) fn play_land(
        &mut self,
        player: PlayerId,
        object: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        let Object::Card(card) = self.objects[object as usize] else {
            return Err(Reject::NotCastable);
        };
        // Playable from hand, exile, or (via Serra Paragon, CR 118.9) the graveyard — a land is
        // never cast from the command zone.
        if !matches!(
            self.playable_zone(object, player),
            Some(Zone::Hand | Zone::Exile | Zone::Graveyard)
        ) || !matches!(card.def.kind, CardKind::Land { .. })
        {
            return Err(Reject::NotCastable);
        }
        if !self.can_take_sorcery_speed_action(player)
            || self.players[player.0 as usize].lands_played >= 1
        {
            return Err(Reject::WrongTiming);
        }

        let events = vec![Event::LandPlayed {
            permanent: self.next_object_id(),
            from: object,
            player,
        }];
        self.apply_all(&events);
        Ok(events)
    }

    /// Validate a [`SacrificeCost`]'s named picks (CR 118.9/602.2b — checked before anything is
    /// paid, an uncompletable/unnamed cost makes the activation illegal): `None` needs no picks,
    /// `This` sacrifices `source` itself, `Creature { filter, count }` needs exactly `count`
    /// distinct permanents `player` controls matching `filter`. Read-only (no events) — shared by
    /// [`Self::activate_ability`]'s activation-sacrifice cost and [`Self::cycle`]'s cycling one
    /// (CR 702.29b, Edge of Autumn's "Cycling—Sacrifice a land").
    fn validate_sacrifice_cost(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: SacrificeCost,
        named: &[ObjectId],
    ) -> Result<Vec<ObjectId>, Reject> {
        match cost {
            SacrificeCost::None => Ok(Vec::new()),
            SacrificeCost::This => Ok(vec![source]),
            SacrificeCost::Creature { filter, count } => {
                if named.len() != count as usize {
                    return Err(Reject::CannotActivate);
                }
                let mut chosen: Vec<ObjectId> = Vec::with_capacity(named.len());
                for &s in named {
                    let legal = !chosen.contains(&s)
                        && self.as_permanent(s).is_some()
                        && self.controller_of(s) == player
                        && self.permanent_matches(&filter, s, player, Some(source));
                    if !legal {
                        return Err(Reject::CannotActivate);
                    }
                    chosen.push(s);
                }
                Ok(chosen)
            }
        }
    }

    /// Pay a validated sacrifice cost's events (CR 118.9): each of `sacrificed` goes to the
    /// graveyard via the normal death choke, so "when this/a creature dies" watchers fire off it
    /// like any other sacrifice. Shared by [`Self::activate_ability`] and [`Self::cycle`], paired
    /// with [`Self::validate_sacrifice_cost`].
    fn pay_sacrifice_events(
        &mut self,
        player: PlayerId,
        sacrificed: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        for &id in sacrificed {
            let def = self.def_of(id);
            let sac = self.sacrifice_event(id);
            self.push_apply(events, sac);
            self.push_apply(
                events,
                Event::Sacrificed {
                    object: id,
                    by: player,
                    def,
                },
            );
        }
    }

    /// Activate a hand card's Cycling ability (CR 702.29a — "{N}, Discard this card: Draw a
    /// card."): pay the mana and any [`CardDef::cycling_sacrifice`] (CR 702.29b), discard `card`
    /// (that's the rest of the cost), then draw one card. This is the engine's first
    /// activate-from-hand surface — a keyword ability that functions only from the hand, not a
    /// permanent's [`Timing::Activated`]. `sacrifice` names the permanent paying
    /// `cycling_sacrifice`; ignored for a card whose cycling carries none.
    pub(crate) fn cycle(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        sacrifice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        // Cycling is an activated ability (CR 702.29) — requires priority (CR 117.1b).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Object::Card(c) = self.objects[card as usize] else {
            return Err(Reject::CannotActivate);
        };
        if c.zone != Zone::Hand || c.owner != player {
            return Err(Reject::CannotActivate);
        }
        let Some(cost) = c.def.cycling else {
            return Err(Reject::CannotActivate);
        };
        // Resolve the cycling sacrifice cost up front (CR 118.9/602.2b), same choke an ordinary
        // activation's sacrifice cost uses.
        let named: Vec<ObjectId> = sacrifice.into_iter().collect();
        let sacrificed =
            self.validate_sacrifice_cost(player, card, c.def.cycling_sacrifice, &named)?;

        // Pay the cost — mana (settled first, auto-tapping lands; an unpayable cost rejects
        // before the discard), the sacrifice, and "discard this card" (CR 702.29a) — before the
        // ability resolves.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)
            .map_err(|_| Reject::CannotActivate)?;
        self.pay_sacrifice_events(player, &sacrificed, &mut events);
        self.push_apply(
            &mut events,
            Event::MovedToGraveyard {
                card: self.next_object_id(),
                from: card,
            },
        );
        // CR 702.29a: cycling is an activated ability — its "Draw a card" goes on the stack as a
        // real (source-less) activated ability, so a responder (Azorius Guildmage's "counter
        // target activated ability") can interact with it before it resolves. `card` is the
        // now-discarded card, last-known information like any other post-move source.
        self.push_ability_group(
            player,
            card,
            &[(
                Effect::DrawCards {
                    count: Amount::Fixed(1),
                },
                None,
            )],
            true,
            &mut events,
        );
        // CR 702.29e: "when you cycle this card" triggers off the discard above and — placed by
        // the ordinary trigger pipeline in `after_events` — lands on top of the draw already on
        // the stack, so it resolves first (Krosan Tusker's "(Do this before you draw.)"). Scanned
        // off the cycled card's own def, mirroring `Trigger::YouCastThis`'s self-scan.
        if c.def
            .abilities
            .iter()
            .any(|a| a.timing == Timing::Triggered(Trigger::Cycled))
        {
            self.queue_trigger_group(TriggerContext::of(player), card, c.def, Trigger::Cycled);
        }
        // An action resets the pass count; the cycler keeps priority (CR 117.3c) — overriding the
        // active-player default `push_ability_group` set.
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Activate a hand card's [`CardDef::hand_ability`] (CR 113.6/602.5e — a hand-activated,
    /// discard-this-card ability with an authored payload, e.g. Magma Opus's "{U/R}{U/R},
    /// Discard this card: Create a Treasure token.") or [`CardDef::forecast`] (CR 702.57 —
    /// Skyscribing's Forecast, which *reveals* rather than discards its card and is gated to the
    /// controller's own upkeep, once each turn). The general sibling of [`Game::cycle`] for a
    /// card whose from-hand ability isn't cycling's fixed draw-1 — same skeleton (priority,
    /// hand+owner check, pay cost, then discard-or-reveal), an authored effect list instead of a
    /// hardcoded draw. No pool card has both `hand_ability` and `forecast`, so `hand_ability` (if
    /// present) always wins; only `forecast`'s own gates apply when it's the one carried.
    pub(crate) fn activate_hand_ability(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        // A hand-activated ability is still an activated ability (CR 602) — requires priority
        // (CR 117.1b).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Object::Card(c) = self.objects[card as usize] else {
            return Err(Reject::CannotActivate);
        };
        if c.zone != Zone::Hand || c.owner != player {
            return Err(Reject::CannotActivate);
        }
        let forecast = c.def.hand_ability.is_none();
        let ability = match (c.def.hand_ability, c.def.forecast) {
            (Some(ability), _) => ability,
            (None, Some(ability)) => {
                // Forecast (CR 702.57a): activated only during the controller's own upkeep, and
                // only once each turn.
                if self.step != Step::Upkeep || self.active_player != player {
                    return Err(Reject::CannotActivate);
                }
                // ponytail: reuses the battlefield `once_each_turn` activation-cap store
                // (`(source, ability_index)`, normally a real ability array index) with a fixed
                // sentinel index of 0 — a hand card carries at most one `forecast`/`hand_ability`,
                // so there's no battlefield ability index to collide with on the same object id.
                if self
                    .once_per_turn
                    .activated
                    .iter()
                    .any(|&(o, i)| o == card && i == 0)
                {
                    return Err(Reject::CannotActivate);
                }
                ability
            }
            (None, None) => return Err(Reject::CannotActivate),
        };

        // Pay the cost — mana (settled first, auto-tapping lands; an unpayable cost rejects
        // before the discard/reveal) and, for `hand_ability`, "discard this card" (the rest of
        // the cost) — before the ability goes on the stack.
        let mut events = Vec::new();
        self.settle_payment(player, ability.cost, None, None, &mut events)
            .map_err(|_| Reject::CannotActivate)?;
        if forecast {
            // Forecast reveals rather than discards (CR 702.57) — the card stays in hand.
            // ponytail: the reveal itself isn't a modeled event (no observer reads it — nothing
            // downstream keys off "was this card revealed"), the same unobserved-reveal posture
            // the check-land family (`furycalm_snarl` et al.) already takes for a hand reveal;
            // model a real reveal event if a future card reads one. Once each turn is recorded so
            // a repeat activation this turn rejects above.
            self.push_apply(
                &mut events,
                Event::AbilityActivatedThisTurn {
                    object: card,
                    ability_index: 0,
                },
            );
        } else {
            self.push_apply(
                &mut events,
                Event::MovedToGraveyard {
                    card: self.next_object_id(),
                    from: card,
                },
            );
        }
        // CR 113.6/602: this is an activated ability — its authored payload goes on the stack (a
        // single stack ability, `Sequence`-wrapped when it has more than one step) so a responder
        // (Azorius Guildmage) can interact with it, just like cycling's draw above. `card` is the
        // now-discarded (or still-in-hand, revealed) source, last-known information.
        // ponytail: a hand ability's payload takes no target in the pool (Magma Opus's Treasure,
        // the landcyclers' library search); pushed with `None`. Thread a chosen target through
        // here if a targeted hand ability ever appears.
        let effect = match ability.effects {
            [single] => *single,
            steps => Effect::Sequence { steps },
        };
        self.push_ability_group(player, card, &[(effect, None)], true, &mut events);
        // An action resets the pass count; the activator keeps priority (CR 117.3c) — overriding
        // the active-player default `push_ability_group` set.
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Suspend a hand card (CR 702.62 — Rousing Refrain): rather than cast it, pay its
    /// [`CardDef::suspend`] cost and exile it with N time counters. A special action from the
    /// hand, usable any time the card could be cast (CR 702.62b — sorcery-speed for a sorcery).
    /// A time counter is removed at each of the owner's upkeeps ([`Game::perform_turn_based_actions`]),
    /// and the owner may cast it for free once the last is gone.
    pub(crate) fn suspend(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Object::Card(c) = self.objects[card as usize] else {
            return Err(Reject::CannotActivate);
        };
        if c.zone != Zone::Hand || c.owner != player {
            return Err(Reject::CannotActivate);
        }
        let Some(suspend) = c.def.suspend else {
            return Err(Reject::CannotActivate);
        };
        // CR 702.62b: a card may be suspended any time it could be cast (timing follows the card).
        if !self.cast_timing_ok(player, card, c.def, playable::CastPlayKind::Full) {
            return Err(Reject::WrongTiming);
        }

        // Pay the suspend cost (auto-tapping lands; an unpayable cost rejects before the exile),
        // then exile the card with its time counters (CR 702.62c).
        let mut events = Vec::new();
        self.settle_payment(player, *suspend.cost, None, None, &mut events)
            .map_err(|_| Reject::CannotActivate)?;
        self.push_exile_with_time_counters(card, suspend.counters, &mut events);
        // An action resets the pass count; the player keeps priority (CR 117.3c).
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Encore a graveyard card (CR 702.140 — Angel of Indemnity): pay its [`CardDef::encore`]
    /// mana cost and exile it from the graveyard (both halves of the cost, CR 702.140a) to create
    /// under this player, for each opponent, a token copy of the card that must attack that
    /// opponent this turn if able (CR 702.140c), gains haste, and is sacrificed at the beginning
    /// of the next end step. A sorcery-speed activated ability from the graveyard (CR 702.140b),
    /// modeled with [`Game::suspend`]'s pay-then-exile shape (see the second ponytail below).
    ///
    /// ponytail: the per-opponent must-attack haste-copy body is the reusable substrate for
    /// myriad (muddle_the_ever_changing, #74/#88) — the same shape; factor a shared helper when
    /// the myriad keyword lands.
    ///
    /// ponytail: encore is a non-mana activated ability (CR 702.140), so its effect is meant to go
    /// on the stack (the `push_ability_group` path other non-mana activations use), giving
    /// opponents a response window before the copies appear. This resolves it immediately instead
    /// (pay/exile, then mint, keep priority), so there is no priority window between activation and
    /// the copies entering — behavior-identical for angel_of_indemnity (nothing responds to the
    /// copies mid-activation in the pool), but a card that lets an opponent interact with the
    /// ability on the stack needs the real stack path. Upgrade: route the copy body through an
    /// `Effect` placed with `push_ability_group`.
    pub(crate) fn encore(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        // Encore is an activated ability (CR 702.140) — requires priority (CR 117.1b).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Object::Card(c) = self.objects[card as usize] else {
            return Err(Reject::CannotActivate);
        };
        if c.zone != Zone::Graveyard || c.owner != player {
            return Err(Reject::CannotActivate);
        }
        let Some(cost) = c.def.encore else {
            return Err(Reject::CannotActivate);
        };
        // CR 702.140b: encore may be activated only as a sorcery.
        if !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        let def = c.def;

        // Pay the cost (CR 702.140a): the encore mana cost (settled first, auto-tapping lands; an
        // unpayable cost rejects before any exile) plus exiling this card from the graveyard.
        let mut events = Vec::new();
        self.settle_payment(player, *cost, None, None, &mut events)
            .map_err(|_| Reject::CannotActivate)?;
        self.push_apply(
            &mut events,
            Event::MovedToExile {
                card: self.next_object_id(),
                from: card,
            },
        );

        // "For each opponent, create a token copy of this card that attacks that opponent this
        // turn if able. They gain haste. Sacrifice them at the beginning of the next end step."
        // ponytail: the minted token's `def` is the card's printed `CardDef` copyable values (CR
        // 707.2), not a full copy-layer read — the same shortcut #127's copy slices carry.
        const HASTE: &[Keyword] = &[Keyword::Haste];
        let opponents: Vec<PlayerId> = self.living_players().filter(|&p| p != player).collect();
        for opponent in opponents {
            // Doubling Season (CR 614): each copy enters under `player`.
            let count = self.token_count_after_replacements(player, 1);
            for _ in 0..count {
                let token = self.next_object_id();
                self.push_apply(
                    &mut events,
                    Event::TokenCreated {
                        token,
                        controller: player,
                        def,
                        creator: card,
                    },
                );
                self.push_apply(
                    &mut events,
                    Event::MustAttackDeclared {
                        object: token,
                        defender: opponent,
                    },
                );
                self.push_apply(
                    &mut events,
                    Event::TempBoost {
                        object: token,
                        power: 0,
                        toughness: 0,
                        keywords: HASTE,
                        source_name: def.name,
                    },
                );
                self.push_apply(
                    &mut events,
                    Event::DelayedTriggerScheduled {
                        controller: player,
                        source: card,
                        fire_at: Step::End,
                        effect: Effect::SacrificeObject {
                            object: Some(token),
                        },
                    },
                );
            }
        }
        // A special action resets the pass count; the player keeps priority (CR 117.3c).
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Turn a face-down permanent face up: a special action (no stack) usable any time its
    /// controller has priority. Pay the reveal cost — a morph card's *morph* cost (CR 702.37c) if
    /// it has one, otherwise a manifest's hidden *printed* cost (CR 701.34e — Reality Shift) — then
    /// clear the face-down flag to reveal it. Only a creature card may be turned face up (a
    /// noncreature manifest stays a 2/2 forever).
    pub(crate) fn turn_face_up(
        &mut self,
        player: PlayerId,
        permanent: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Some(perm) = self.as_permanent(permanent) else {
            return Err(Reject::CannotActivate);
        };
        if !perm.face_down || perm.owner != player {
            return Err(Reject::CannotActivate);
        }
        // CR 701.34e: only a creature card may be turned face up.
        let CardKind::Creature { .. } = perm.def.kind else {
            return Err(Reject::CannotActivate);
        };
        // A morph card turns up for its morph cost (CR 702.37c); a manifest (no morph) pays the
        // hidden card's printed cost (CR 701.34e).
        // ponytail: a manifested *morph* card (CR 702.37j — pay either the {3}-back manifest turn
        // or the morph cost) isn't modeled; no pool card manifests a morph card, so a `morph`
        // card here was always a morph cast and its morph cost is correct. Add the dual-cost fork
        // when a card first manifests a morph card.
        let cost = perm.def.morph.unwrap_or(perm.def.cost);

        // Pay the hidden card's mana cost (auto-tapping lands; an unpayable cost rejects before the
        // reveal), then flip it face up.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)
            .map_err(|_| Reject::CannotActivate)?;
        self.turn_face_up_free(permanent, &mut events);
        // A special action resets the pass count; the player keeps priority (CR 117.3c).
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Reveal a face-down `permanent` — emit and apply [`Event::TurnedFaceUp`] with none of the
    /// special-action bookkeeping (no payment, no priority/pass reset). This is the shared reveal
    /// tail of the [`Game::turn_face_up`] special action and also the free flip driven by
    /// Illusionary Mask's CR 615 replacement (see [`Game::flip_masked`]), which reveals a masked
    /// creature mid-event without it being an action.
    pub(crate) fn turn_face_up_free(&mut self, permanent: ObjectId, events: &mut Vec<Event>) {
        self.push_apply(events, Event::TurnedFaceUp { permanent });
    }

    /// Illusionary Mask's CR 615 self-replacement: if `object` is a masked face-down permanent, it
    /// "is turned face up" first (for free) the instant it would assign or deal damage, be dealt
    /// damage, or become tapped — so the interaction then proceeds on the revealed creature (its
    /// real characteristics come from `def` once `face_down` clears). A no-op for a plain
    /// morph/manifest face-down permanent (not `masked`) or an already-face-up one.
    pub(crate) fn flip_masked(&mut self, object: ObjectId, events: &mut Vec<Event>) {
        let Some(perm) = self.as_permanent(object) else {
            return;
        };
        if !perm.masked || !perm.face_down {
            return;
        }
        self.turn_face_up_free(object, events);
    }

    /// Cast a copy of a prepared permanent's back-face spell (soc/sos prepare DFCs — Kirol,
    /// History Buff). While `source` is prepared ([`Permanent::prepared`]), its controller "may
    /// cast a copy of its spell" (the [`CardDef::back`] face): this pays the back face's mana
    /// cost, puts the copy on the stack, and unprepares the source ("Doing so unprepares it").
    /// The front permanent stays on the battlefield.
    ///
    /// ponytail: "cast a **copy** of its spell" is modeled as casting the back-face spell itself
    /// from the battlefield, paying its cost (CR — casting a copy pays the cost). Since the front
    /// permanent stays put and the copy ceases to exist on resolve, this is functionally identical
    /// to a copy for every pool prepare card — a true spell-copy vs. the original face is
    /// unobservable here (no pool card observes the difference).
    pub(crate) fn cast_prepared(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        // Casting requires priority (CR 117.1a).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Some(perm) = self.as_permanent(source) else {
            return Err(Reject::CannotActivate);
        };
        // CR 602.2: casting a prepared permanent's back face is its controller's action — a
        // stolen prepared permanent casts for its thief, not its owner.
        if self.controller_of(source) != player {
            return Err(Reject::CannotActivate);
        }
        if !perm.prepared {
            return Err(Reject::CannotActivate);
        }
        let Some(back) = perm.def.back else {
            return Err(Reject::CannotActivate);
        };
        let back = *back;
        // "You may cast a copy of its spell" — casting obeys the back face's timing (Pack a Punch
        // is a sorcery: sorcery-speed only).
        if !back.is_instant_speed() && !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        // CR 601.2b: {X} is chosen ahead of targets, same clamp `Game::validate_cast` applies.
        let x = x.min(u8::MAX as u32);
        // A back face may itself be a multi-target spell (Run the Play's "up to X target
        // creatures" — CR 601.2c): its targets are chosen *after* the cast, the same
        // choose-after-cast flow `Game::cast_with_kind` gives a directly-cast multi-target
        // spell, rather than the single up-front `target` a single-target/untargeted back face
        // takes.
        let multi_target = self.spell_multi_target(back);
        if let Some((spec, count)) = multi_target {
            if target.is_some() {
                return Err(Reject::IllegalTarget);
            }
            let n = self
                .legal_targets_for(spec, source, player, color_identity(back), x)
                .len();
            if count.min > 0 && n == 0 {
                return Err(Reject::IllegalTarget);
            }
        } else if !self.targets_are_legal(source, back, target, player, None, x) {
            return Err(Reject::IllegalTarget);
        }

        // Pay the back face's mana cost first (settling is the last fallible step — an unpayable
        // cost rejects here with nothing tapped and the source still prepared).
        let cost = self.cast_cost(
            player,
            source,
            back,
            target,
            x,
            Zone::Battlefield,
            0,
            false,
            false,
            false,
            0,
            0,
        );
        let mut events = Vec::new();
        self.settle_payment(
            player,
            cost,
            None,
            Some(back.spell_characteristics()),
            &mut events,
        )?;
        // Put the copy on the stack, then unprepare the source.
        let spell = self.next_object_id();
        self.push_apply(
            &mut events,
            Event::PreparedSpellCast {
                spell,
                source,
                controller: player,
                target,
                x,
            },
        );
        self.push_apply(
            &mut events,
            Event::PreparedChanged {
                object: source,
                prepared: false,
            },
        );
        if let Some((spec, count)) = multi_target {
            self.choose_spell_targets(spell, spec, count, player, player, &mut events);
        }
        // Casting is an action: reset the pass count; the caster keeps priority. (CR 117, CR 601)
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Cast the adventure half of an adventure card from hand (CR 715 — Brazen Borrower's Petty
    /// Theft, Elusive Otter's Grove's Bounty). `source` is the card in `player`'s hand; casting
    /// pays its [`CardDef::adventure`] face's mana cost and puts that instant/sorcery spell on the
    /// stack. On resolution the card is exiled "on an adventure" (see
    /// [`Game::finish_instant_sorcery_resolution`]) and its owner may cast the creature half from
    /// exile later at normal cost. Mirrors [`Game::cast_prepared`], but the source is a hand card
    /// (not a battlefield permanent) and its adventure face — not its back face — is cast.
    pub(crate) fn cast_adventure(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        // Casting requires priority (CR 117.1a).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        // The adventure half is cast from the card's owner's hand (CR 715.3c).
        if self.playable_zone(source, player) != Some(Zone::Hand) {
            return Err(Reject::NotCastable);
        }
        let front = self.def_of(source);
        let Some(adventure) = front.adventure else {
            return Err(Reject::NotCastable);
        };
        let adventure = *adventure;
        // The adventure obeys its own timing — a sorcery (Grove's Bounty) is sorcery-speed only.
        if !adventure.is_instant_speed() && !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        // CR 601.2b: {X} is chosen ahead of targets, same clamp `Game::validate_cast` applies.
        let x = x.min(u8::MAX as u32);
        // An adventure may be multi-target (Grove's Bounty's "any number of target creatures"):
        // its targets are chosen after the cast, like a directly-cast multi-target spell.
        let multi_target = self.spell_multi_target(adventure);
        if let Some((spec, count)) = multi_target {
            if target.is_some() {
                return Err(Reject::IllegalTarget);
            }
            let n = self
                .legal_targets_for(spec, source, player, color_identity(adventure), x)
                .len();
            if count.min > 0 && n == 0 {
                return Err(Reject::IllegalTarget);
            }
        } else if !self.targets_are_legal(source, adventure, target, player, None, x) {
            return Err(Reject::IllegalTarget);
        }

        // Pay the adventure face's mana cost first (settling is the last fallible step — an
        // unpayable cost rejects here with nothing tapped and the card still in hand).
        let cost = self.cast_cost(
            player,
            source,
            adventure,
            target,
            x,
            Zone::Hand,
            0,
            false,
            false,
            false,
            0,
            0,
        );
        let mut events = Vec::new();
        self.settle_payment(
            player,
            cost,
            None,
            Some(adventure.spell_characteristics()),
            &mut events,
        )?;
        // Move the card from hand onto the stack as its adventure spell.
        let spell = self.next_object_id();
        self.push_apply(
            &mut events,
            Event::AdventureSpellCast {
                spell,
                source,
                controller: player,
                target,
                x,
            },
        );
        if let Some((spec, count)) = multi_target {
            self.choose_spell_targets(spell, spec, count, player, player, &mut events);
        }
        // Casting is an action: reset the pass count; the caster keeps priority. (CR 117, CR 601)
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Cast a card for its bestow cost (CR 702.103 — Eidolon of Countless Battles). `object` is the
    /// card in `player`'s hand; casting pays its [`CardDef::bestow`] alternative cost and puts it on
    /// the stack as a bestowed Aura spell with "enchant creature" (CR 702.103c), targeting `target`.
    /// On resolution it enters attached to `target` (see [`Game::resolve_spell`]) as an Aura, not a
    /// creature (CR 702.103e). Mirrors [`Game::cast_adventure`], but pays the bestow cost and marks
    /// the resulting [`Spell::bestowed`] rather than casting a different face.
    pub(crate) fn cast_bestow(
        &mut self,
        player: PlayerId,
        object: ObjectId,
        target: Option<Target>,
    ) -> Result<Vec<Event>, Reject> {
        // Casting requires priority (CR 117.1a).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        // Bestow is cast from the card's owner's hand.
        if self.playable_zone(object, player) != Some(Zone::Hand) {
            return Err(Reject::NotCastable);
        }
        let def = self.def_of(object);
        let Some(bestow) = def.bestow else {
            return Err(Reject::NotCastable);
        };
        // A bestowed permanent (enchantment) creature spell obeys its creature nature's timing —
        // sorcery speed only (CR 601.3e/307), no flash.
        if !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        // Bestow grants "enchant creature" (CR 702.103c): the cast target must be a creature. Reuse
        // the Aura cast-target legality by viewing the card as an Aura here — `required_target`
        // returns `def.enchant` (any creature for Eidolon) for `CardKind::Aura`.
        let as_aura = CardDef {
            kind: CardKind::Aura,
            ..def
        };
        if !self.targets_are_legal(object, as_aura, target, player, None, 0) {
            return Err(Reject::IllegalTarget);
        }
        // Pay the bestow cost first (settling is the last fallible step — an unpayable cost rejects
        // here with nothing tapped and the card still in hand).
        // ponytail: pays the flat bestow cost — no cost reducer / commander-tax / ward pipeline
        // (`cast_cost`), since Eidolon is the only bestow card in the pool and none of those
        // interact with it. Route through `cast_cost` if a bestow card ever needs a reduction.
        let mut events = Vec::new();
        self.settle_payment(
            player,
            bestow,
            None,
            Some(def.spell_characteristics()),
            &mut events,
        )?;
        let spent_colors = spent_colors_from(&events);
        let spell = self.next_object_id();
        self.push_apply(
            &mut events,
            Event::SpellCast {
                spell,
                from: object,
                controller: player,
                target,
                x: 0,
                modes: Modes::default(),
                flashback: false,
                escape: false,
                sacrifice_count: 0,
                kicked: false,
                bought_back: false,
                strive_count: 0,
                replicate_count: 0,
                bestowed: true,
                face_down: false,
                masked: false,
                evoked: false,
                spent_colors,
            },
        );
        // Casting is an action: reset the pass count; the caster keeps priority. (CR 117, CR 601)
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Cast a hand card face down as a 2/2 creature for {3} (CR 702.37b — morph). Any card whose
    /// [`CardDef::morph`] is `Some` may be cast this way, paying a flat generic {3} (independent
    /// of the card's morph cost, which is what turns it face up later). It lands as a face-down
    /// creature spell → face-down permanent (CR 708.2: a 2/2 colorless creature with no name,
    /// types, or abilities until turned up). Cast at ordinary creature-spell timing (sorcery
    /// speed).
    ///
    /// ponytail: a flat {3}, not routed through `cast_cost` — no cost reducer / ward pipeline
    /// interacts with a face-down cast (its real cost is hidden). Route through `cast_cost` if a
    /// pool card ever reduces the face-down cast cost.
    pub(crate) fn cast_face_down(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        // Casting requires priority (CR 117.1a).
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        // A morph card is cast face down from its owner's hand.
        if self.playable_zone(card, player) != Some(Zone::Hand) {
            return Err(Reject::NotCastable);
        }
        if self.def_of(card).morph.is_none() {
            return Err(Reject::NotCastable);
        }
        // A face-down creature spell obeys creature-spell timing — sorcery speed only.
        if !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        // CR 702.37b: the face-down cast cost is a flat generic {3}, not the card's printed or
        // morph cost. Settle first — the last fallible step, so an unpayable {3} rejects with the
        // card still in hand.
        let face_down_cost = Cost {
            generic: 3,
            ..Cost::FREE
        };
        let mut events = Vec::new();
        self.settle_payment(player, face_down_cost, None, None, &mut events)?;
        let spent_colors = spent_colors_from(&events);
        // A morph cast is not masked — only Illusionary Mask sets the CR 615 replacement.
        self.push_face_down_spell_cast(player, card, spent_colors, false, &mut events);
        // Casting is an action: reset the pass count; the caster keeps priority. (CR 117, CR 601)
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
    }

    /// Put `card` onto the stack as a face-down 2/2 creature spell (CR 708.2) controlled by
    /// `player`, `spent_colors` recording which colors (if any) were spent casting it. Shared by
    /// morph's flat `{3}` cast ([`Game::cast_face_down`]) and Illusionary Mask's free `{X}` cast
    /// ([`Game::cast_creature_face_down`]) — the two differ only in what (if anything) was paid and
    /// whether the result is `masked` (Illusionary Mask's CR 615 turn-face-up-on-interaction
    /// replacement), so neither the priority reset nor the payment lives here.
    pub(crate) fn push_face_down_spell_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        spent_colors: [bool; Color::COUNT],
        masked: bool,
        events: &mut Vec<Event>,
    ) {
        let spell = self.next_object_id();
        self.push_apply(
            events,
            Event::SpellCast {
                spell,
                from: card,
                controller: player,
                target: None,
                x: 0,
                modes: Modes::default(),
                flashback: false,
                escape: false,
                sacrifice_count: 0,
                kicked: false,
                bought_back: false,
                strive_count: 0,
                replicate_count: 0,
                bestowed: false,
                face_down: true,
                masked,
                evoked: false,
                spent_colors,
            },
        );
    }

    /// The gates on activating ability `index` of `source` that don't depend on the chosen
    /// inputs (target, sacrifice) or on how the mana cost will be paid: `player` owns a live
    /// permanent with that activated ability, or (CR 112.6/603.6e) owns a `functions_in_graveyard`
    /// card sitting in their graveyard with it; loyalty rules (CR 606: sorcery-speed, once per
    /// turn, a `−N` needs loyalty ≥ N); equip's sorcery-speed timing (CR 702.6e); a tap cost
    /// needs an untapped, non-sick body; a life cost needs that much life (CR 119.4). Returns
    /// the ability and its cost on success, or the exact rejection for intent validation;
    /// [`Game::meaningful_actions`] reads the same gate (any `Err` = "not activatable"), so
    /// the two can't disagree.
    pub(crate) fn ability_activation_gate(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: usize,
    ) -> Result<(Ability, ActivationCost), Reject> {
        let perm = self.as_permanent(source);
        // CR 112.6/603.6e: a card whose def is flagged `functions_in_graveyard` activates its
        // ability *only* from the graveyard (Teacher's Pest's "{B}{G}: Return this card ... to
        // the battlefield tapped") — the activated-ability twin of the restriction the
        // triggered-ability scans already enforce (`queue_death_watcher` skips such a card while
        // it's on the battlefield). Every other non-permanent object (an ordinary graveyard card,
        // a spell on the stack) has no activatable ability at all.
        let graveyard_functional = self.def_of(source).functions_in_graveyard;
        if graveyard_functional {
            if self.zone_of(source) != Zone::Graveyard {
                return Err(Reject::CannotActivate);
            }
        } else if perm.is_none() {
            return Err(Reject::CannotActivate);
        }
        // CR 602.2: only a permanent's *controller* may activate its abilities — a stolen
        // permanent activates for its thief, not its owner. (A `functions_in_graveyard` card
        // activates from its owner's graveyard, where control and ownership coincide.)
        if self.controller_of(source) != player {
            return Err(Reject::CannotActivate);
        }
        // CR 613.1e/701 "loses all abilities": a printed activated ability is suppressed while an
        // ability-removing Aura (Darksteel Mutation) is attached. The Aura's own granted abilities
        // (indices past the printed slice) sit after the removal in CR 613 order, so stay active.
        if index < self.def_of(source).abilities.len() && self.host_loses_all_abilities(source) {
            return Err(Reject::CannotActivate);
        }
        let Some(ability) = self.ability_at(source, index) else {
            return Err(Reject::CannotActivate);
        };
        let Timing::Activated(cost) = ability.timing else {
            return Err(Reject::CannotActivate);
        };
        // A Pacifism-family Aura's "activated abilities can't be activated[, unless they're mana
        // abilities]" restriction (Faith's Fetters, Prison Term; CR 605.3a exempts mana abilities
        // under the `mana_only` axis, nothing exempts under `none`).
        if let Some(restriction) = self.host_activated_ability_restriction(source) {
            let mana_exempt = restriction == AbilityRestriction::ManaAbilitiesOnly
                && ability.effect.is_mana_ability();
            if !mana_exempt {
                return Err(Reject::CannotActivate);
            }
        }
        // A Class's "Level N" ability (CR 717.2 — "Gain the next level as a sorcery"): activatable
        // only to gain the *next* level, so its source must currently sit at exactly `level - 1`
        // (each level gained exactly once). Supersedes the `min_level` gate below (level-up
        // abilities carry `min_level` 0). A non-permanent source can't be a Class, so reject.
        if let Effect::LevelUp { level } = ability.effect {
            if perm.is_none_or(|p| p.level + 1 != level) {
                return Err(Reject::CannotActivate);
            }
        } else if perm.is_some_and(|p| ability.min_level > p.level) {
            // A level-gated activated ability functions only at or above its level (CR 717.5).
            return Err(Reject::CannotActivate);
        }
        // An activation restriction expressed as a condition ("Activate only if you control five
        // or more lands" — Temple of the False God). CR 602.5b: an unmet restriction makes the
        // activation illegal. Reuses the intervening-if evaluator the triggers share.
        if let Some(condition) = ability.condition
            && !self.condition_holds(condition, TriggerContext::of(player))
        {
            return Err(Reject::CannotActivate);
        }
        // A loyalty ability (CR 606): sorcery-speed, at most one per planeswalker per turn,
        // and a `−N` may only be activated with loyalty ≥ N. The loyalty change is its whole
        // cost — the checks below are for mana/tap/life costs it doesn't carry.
        if let Some(loyalty_cost) = cost.loyalty {
            // No pool card gives a graveyard-functional card a loyalty ability — a planeswalker
            // has no graveyard-functional printing — so this path always has a live permanent.
            let Some(perm) = perm else {
                return Err(Reject::CannotActivate);
            };
            if !self.can_take_sorcery_speed_action(player) {
                return Err(Reject::WrongTiming);
            }
            if perm.loyalty_activated || perm.loyalty + loyalty_cost < 0 {
                return Err(Reject::CannotActivate);
            }
            return Ok((ability, cost));
        }
        // Equip (CR 702.6e) is sorcery-speed. (Its creature-you-control target is a chosen
        // input, checked by the caller.)
        if matches!(ability.effect, Effect::Equip) && !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::WrongTiming);
        }
        // A tap cost requires an untapped permanent that isn't summoning sick (haste aside) — and,
        // trivially, a permanent to tap at all (no graveyard-functional card in the pool has a
        // `{T}` graveyard ability).
        if cost.taps_self && (perm.is_none_or(|p| p.tapped) || self.is_sick_without_haste(source)) {
            return Err(Reject::CannotActivate);
        }
        // A life cost (fetchlands' "Pay 1 life", War Room's "pay life equal to the number of
        // colors in your commander's color identity") can only be paid if the player has that
        // much life (CR 119.4).
        if self.life(player) < self.resolve_amount(cost.pay_life, player, source, None, 0) {
            return Err(Reject::CannotActivate);
        }
        // A remove-a-counter cost (CR 118 — Steelbane Hydra's "Remove a +1/+1 counter from this
        // creature"; staff_of_the_storyteller's "remove a story counter") can only be paid if the
        // source has that many counters of the right kind on it (CR 602.2b — an uncompletable
        // cost makes the activation illegal).
        let has_enough = match cost.remove_counters_kind {
            None => self.plus_counters(source) >= cost.remove_counters as i32,
            Some(kind) => self.counters_of_kind(source, kind) >= cost.remove_counters,
        };
        if !has_enough {
            return Err(Reject::CannotActivate);
        }
        // A "mill a card" additional cost (CR 701.13/602.2b — Millikin's "{T}, Mill a card:
        // Add {C}.") can only be paid if the library has that many cards to mill.
        if self.players[player.0 as usize].library.len() < cost.mill_self as usize {
            return Err(Reject::CannotActivate);
        }
        // "Activate only once each turn" (CR 602.2b — an activation restriction; Beledros
        // Witherbloom's untap ability): already activated this turn, so this activation is
        // illegal.
        if cost.once_each_turn
            && self
                .once_per_turn
                .activated
                .iter()
                .any(|&(o, i)| o == source && i == index)
        {
            return Err(Reject::CannotActivate);
        }
        // "Activate only as a sorcery" (CR 602.5b — Ozolith, the Shattered Spire's counter
        // ability): an ordinary activated ability restricted to a legal sorcery-speed moment. (CR 602, CR 113)
        // ponytail: reuses the sorcery-cast timing predicate; no separate "only during your main
        // phase" variant exists because every pool card spelling this restriction means CR 602.5b.
        if cost.sorcery_speed && !self.can_take_sorcery_speed_action(player) {
            return Err(Reject::CannotActivate);
        }
        Ok((ability, cost))
    }

    /// Activate a permanent's activated ability. Mana abilities pay and produce mana
    /// immediately; any other activated ability pays its cost and goes on the stack.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn activate_ability(
        &mut self,
        player: PlayerId,
        object: ObjectId,
        ability_index: usize,
        target: Option<Target>,
        sacrifice: Vec<ObjectId>,
        discard_cost: Vec<ObjectId>,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        let (ability, cost) = self.ability_activation_gate(player, object, ability_index)?;
        // The ability's source's own colors (CR 702.16b) — Nin, the Pain Artist (a UR source)
        // can't target a permanent with protection from red or blue.
        let source_colors = color_identity(self.def_of(object));
        // `ThisPermanent`/`EnchantedCreature` are a fixed reference, not a real choice (CR:
        // these abilities never say "target") — the activator names no target at all, and this
        // resolves it to the ability's own source (or its attachment host) regardless of what
        // the caller passed. `NoLegalTarget` here (e.g. Redemption Arc unattached) rejects the
        // activation outright, mirroring `place_targeted_ability`'s trigger-side handling.
        let target = match ability.effect.target() {
            spec @ (TargetSpec::ThisPermanent | TargetSpec::EnchantedCreature) => {
                // An activated ability carries no {X} (mirrors `run`'s "abilities (CR 602, CR 113)
                // carry no X" for `StackItem::Ability`).
                let legal = self.legal_targets_for(spec, object, player, source_colors, 0);
                match legal.first() {
                    Some(&fixed) => Some(fixed),
                    None => return Err(Reject::IllegalTarget),
                }
            }
            // An ordinary chosen target isn't re-validated at activation (this engine's
            // established posture — see `deekah_grant_unblockable_lets_token_through`, which
            // relies on an illegal target fizzling at resolution, CR 608.2b, rather than
            // rejecting the activation outright); `Game::resolve_top`'s `target_still_legal`
            // re-check is where protection (CR 702.16b) actually filters it, with these same
            // `source_colors`.
            _ => target,
        };
        // A loyalty ability's change (+N / 0 / −N) is paid as a cost before the effect goes
        // on the stack.
        // ponytail: loyalty abilities in the pool carry no mana/tap/sacrifice cost, so only the
        // loyalty change is paid here — fold in the mana/tap path if a hybrid loyalty ability appears.
        if let Some(loyalty_cost) = cost.loyalty {
            let mut events = Vec::new();
            self.push_apply(
                &mut events,
                Event::LoyaltyChanged {
                    object,
                    amount: loyalty_cost,
                },
            );
            self.push_apply(
                &mut events,
                Event::LoyaltyActivated {
                    object,
                    active: true,
                },
            );
            self.push_ability_group(
                player,
                object,
                &[(ability.effect, target)],
                true,
                &mut events,
            );
            return Ok(events);
        }
        // Equip targets a creature you control (CR 702.6e; its timing is gated above).
        if matches!(ability.effect, Effect::Equip) {
            let controls_target_creature = matches!(target, Some(Target::Object(c))
                if self.is_creature_on_battlefield(c) && self.controller_of(c) == player);
            if !controls_target_creature {
                return Err(Reject::IllegalTarget);
            }
        }
        // Resolve the sacrifice cost up front (CR 118.9 — a cost, checked before anything is
        // paid).
        let sacrificed =
            self.validate_sacrifice_cost(player, object, cost.sacrifice, &sacrifice)?;
        // A "discard a card" cost (CR 602.2b — Wild Mongrel's "Discard a card") names exactly
        // `discard_cost` distinct cards currently in the activator's hand; a short, duplicated,
        // or not-in-hand choice rejects the whole activation before any event is applied, same
        // as an illegal sacrifice pick above.
        if discard_cost.len() != cost.discard_cost as usize {
            return Err(Reject::CannotActivate);
        }
        let hand = self.hand_of(player);
        let mut named: Vec<ObjectId> = Vec::with_capacity(discard_cost.len());
        for &id in &discard_cost {
            if named.contains(&id) || !hand.contains(&id) {
                return Err(Reject::CannotActivate);
            }
            named.push(id);
        }
        // "Exile N target cards from an opponent's graveyard" as a targeted additional cost (CR
        // 601.2c/602.2b — Spurnmage Advocate): at least `count` legal graveyard cards must exist
        // to activate at all — an uncompletable targeted cost makes the whole activation illegal,
        // same rule the `discard_cost`/`mill_self` count checks already enforce above/in the gate.
        // The actual cards are named in a follow-up `ChooseActivationCostTargets` pause (below),
        // once every other cost is paid — CR 601.2c's targets, unlike a plain untargeted choice.
        let graveyard_exile_target_legal = (cost.graveyard_exile_target_count > 0).then(|| {
            self.legal_targets_for(
                TargetSpec::CardInGraveyard {
                    whose: GraveyardScope::Opponents,
                    filter: CardFilter::AnyCard,
                },
                object,
                player,
                [false; Color::COUNT],
                0,
            )
        });
        if let Some(legal) = &graveyard_exile_target_legal
            && legal.len() < cost.graveyard_exile_target_count as usize
        {
            return Err(Reject::CannotActivate);
        }
        // Read the sacrificed creature's power/toughness *before* it's sacrificed (Dina, Soul
        // Steeper's "+X/+0"; Dina, Essence Brewer's "gain X life and put X counters", X = that
        // power; Miren, the Moaning Well's "gain life equal to the sacrificed creature's
        // toughness") — by the time the ability resolves off the stack, the creature is gone and
        // there's nothing left to read `Amount::SourcePower`/`SourceToughness` from. No pool card
        // combines `SourcePower`/`SourceToughness` with a multi-creature sacrifice cost, so the
        // first sacrificed creature is the only one that can matter here.
        let effect = match sacrificed.first() {
            Some(&id) => {
                contextualize_sacrifice_effect(ability.effect, self.power(id), self.toughness(id))
            }
            None => ability.effect,
        };
        // Pay the cost. The mana settles first (auto-tapping lands for a pool shortfall) so an
        // unpayable activation rejects before any other cost event lands; its own source is
        // excluded from the auto-tap plan when the activation already taps it.
        // CR 107.3/601.2b: the chosen `{X}` folds into the mana cost (paid once per `{X}` symbol);
        // a no-`{X}` cost ignores `x`.
        let mut events = Vec::new();
        let exclude = cost.taps_self.then_some(object);
        // No cast spell funds this payment, but a `SpendRestriction::HasX` credit (Elementalist's
        // Palette's "spend only on costs that contain {X}") cares about {X} in *this* cost, not
        // just a spell's — Nin, the Pain Artist's own `{X}{U}{R}` activation qualifies (CR 106.9).
        let characteristics = SpellCharacteristics {
            mana_value: 0,
            has_x: cost.mana.x > 0,
            is_instant_or_sorcery: false,
        };
        self.settle_payment(
            player,
            cost.mana.with_x(x),
            exclude,
            Some(characteristics),
            &mut events,
        )
        .map_err(|_| Reject::CannotActivate)?;
        // Read what the payment actually spent right off its trailing `ManaSpent`, before any
        // later push dilutes the tail — Illusionary Mask's "the mana you spent on {X}" (CR 107.3)
        // reads this multiset when the ability resolves.
        let spent_mana = spent_counts_from(&events);
        if cost.once_each_turn {
            self.push_apply(
                &mut events,
                Event::AbilityActivatedThisTurn {
                    object,
                    ability_index,
                },
            );
        }
        if cost.taps_self {
            self.push_apply(&mut events, Event::Tapped { object });
        }
        let pay_life = self.resolve_amount(cost.pay_life, player, object, None, 0);
        if pay_life > 0 {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -pay_life,
                    source: Some(object),
                },
            );
        }
        if cost.remove_counters > 0 {
            let removal = -(cost.remove_counters as i32);
            let event = match cost.remove_counters_kind {
                None => Event::CountersPlaced {
                    object,
                    count: removal,
                    source_name: self.def_of(object).name,
                },
                Some(kind) => Event::KindCountersPlaced {
                    object,
                    kind,
                    count: removal,
                },
            };
            self.push_apply(&mut events, event);
        }
        // "Mill a card" as part of the cost (CR 701.13 — Millikin's "{T}, Mill a card: Add
        // {C}."). Gated payable above (`ability_activation_gate`), so this always mills exactly
        // `mill_self` cards.
        if cost.mill_self > 0 {
            for event in self.mill_events(player, cost.mill_self as u32) {
                self.push_apply(&mut events, event);
            }
        }
        // "Discard a card" as part of the cost (CR 602.2b — Wild Mongrel's "Discard a card").
        // Validated payable above; routes through the normal discard events (mirroring `cast`'s
        // own additional-discard-cost loop) so "whenever you discard a card" watchers fire off
        // it, same as any other discard.
        for &id in &named {
            let card = self.next_object_id();
            let def = self.def_of(id);
            self.push_apply(&mut events, Event::MovedToGraveyard { card, from: id });
            self.push_apply(
                &mut events,
                Event::Discarded {
                    card,
                    from: id,
                    def,
                    player,
                },
            );
        }
        // "Return this to its owner's hand" as part of the cost (Rootha, Mercurial Artist's
        // "Return Rootha to its owner's hand"). A token ceases to exist instead of reaching a
        // hand (CR 111.7) — same branch `Effect::ReturnToHand` takes for a targeted bounce.
        if cost.return_self {
            let perm = self.permanent(object);
            let event = if perm.token {
                Event::TokenCeasedToExist {
                    token: object,
                    controller: perm.owner,
                    def: perm.def,
                }
            } else {
                Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                }
            };
            self.push_apply(&mut events, event);
        }
        // "Exile this artifact" as part of the cost (Perpetual Timepiece's "Exile this
        // artifact"). A token ceases to exist instead of reaching exile (CR 111.7) — the same
        // fork `return_self` takes above.
        if cost.exile_self {
            let perm = self.permanent(object);
            let event = if perm.token {
                Event::TokenCeasedToExist {
                    token: object,
                    controller: perm.owner,
                    def: perm.def,
                }
            } else {
                Event::MovedToExile {
                    card: self.next_object_id(),
                    from: object,
                }
            };
            self.push_apply(&mut events, event);
        }
        // A painland/Talisman self-damage rider ("This land deals 1 damage to you") — an effect,
        // not a cost, so it's applied here rather than gating activation. Its ability is always a
        // mana ability, resolving instantly below, so the placement is indistinguishable. (CR 605, CR 113)
        if cost.self_damage > 0 {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -(cost.self_damage as i32),
                    source: Some(object),
                },
            );
        }
        self.pay_sacrifice_events(player, &sacrificed, &mut events);

        if effect.is_mana_ability() {
            // "Add N mana of any one color" (CR 106.4 — Lotus Field, Kami of Whispered Hopes):
            // the controller names the color as part of resolving the ability. CR 605.3a only
            // exempts a mana ability from the stack, not from choices made while it resolves, so (CR 605, CR 405, CR 113)
            // this pauses on a ChooseManaColor choice instead of resolving straight to mana —
            // the same pending-choice/answer flow every other resolution-time decision uses.
            if let Effect::AddMana {
                mana,
                repeat,
                single_color: true,
                ..
            } = effect
            {
                let repeat = self.resolve_count(repeat, player, object, target, 0);
                let amount = (mana.any as u32).saturating_mul(repeat).min(u8::MAX as u32) as u8;
                if amount > 0 {
                    pending::raise(
                        self,
                        pending::ChoiceRequest::ChooseManaColor {
                            player,
                            source: object,
                            amount,
                        },
                    );
                    return Ok(events);
                }
            }
            // Mana abilities resolve immediately — no stack, no priority change. Goes through (CR 117, CR 405, CR 113)
            // `Game::run` so a composite mana ability (CR 605, CR 113) (Brass Infiniscope's
            // `Sequence` of `AddMana` + `ScheduleNextCastTrigger` — CR 605.3a doesn't require a
            // mana ability to do *only* that) resolves both steps in order instead of hitting
            // the private mint's `Sequence => unreachable!()` guard; behavior-preserving for the
            // common bare-`AddMana` case, which still falls through `run`'s catch-all to mint+apply.
            self.run(
                effect,
                ResolveCtx {
                    controller: player,
                    source: object,
                    target,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            );
            // Mana-provenance side-channel (Study Hall / Path of Ancestry / Opal Palace): tag each
            // credit this ability just produced against its own source, so a later spell-cast
            // payment can fire `object`'s `Trigger::SpendManaToCast`. Done here (not in the pure
            // `&self` mint `AddMana` arm) because this is where the source id and the
            // resolved `ManaAdded` events coexist.
            if effect.tracks_mana_provenance() {
                for event in &events {
                    let Event::ManaAdded {
                        player: p,
                        mana,
                        amount,
                        ..
                    } = *event
                    else {
                        continue;
                    };
                    if p != player {
                        continue;
                    }
                    for _ in 0..amount {
                        self.players[player.0 as usize]
                            .mana_provenance
                            .push((object, mana));
                    }
                }
            }
            // Fertile Ground / Mirari's Wake fire off an `add_mana` land's tap too (a painland,
            // filter land, or any land whose mana is an explicit ability rather than `produces`
            // sugar). The helper's land-guard skips a non-land mana source (Sol Ring, a dork).
            // ponytail: a `single_color` land (Lotus Field) returns above before reaching here, so
            // its tap fires no watch — no pool land is both `single_color` and a watch host; move
            // this call above that early return if one ever is.
            self.land_tapped_for_mana(object, player, &mut events);
            return Ok(events);
        }

        // Every other cost is paid; a targeted graveyard-exile cost (Spurnmage Advocate) still
        // needs its own targets named. Pause on a `ChooseActivationCostTargets` choice — answering
        // it exiles them and pushes this already-fixed `(effect, target)` onto the stack, mirroring
        // `push_ability_group_with_x` below exactly (see `Game::choose_activation_cost_targets_answer`).
        if let Some(legal) = graveyard_exile_target_legal {
            pending::raise_choice(
                self,
                PendingChoice::ChooseActivationCostTargets {
                    player,
                    source: object,
                    effect,
                    target,
                    x,
                    spent_mana,
                    legal,
                    count: cost.graveyard_exile_target_count,
                },
            );
            return Ok(events);
        }

        // A two-target activated ability (Zedruu's donation, CR 601.2c): its first target (the
        // permanent you control) rides `target` — validate it against the effect's own spec (CR
        // 602.2b), then pause to choose the second, independent target clause (the recipient
        // opponent) before the ability hits the stack, threading the activation's `{X}`/spent
        // mana and `activated` through the placement.
        if self
            .ability_second_target_clause(effect, object, player)
            .is_some()
        {
            let spec = effect.target();
            let legal = self.legal_targets_for(spec, object, player, source_colors, x);
            if !target.is_some_and(|t| legal.contains(&t)) {
                return Err(Reject::IllegalTarget);
            }
            self.place_ability_second_clause(
                player,
                object,
                effect,
                target,
                x,
                spent_mana,
                true,
                &mut events,
            );
            return Ok(events);
        }

        // Non-mana activated abilities go on the stack, reusing the trigger placement path,
        // threading the chosen `{X}` so `Amount::X` resolves against it (CR 107.3) and the spent
        // multiset for Illusionary Mask's payability test.
        self.push_ability_group_with_x(
            player,
            object,
            &[(effect, target)],
            x,
            spent_mana,
            true,
            &mut events,
        );
        // CR 707.10: "Whenever you … activate an ability, if that ability's activation cost
        // contains {X}, copy that ability" (Unbound Flourishing). Fire the watch off the just-
        // placed ability — the copy trigger lands above it (CR 603.3b) and, on resolution, mints
        // a copy carrying its effect/target/X. Gated on the *cost* containing `{X}`
        // (`cost.mana.x > 0`), not the chosen value, so an `{X}` = 0 activation still copies
        // (CR 707.10 copies value 0). No pool card is a non-`{X}` activated-ability copy consumer,
        // so a fixed-cost activation queues nothing.
        if cost.mana.x > 0 {
            self.queue_activate_ability_triggers(player, object);
        }
        Ok(events)
    }
}

/// The colors of mana actually spent by the payment [`Game::settle_payment`] just appended to
/// `events` (CR 106.9 — Court Hussar's "unless {W} was spent to cast it"), read off its trailing
/// [`Event::ManaSpent`]. `settle_payment` always pushes that event last on success (any tap
/// events it needs come first), so this only ever runs immediately after such a call, before any
/// later push dilutes the tail.
fn spent_colors_from(events: &[Event]) -> [bool; Color::COUNT] {
    match events.last() {
        Some(Event::ManaSpent { mana, .. }) => mana.colors_spent(),
        // unreachable: see this fn's doc — `settle_payment` always ends with `ManaSpent`.
        _ => [false; Color::COUNT],
    }
}

/// The per-kind counts of mana actually spent by the payment [`Game::settle_payment`] just
/// appended to `events` ([`ManaPool::spent_counts`] — Illusionary Mask's CR 107.3 "the mana you
/// spent on {X}" test), read off its trailing [`Event::ManaSpent`] exactly the way
/// [`spent_colors_from`] reads the colors (and under the same always-last guarantee).
fn spent_counts_from(events: &[Event]) -> [u8; 6] {
    match events.last() {
        Some(Event::ManaSpent { mana, .. }) => mana.spent_counts(),
        // unreachable: see `spent_colors_from`'s doc — `settle_payment` always ends with `ManaSpent`.
        _ => [0; 6],
    }
}

/// Build an [`Event::SpellDamageDivided`] from `(target, amount)` pairs (CR 601.2d), splitting
/// the object shares into the [`DamageAssignment`] combat also uses and the player shares into the
/// parallel `Copy` player array (a player isn't an object). Shared by the single-target autofill
/// (`maybe_begin_damage_division`) and the multi-target divide answer (`divide_spell_damage`).
pub(crate) fn spell_damage_divided(spell: ObjectId, pairs: &[(Target, i32)]) -> Event {
    let objects: Vec<(ObjectId, i32)> = pairs
        .iter()
        .filter_map(|&(t, amt)| t.object_id().map(|id| (id, amt)))
        .collect();
    let mut players = [None; MAX_TARGETS];
    let player_pairs = pairs.iter().filter_map(|&(t, amt)| match t {
        Target::Player(p) => Some((p, amt)),
        Target::Object(_) => None,
    });
    for (slot, pair) in players.iter_mut().zip(player_pairs) {
        *slot = Some(pair);
    }
    Event::SpellDamageDivided {
        spell,
        assignment: DamageAssignment::from_pairs(&objects),
        players,
    }
}
