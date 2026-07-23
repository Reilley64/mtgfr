//! Target / order / proliferate / phase-out answers.

use crate::*;

impl Game {
    pub(crate) fn choose_order(
        &mut self,
        player: PlayerId,
        order: Vec<usize>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(choice) = self.take_pending_choice() else {
            return Err(Reject::ChoicePending);
        };
        let is_ordering = matches!(choice, PendingChoice::OrderTriggers { .. });
        if !is_ordering || player != choice.player() || !is_permutation(&order, choice.len()) {
            self.restore_pause(choice); // put it back; the answer was invalid
            return Err(Reject::IllegalChoice);
        }

        let mut events = Vec::new();
        // CR 603.3d: each ability's target is chosen as *it* goes on the stack, in the chosen
        // order — so re-split the still-queued group into N one-ability `TriggerGroup`s (front
        // of the queue, in `order`) and place them one at a time through the normal path
        // (`Game::place_pending_triggers` / `place_targeted_ability`), the same way a delayed or
        // reflexive trigger rides that path. The group was deliberately left on the queue while
        // the choice was pending (see `place_pending_triggers`), so each ability arrives with its
        // own `optional` / `cost` / `condition` intact. `expanded: true`: this group already ran
        // its trigger-doubling pass before `OrderTriggers` was raised, so it must not double again.
        let Some(group) = self
            .pending_trigger_groups
            .first()
            .filter(|g| g.abilities.len() == choice.len())
            .cloned()
        else {
            self.restore_pause(choice);
            return Err(Reject::IllegalChoice);
        };
        self.pending_trigger_groups.remove(0);
        for (offset, &i) in order.iter().enumerate() {
            self.pending_trigger_groups.insert(
                offset,
                TriggerGroup {
                    controller: group.controller,
                    source: group.source,
                    abilities: vec![group.abilities[i]],
                    expanded: true,
                },
            );
        }
        self.place_pending_triggers(&mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseTarget`] (a single triggered-ability target) or a
    /// [`PendingChoice::ChooseSpellTargets`] (a multi-target spell's N targets, CR 601.2c).
    pub(crate) fn choose_targets(
        &mut self,
        player: PlayerId,
        targets: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        if let Some(PendingChoice::ChooseSpellTargets {
            spell,
            min,
            max,
            legal,
            clause,
            ..
        }) = self.pending_choice.clone()
        {
            return self
                .choose_spell_targets_answer(player, spell, clause, min, max, &legal, targets);
        }
        if let Some(PendingChoice::ChooseSplittingOpponent {
            player: controller,
            source,
            legal,
            then,
        }) = self.pending_choice.clone()
        {
            return self.choose_splitting_opponent_answer(controller, source, legal, then, targets);
        }
        if let Some(PendingChoice::ChooseAbilityTargets {
            player: chooser,
            source,
            effect,
            target: first,
            min,
            max,
            legal,
            x,
            spent_mana,
            activated,
        }) = self.pending_choice.clone()
        {
            return self.choose_ability_targets_answer(
                player, chooser, source, effect, first, min, max, &legal, x, spent_mana, activated,
                targets,
            );
        }
        if let Some(PendingChoice::ChooseActivationCostTargets {
            player: activator,
            source,
            effect,
            target,
            x,
            spent_mana,
            legal,
            count,
        }) = self.pending_choice.clone()
        {
            return self.choose_activation_cost_targets_answer(
                player, activator, source, effect, target, x, spent_mana, &legal, count, targets,
            );
        }
        let Some(PendingChoice::ChooseTarget {
            source,
            effect,
            legal,
            count,
            x: activation_x,
            activated,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // "Up to N" (`count.min == 0`): an empty answer declines. CR 601.2c treats choosing zero
        // of "up to N" as a complete, legal choice, so the ability still goes on the stack with no
        // target chosen — but only when that accomplishes something
        // (`Effect::has_target_independent_step`, e.g. Kinetic Ooze's X-threshold riders, which
        // don't need the declined destroy target). An ability whose every step needs that same
        // target (Killian's tap-and-goad) drops outright, same as before.
        if targets.is_empty() && count.min == 0 {
            self.finish_answer();
            if !effect.has_target_independent_step() {
                return Ok(Vec::new());
            }
            let mut events = Vec::new();
            // The first clause was declined; a second target clause (Kinetic Ooze's X≥10 doubling)
            // is still chosen at placement (CR 603.3d) before the ability goes on the stack.
            self.place_ability_second_clause(
                player,
                source,
                effect,
                None,
                activation_x,
                [0; 6],
                activated,
                &mut events,
            );
            return Ok(events);
        }
        // Between `count.min` and `count.max` distinct targets, all drawn from `legal` (CR
        // 601.2c) — the same multi-target validation `choose_ability_targets_answer`/
        // `choose_spell_targets_answer` apply to their own counted choices.
        if !(count.min as usize..=count.max as usize).contains(&targets.len()) {
            return Err(Reject::IllegalChoice);
        }
        for (i, t) in targets.iter().enumerate() {
            if targets[..i].contains(t) || !legal.contains(t) {
                return Err(Reject::IllegalTarget);
            }
        }

        self.finish_answer();
        let mut events = Vec::new();
        let [target] = targets[..] else {
            // More than one target chosen for the ability's own (first) clause (CR 601.2c —
            // Numot, the Devastator's "destroy up to two target lands"): push one ability
            // instance per target, each independently re-checked for legality at its own
            // resolution (CR 608.2b) — the triggered-ability twin of how a multi-target *spell*
            // is decomposed (`multi_target_steps`).
            // ponytail: N separate `StackItem::Ability` entries rather than one ability holding a
            // `TargetList` for this (first) clause — fine while nothing in the pool "counters
            // target ability" against a multi-target trigger; give the primary clause a real
            // `TargetList` (like `targets_second` already is) if one ever does.
            let abilities: Vec<(Effect, Option<Target>)> =
                targets.iter().map(|&t| (effect, Some(t))).collect();
            self.push_ability_group_with_x(
                player,
                source,
                &abilities,
                activation_x,
                [0; 6],
                activated,
                &mut events,
            );
            return Ok(events);
        };
        // A fight's second creature (see `Effect::Misc(MiscEffect::Fight)`) is chosen mid-resolution, not placed
        // as a new ability — apply the mutual damage directly instead of going back on the stack.
        if let Effect::Misc(MiscEffect::Fight { enemy, .. }) = effect {
            let your_creature = expect_object_target(Some(target), "a fight's chosen creature");
            let enemy_creature =
                expect_object_target(enemy, "a fight's pre-resolved opponent creature");
            self.fight(your_creature, enemy_creature, &mut events);
        } else if let Effect::Counters(CountersEffect::MoveCounters {
            from, all_kinds, ..
        }) = effect
        {
            // A move-counters effect's destination (see `Effect::Counters(CountersEffect::MoveCounters)`) is chosen
            // mid-resolution, same "act directly, don't go back on the stack" treatment Fight
            // gets above.
            let from = expect_object_target(from, "a move-counters effect's stashed source");
            let to = expect_object_target(Some(target), "a move-counters effect's destination");
            self.move_counters(from, to, all_kinds, &mut events);
        } else if let Effect::Copy(CopyEffect::Demonstrate { spell }) = effect {
            // The chosen opponent (CR 702.147a) also gets a copy — mint the controller's own
            // copy now (with its usual CR 707.10c retarget); the opponent's copy is deferred to
            // `Game::resume_deferred_sequence` so it mints only after the controller's copy's own
            // retarget choice (if any) is fully answered — two different copies' controllers can't
            // share one `mint_spell_copies` call (see `Effect::Copy(CopyEffect::Demonstrate)`'s doc).
            let Target::Player(opponent) = target else {
                return Err(Reject::IllegalTarget);
            };
            self.mint_spell_copies(Amount::Fixed(1), player, spell, None, 0, &mut events);
            self.resume.demonstrate_opponent_copy = Some((opponent, spell));
        } else {
            // The first clause's target is chosen; a second target clause (Kinetic Ooze's X≥10
            // doubling) is chosen next at placement (CR 603.3d) before the ability hits the stack.
            self.place_ability_second_clause(
                player,
                source,
                effect,
                Some(target),
                activation_x,
                [0; 6],
                activated,
                &mut events,
            );
        }
        Ok(events)
    }

    /// Every counter-removal event for `object` — one [`Event::CountersPlaced`] (negative) if it
    /// has any +1/+1 counters, plus one [`Event::KindCountersPlaced`] (negative) per named kind
    /// present — and the total count removed. Shared by `RemoveAllCountersThenDraw` (remove and
    /// draw) and [`Self::move_counters`] (remove, then re-place on the destination).
    pub(crate) fn answer_proliferate(
        &mut self,
        player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::Proliferate {
            source,
            options,
            remaining,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // Each option is chosen at most once (CR 701.27: a *set* of permanents/players, not a
        // multiset — the same permanent can't be picked twice for a double proliferation).
        let distinct = chosen
            .iter()
            .enumerate()
            .all(|(i, id)| !chosen[..i].contains(id));
        if !distinct || chosen.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &chosen {
            let plus = self.permanent(id).plus_counters;
            if plus > 0 {
                let n = self.counters_after_replacements(id, 1);
                if n > 0 {
                    self.push_apply(
                        &mut events,
                        Event::CountersPlaced {
                            object: id,
                            count: n,
                            source_name: "",
                        },
                    );
                }
            }
            for &kind in CounterKind::ALL.iter() {
                if self.permanent(id).kind_counters[kind as usize] > 0 {
                    self.push_apply(
                        &mut events,
                        Event::KindCountersPlaced {
                            object: id,
                            kind,
                            count: 1,
                        },
                    );
                }
            }
        }
        pending::raise(
            self,
            pending::ChoiceRequest::Proliferate {
                player,
                source,
                remaining,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PhaseOut`]: every chosen creature (and everything attached to it)
    /// phases out (CR 702.26). An empty answer is a legal "phase out nothing" (CR "any number ...
    /// target"). Reuses [`Self::answer_proliferate`]'s distinct/subset validation.
    pub(crate) fn answer_phase_out(
        &mut self,
        player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PhaseOut {
            player: chooser,
            options,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // A set, not a multiset — each chosen creature at most once, all from the offered set.
        let distinct = chosen
            .iter()
            .enumerate()
            .all(|(i, id)| !chosen[..i].contains(id));
        if player != chooser || !distinct || chosen.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &chosen {
            self.push_apply(&mut events, Event::PhasedOut { object: id });
        }
        Ok(events)
    }

    /// Validate and record a triggered ability's *second* independent target clause (CR 603.3d —
    /// Kinetic Ooze's X≥10 "double ... any number of other target creatures"), then push the
    /// assembled ability — its first-clause `first` target and this clause's chosen `targets` — onto
    /// the stack. Between `min` and `max` distinct targets, all drawn from `legal` (CR 601.2c).
    #[allow(clippy::too_many_arguments)]
    fn choose_ability_targets_answer(
        &mut self,
        player: PlayerId,
        chooser: PlayerId,
        source: ObjectId,
        effect: Effect,
        first: Option<Target>,
        min: u8,
        max: u8,
        legal: &[Target],
        x: u32,
        spent_mana: [u8; 6],
        activated: bool,
        targets: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        if player != chooser || !(min as usize..=max as usize).contains(&targets.len()) {
            return Err(Reject::IllegalChoice);
        }
        // Distinct (CR 601.2c: "the same target can't be chosen twice") and all legal.
        for (i, t) in targets.iter().enumerate() {
            if targets[..i].contains(t) || !legal.contains(t) {
                return Err(Reject::IllegalTarget);
            }
        }
        self.finish_answer();
        let mut events = Vec::new();
        self.push_ability_with_targets(
            player,
            source,
            effect,
            first,
            TargetList::from_targets(&targets),
            x,
            spent_mana,
            activated,
            &mut events,
        );
        Ok(events)
    }

    /// Validate and pay a [`PendingChoice::ChooseActivationCostTargets`] answer (Spurnmage
    /// Advocate's targeted graveyard-exile cost, CR 601.2c/602.2b): exactly `count` distinct legal
    /// targets, all from the *same* opponent's graveyard (CR: "an opponent's graveyard" is one
    /// opponent, not a mix). Exiles them, then pushes the activation's already-fixed
    /// `(effect, target)` onto the stack — the same shape [`Game::activate_ability`]'s ordinary
    /// (no-second-cost) path pushes directly.
    #[allow(clippy::too_many_arguments)]
    fn choose_activation_cost_targets_answer(
        &mut self,
        player: PlayerId,
        activator: PlayerId,
        source: ObjectId,
        effect: Effect,
        target: Option<Target>,
        x: u32,
        spent_mana: [u8; 6],
        legal: &[Target],
        count: u8,
        chosen: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        if player != activator || chosen.len() != count as usize {
            return Err(Reject::IllegalChoice);
        }
        // Distinct, legal, and all owned by the same graveyard's owner (CR 601.2c's "an
        // opponent's graveyard" names one opponent, not any mix of opponents).
        let mut owner = None;
        for (i, &t) in chosen.iter().enumerate() {
            if chosen[..i].contains(&t) || !legal.contains(&t) {
                return Err(Reject::IllegalTarget);
            }
            let Some(id) = t.object_id() else {
                return Err(Reject::IllegalTarget);
            };
            let card_owner = self.owner_of(id);
            if *owner.get_or_insert(card_owner) != card_owner {
                return Err(Reject::IllegalTarget);
            }
        }
        self.finish_answer();
        let mut events = Vec::new();
        for &t in &chosen {
            let card = expect_object_target(Some(t), "a graveyard-exile activation cost target");
            self.push_apply(
                &mut events,
                Event::MovedToExile {
                    card: self.next_object_id(),
                    from: card,
                },
            );
        }
        self.push_ability_group_with_x(
            player,
            source,
            &[(effect, target)],
            x,
            spent_mana,
            true,
            &mut events,
        );
        Ok(events)
    }

    /// Validate and record a multi-target spell's chosen targets (CR 601.2c): between `min` and
    /// `max` of them, all distinct, all drawn from `legal`. Writes them onto the spell via
    /// [`Event::SpellTargetsChosen`]; rejects (leaving the choice pending) otherwise. `chooser` is
    /// the player who just answered — chains into the next independent clause (if any) as the same
    /// chooser, legality still anchored on the spell's own controller (see
    /// [`Game::choose_spell_targets`]'s `anchor`/`chooser` doc).
    #[allow(clippy::too_many_arguments)]
    fn choose_spell_targets_answer(
        &mut self,
        chooser: PlayerId,
        spell: ObjectId,
        clause: u8,
        min: u8,
        max: u8,
        legal: &[Target],
        targets: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        if !(min as usize..=max as usize).contains(&targets.len()) {
            return Err(Reject::IllegalChoice);
        }
        // Distinct (CR 601.2c: "the same target can't be chosen twice") and all legal.
        for (i, t) in targets.iter().enumerate() {
            if targets[..i].contains(t) || !legal.contains(t) {
                return Err(Reject::IllegalTarget);
            }
        }
        let anchor = self.spell(spell).controller;
        self.finish_answer();
        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::SpellTargetsChosen {
                spell,
                targets: TargetList::from_targets(&targets),
                clause,
            },
        );
        // Chain into the next independent target clause, if any, before the CR 601.2d split runs.
        self.advance_spell_target_clauses(spell, clause as usize + 1, anchor, chooser, &mut events);
        Ok(events)
    }
}
