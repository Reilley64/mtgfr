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
        match choice {
            PendingChoice::OrderTriggers {
                source, effects, ..
            } => {
                // CR 603.3d: each ability's target is chosen as *it* goes on the stack, in the
                // chosen order — so re-queue the ordered abilities as N one-ability
                // `TriggerGroup`s (front of the queue, in `order`) and place them one at a time
                // through the normal path (`Game::place_pending_triggers` /
                // `place_targeted_ability`), the same way a delayed or reflexive trigger rides
                // that path. `expanded: true`: this group already ran its trigger-doubling pass
                // before `OrderTriggers` was raised (see `place_pending_triggers`), so it must
                // not double again.
                for (offset, &i) in order.iter().enumerate() {
                    self.pending_trigger_groups.insert(
                        offset,
                        TriggerGroup {
                            controller: player,
                            source,
                            abilities: vec![Ability {
                                // Fabricated placeholder — the real `Ability` (with its own
                                // `optional`/`cost`/`condition`) was already consumed building
                                // `effects` above; `place_pending_triggers` only reads
                                // `effect`/`optional`/`cost`/`once_each_turn` off this one.
                                timing: Timing::Triggered(Trigger::Upkeep),
                                effect: effects[i],
                                optional: false,
                                min_level: 0,
                                cost: Cost::FREE,
                                condition: None,
                                once_each_turn: false,
                            }],
                            expanded: true,
                        },
                    );
                }
                self.place_pending_triggers(&mut events);
            }
            _ => unreachable!("guarded to ordering choices above"),
        }
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
            return self.choose_spell_targets_answer(spell, clause, min, max, &legal, targets);
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
        }) = self.pending_choice.clone()
        {
            return self.choose_ability_targets_answer(
                player, chooser, source, effect, first, min, max, &legal, targets,
            );
        }
        let Some(PendingChoice::ChooseTarget {
            source,
            effect,
            legal,
            optional,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // "Up to one" (`optional`): an empty answer declines. CR 601.2c treats choosing zero of
        // "up to one" as a complete, legal choice, so the ability still goes on the stack with no
        // target chosen — but only when that accomplishes something
        // (`Effect::has_target_independent_step`, e.g. Kinetic Ooze's X-threshold riders, which
        // don't need the declined destroy target). An ability whose every step needs that same
        // target (Killian's tap-and-goad) drops outright, same as before.
        if targets.is_empty() && optional {
            self.finish_answer();
            if !effect.has_target_independent_step() {
                return Ok(Vec::new());
            }
            let mut events = Vec::new();
            // The first clause was declined; a second target clause (Kinetic Ooze's X≥10 doubling)
            // is still chosen at placement (CR 603.3d) before the ability goes on the stack.
            self.place_ability_second_clause(player, source, effect, None, &mut events);
            return Ok(events);
        }
        let [target] = targets[..] else {
            return Err(Reject::IllegalChoice);
        };
        if !legal.contains(&target) {
            return Err(Reject::IllegalTarget);
        }

        self.finish_answer();
        let mut events = Vec::new();
        // A fight's second creature (see `Effect::Fight`) is chosen mid-resolution, not placed
        // as a new ability — apply the mutual damage directly instead of going back on the stack.
        if let Effect::Fight { enemy, .. } = effect {
            let your_creature = expect_object_target(Some(target), "a fight's chosen creature");
            let enemy_creature =
                expect_object_target(enemy, "a fight's pre-resolved opponent creature");
            self.fight(your_creature, enemy_creature, &mut events);
        } else if let Effect::MoveCounters {
            from, all_kinds, ..
        } = effect
        {
            // A move-counters effect's destination (see `Effect::MoveCounters`) is chosen
            // mid-resolution, same "act directly, don't go back on the stack" treatment Fight
            // gets above.
            let from = expect_object_target(from, "a move-counters effect's stashed source");
            let to = expect_object_target(Some(target), "a move-counters effect's destination");
            self.move_counters(from, to, all_kinds, &mut events);
        } else if let Effect::Demonstrate { spell } = effect {
            // The chosen opponent (CR 702.147a) also gets a copy — mint the controller's own
            // copy now (with its usual CR 707.10c retarget); the opponent's copy is deferred to
            // `Game::resume_deferred_sequence` so it mints only after the controller's copy's own
            // retarget choice (if any) is fully answered — two different copies' controllers can't
            // share one `mint_spell_copies` call (see `Effect::Demonstrate`'s doc).
            let Target::Player(opponent) = target else {
                return Err(Reject::IllegalTarget);
            };
            self.mint_spell_copies(Amount::Fixed(1), player, spell, None, 0, &mut events);
            self.pending_demonstrate_opponent_copy = Some((opponent, spell));
        } else {
            // The first clause's target is chosen; a second target clause (Kinetic Ooze's X≥10
            // doubling) is chosen next at placement (CR 603.3d) before the ability hits the stack.
            self.place_ability_second_clause(player, source, effect, Some(target), &mut events);
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
            &mut events,
        );
        Ok(events)
    }

    /// Validate and record a multi-target spell's chosen targets (CR 601.2c): between `min` and
    /// `max` of them, all distinct, all drawn from `legal`. Writes them onto the spell via
    /// [`Event::SpellTargetsChosen`]; rejects (leaving the choice pending) otherwise.
    fn choose_spell_targets_answer(
        &mut self,
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
        let player = self.spell(spell).controller;
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
        self.advance_spell_target_clauses(spell, clause as usize + 1, player, &mut events);
        Ok(events)
    }
}
