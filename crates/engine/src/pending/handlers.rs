//! Pending-choice handlers and dig-loop kickoff helpers.
//!
//! Targets, modes, scry, search, edicts, damage assignment, and trigger ordering
//! ([`PendingChoice`]). Closely tied to CR 601.2c (choosing targets) and resolution
//! pauses under CR 608. The submit seam is [`super::answer`] / [`super::forced`];
//! pause sites use [`super::raise`] / [`super::raise_choice`]. Dig-loop kickoffs emit
//! prep events then raise. Deferred / gaps: see the parent module docs and
//! `docs/FIDELITY_BACKLOG.md`.

use crate::*;

impl Game {
    /// Resolve a pending ordering choice: `order` is a permutation of the offered items.
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
    pub(crate) fn remove_all_counters_events(&self, object: ObjectId) -> (Vec<Event>, i32) {
        let mut events = Vec::new();
        let mut removed = 0;
        let plus = self.permanent(object).plus_counters;
        if plus > 0 {
            events.push(Event::CountersPlaced {
                object,
                count: -plus,
                source_name: self.def_of(object).name,
            });
            removed += plus;
        }
        for &kind in CounterKind::ALL.iter() {
            let count = self.permanent(object).kind_counters[kind as usize] as i32;
            if count > 0 {
                events.push(Event::KindCountersPlaced {
                    object,
                    kind,
                    count: -count,
                });
                removed += count;
            }
        }
        (events, removed)
    }

    /// Move counters from `from` onto `to` ([`Effect::MoveCounters`]): +1/+1 counters always
    /// move, through the same replaceable-placement pipeline the destination's own +1/+1
    /// doublers would apply to any other "put a counter" (CR 614); `all_kinds` also moves every
    /// named kind present, raw (named kinds bypass that pipeline everywhere else in the pool —
    /// see [`Effect::EntersWithCounters`]'s doc).
    fn move_counters(
        &mut self,
        from: ObjectId,
        to: ObjectId,
        all_kinds: bool,
        events: &mut Vec<Event>,
    ) {
        let plus = self.permanent(from).plus_counters;
        if plus > 0 {
            self.push_apply(
                events,
                Event::CountersPlaced {
                    object: from,
                    count: -plus,
                    source_name: self.def_of(from).name,
                },
            );
            let n = self.counters_after_replacements(to, plus);
            if n > 0 {
                self.push_apply(
                    events,
                    Event::CountersPlaced {
                        object: to,
                        count: n,
                        source_name: self.def_of(from).name,
                    },
                );
            }
        }
        if !all_kinds {
            return;
        }
        for &kind in CounterKind::ALL.iter() {
            let count = self.permanent(from).kind_counters[kind as usize] as i32;
            if count <= 0 {
                continue;
            }
            self.push_apply(
                events,
                Event::KindCountersPlaced {
                    object: from,
                    kind,
                    count: -count,
                },
            );
            self.push_apply(
                events,
                Event::KindCountersPlaced {
                    object: to,
                    kind,
                    count,
                },
            );
        }
    }

    /// Move +1/+1 counters from `from` onto several destinations at once
    /// ([`Effect::MoveCounters`]'s `distributed` mode, CR 601.2d): one combined removal from
    /// `from` for the summed total, then each destination's placement through the same
    /// replaceable-counters pipeline (CR 614) [`Self::move_counters`] uses for its single-
    /// destination case. `assignment` pairs were already validated (distinct, legal, ≥1 each,
    /// summing to at most the source's live count) by [`Self::divide_moved_counters`].
    fn move_counters_distributed(
        &mut self,
        from: ObjectId,
        assignment: &[(ObjectId, i32)],
        events: &mut Vec<Event>,
    ) {
        let total: i32 = assignment.iter().map(|&(_, n)| n).sum();
        if total == 0 {
            return; // "you may move any number" — declining to move any is a legal no-op.
        }
        self.push_apply(
            events,
            Event::CountersPlaced {
                object: from,
                count: -total,
                source_name: self.def_of(from).name,
            },
        );
        for &(to, n) in assignment {
            let n = self.counters_after_replacements(to, n);
            if n <= 0 {
                continue;
            }
            self.push_apply(
                events,
                Event::CountersPlaced {
                    object: to,
                    count: n,
                    source_name: self.def_of(from).name,
                },
            );
        }
    }

    /// Answer a [`PendingChoice::Proliferate`]: every chosen permanent gets one more counter of
    /// each kind already on it (CR 701.27), through the same replaceable-placement pipeline as
    /// [`Effect::PutCounters`] for +1/+1 (a chosen permanent's own doubler applies here too) and
    /// raw for named kinds (consistent with [`Self::move_counters`]/[`Effect::PutCounters`]'s own
    /// kind split). Then, if this proliferate had more iterations left, re-pauses for the next.
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

    /// Answer a [`PendingChoice::MayYesNo`]: accept the optional trigger (put it on the stack)
    /// or decline (it's simply skipped).
    pub(crate) fn answer_may(&mut self, player: PlayerId, yes: bool) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayYesNo { source, effect, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();
        let mut events = Vec::new();
        if yes {
            // A resolution-time "may copy this spell" rider (`Effect::CopyThisSpell`'s
            // `optional` gate, CR 707.10c — Sevinne's Reclamation) mints inline as part of the
            // still-resolving spell rather than going on the stack as a new triggered ability — (CR 603, CR 405, CR 601)
            // the mandatory storm/Gravestorm mint this mirrors never leaves the stack either.
            if let Effect::CopyThisSpell { count, .. } = effect {
                self.mint_spell_copies(count, player, source, None, 0, &mut events);
            } else if let Effect::Demonstrate { spell } = effect {
                // Demonstrate's controller copy mints only once an opponent is chosen for the
                // second copy (CR 702.147a "choose an opponent to also copy it") — see the
                // `Effect::Demonstrate` branch in `Game::choose_targets`.
                let legal: Vec<Target> = self
                    .legal_targets_for(
                        TargetSpec::OpponentPlayer,
                        spell,
                        player,
                        [false; Color::COUNT],
                        0,
                    )
                    .into_iter()
                    .collect();
                // ponytail: no legal opponent is unreachable in a real (2+ player) Commander game.
                if legal.is_empty() {
                    return Ok(events);
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player,
                        source,
                        effect: Effect::Demonstrate { spell },
                        legal,
                        optional: false,
                    },
                );
            } else if let Effect::TargetPlayerMayDraw { count, .. } = effect {
                // Questing Phelddagrif's blue rider: `player` here is the *targeted* opponent who
                // just answered "yes" (not the ability's own controller, unlike every other arm
                // in this function) — draw them `count` cards directly, no further pause (CR
                // 601.2c: no pay window rides behind this rider).
                let n = self.resolve_count(count, player, source, None, 0);
                let evs = self.draw_events(player, n);
                self.apply_all(&evs);
                events.extend(evs);
            } else if let Effect::MayDrawUnlessPays { cost, caster } = effect {
                // Rhystic Study: `player` (the controller) said they want to draw, so now
                // `caster` (the triggering opponent, baked in by `contextualize_effect`) gets a
                // chance to pay `cost` and stop it — see `Game::pay_or_controller_draws`.
                let caster = caster.expect(
                    "caster baked in by contextualize_effect at CastSpell trigger placement",
                );
                let generic = self.resolve_count(cost, player, source, None, 0);
                pending::raise_choice(
                    self,
                    PendingChoice::PayOrControllerDraws {
                        player: caster,
                        controller: player,
                        cost: Cost {
                            generic: generic as u8,
                            ..Cost::FREE
                        },
                    },
                );
            } else {
                // A targeted "may" (Sun Titan) pauses again to choose its target; a targetless
                // one (Solemn's dies-draw) goes straight on the stack. NoLegalTarget = accepted
                // but nothing to aim at, so it fizzles harmlessly.
                self.place_targeted_ability(player, source, effect, &mut events);
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCost`]: pay the cost to get the optional trigger, or decline.
    /// An unaffordable "pay" leaves the choice pending so the player can still decline.
    pub(crate) fn pay_optional_cost(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayCost {
            source,
            cost,
            effect,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let mut events = Vec::new();
        if !pay {
            self.finish_answer();
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        // A targeted paid trigger pauses to choose its target; a targetless one goes on the stack.
        self.place_targeted_ability(player, source, effect, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCost`] whose `cost` carries a chosen `{X}` (CR 107.3 —
    /// Decree of Justice's "When you cycle this card, you may pay {X}."): pay `cost.with_x(x)`
    /// to get the optional trigger, threading `x` onto the placed ability the same way an
    /// activated ability's own `{X}` cost does (see [`Game::push_ability_group_with_x`]), so its
    /// `Amount::X` reads the chosen value — or decline (`x` ignored). An unaffordable "pay"
    /// leaves the choice pending so the player can still decline, mirroring
    /// [`Game::pay_optional_cost`].
    /// ponytail: targetless only (`push_ability_group_with_x` skips the target-choice dance in
    /// [`Game::place_targeted_ability`]) — no pool card pairs an `{X}`-cost optional trigger with
    /// a target; route through `place_targeted_ability`'s own X-threading path if one ever does.
    pub(crate) fn pay_optional_cost_with_x(
        &mut self,
        player: PlayerId,
        pay: bool,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayCost {
            source,
            cost,
            effect,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let mut events = Vec::new();
        if !pay {
            self.finish_answer();
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall, folding the chosen `{X}`
        // into generic per CR 107.3); unaffordable leaves the choice pending with nothing tapped.
        self.settle_payment(player, cost.with_x(x), None, None, &mut events)?;
        self.finish_answer();
        self.push_ability_group_with_x(
            player,
            source,
            &[(effect, None)],
            x,
            [0; 6],
            false,
            &mut events,
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayOrCounter`]: pay `cost` to save the target spell, or decline
    /// and let it be countered. The mirror image of [`Game::pay_optional_cost`] — same
    /// [`Intent::PayOptionalCost`] shape, opposite default (declining here *does* something: the
    /// counter). An unaffordable "pay" leaves the choice pending so the player can still decline.
    /// ponytail: reuses `PayOptionalCost` rather than a dedicated intent — the wire shape (a bare
    /// pay/decline bool) is identical, and `Game::submit`'s choice gate already routes by the
    /// pending choice's kind, not the intent's.
    pub(crate) fn pay_or_counter(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayOrCounter { cost, spell, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let evs = self.counter_spell(spell);
            self.apply_all(&evs);
            return Ok(evs);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        // Paying leaves the spell on the stack — it resolves normally, untouched.
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayOrControllerDraws`]: `player` (the triggering opponent) pays
    /// `cost` to stop `controller`'s draw, or declines and lets it happen — Rhystic Study's
    /// "unless that player pays {1}". Same [`Intent::PayOptionalCost`] shape and "declining does
    /// something" polarity as [`Game::pay_or_counter`], but the "something" is a draw rather than
    /// a counter.
    pub(crate) fn pay_or_controller_draws(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayOrControllerDraws {
            controller, cost, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let evs = self.draw_events(controller, 1);
            self.apply_all(&evs);
            return Ok(evs);
        }
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        // Paying stops the draw outright — nothing further happens.
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCounteredSpellDestination`] (Hinder's CR 701.5b rider):
    /// `top` puts the already-countered `spell` on top of its owner's library instead of the
    /// bottom. `_player` isn't needed beyond `submit`'s choice-gate actor check (like
    /// [`Game::choose_color`]).
    pub(crate) fn choose_countered_spell_destination(
        &mut self,
        _player: PlayerId,
        top: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCounteredSpellDestination { spell, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::TuckedToLibrary {
                card: self.next_object_id(),
                from: spell,
                to_top: top,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayEchoOrSacrifice`]: pay Echo's cost to keep `source`, or
    /// decline and sacrifice it (CR 702.31d). The permanent-scoped twin of
    /// [`Game::pay_or_counter`] — same [`Intent::PayOptionalCost`] shape and "declining does
    /// something" polarity (there, countering the spell; here, sacrificing the source). An
    /// unaffordable "pay" leaves the choice pending so the player can still decline.
    pub(crate) fn pay_echo(&mut self, player: PlayerId, pay: bool) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayEchoOrSacrifice { source, cost, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut events = Vec::new();
            self.run(
                Effect::SacrificeObject {
                    object: Some(source),
                },
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            );
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        // CR 702.31e: this upkeep is now "since your last upkeep" — echo won't ask again.
        self.permanent_mut(source).echo_unpaid = false;
        self.finish_answer();
        Ok(events)
    }

    /// Answer a [`PendingChoice::SacrificeUnlessPay`]: pay `cost` to keep `source`, or decline
    /// and sacrifice it (CR 701.16). Rupture Spire's own-ETB twin of [`Game::pay_echo`] — same
    /// [`Intent::PayOptionalCost`] shape and polarity, kept as its own handler since it isn't
    /// Echo (see the variant's doc). An unaffordable "pay" leaves the choice pending.
    pub(crate) fn pay_sacrifice_unless(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SacrificeUnlessPay { source, cost, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut events = Vec::new();
            self.run(
                Effect::SacrificeObject {
                    object: Some(source),
                },
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            );
            return Ok(events);
        }
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        Ok(events)
    }

    /// Answer a [`PendingChoice::SacrificeUnlessReturnLand`]: `land` (one of the offered
    /// candidates) returns to its owner's hand and `source` stays; `None` declines and
    /// sacrifices `source` instead (CR 701.16).
    pub(crate) fn return_land_or_sacrifice(
        &mut self,
        player: PlayerId,
        land: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SacrificeUnlessReturnLand {
            source, candidates, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if land.is_some_and(|l| !candidates.contains(&l)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        match land {
            None => self.run(
                Effect::SacrificeObject {
                    object: Some(source),
                },
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            ),
            Some(chosen) => {
                let card = self.next_object_id();
                self.push_apply(&mut events, Event::ReturnedToHand { card, from: chosen });
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::AssignCombatDamage`]: store how the attacker divides its combat
    /// damage among its blockers. The damage itself is dealt in the combat-damage step.
    /// ponytail: validates coverage + totals (non-trample assigns all power to blockers; trample
    /// may leave a remainder for the player); the strict lethal-before-next order rule is skipped. (CR 702, CR 120.3, CR 506)
    pub(crate) fn assign_damage(
        &mut self,
        _player: PlayerId,
        assignment: Vec<(ObjectId, i32)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::AssignCombatDamage {
            attacker, blockers, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let assigned: Vec<ObjectId> = assignment.iter().map(|&(b, _)| b).collect();
        let covers_blockers = assigned.len() == blockers.len()
            && blockers.iter().all(|b| assigned.contains(b))
            && assigned.iter().all(|b| blockers.contains(b));
        let nonneg = assignment.iter().all(|&(_, amt)| amt >= 0);
        let total: i32 = assignment.iter().map(|&(_, amt)| amt).sum();
        let power = self.power(attacker);
        let total_ok = if self.has_keyword(attacker, Keyword::Trample) {
            total <= power
        } else {
            total == power
        };
        if !covers_blockers || !nonneg || !total_ok || assignment.len() > MAX_BLOCKERS {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::CombatDamageDivided {
                attacker,
                assignment: DamageAssignment::from_pairs(&assignment),
            },
        );

        // Chain to the next multi-blocked attacker's division, or hand back priority. (CR 117, CR 402.5, CR 508)
        if let Some((next, blks)) = self.next_undivided_multiblock() {
            self.pause_for(PendingChoice::AssignCombatDamage {
                player: self.active_player,
                attacker: next,
                blockers: blks,
            });
        } else {
            self.consecutive_passes = 0;
            self.priority = self.active_player;
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::DivideSpellDamage`]: settle how a divided-damage spell's total
    /// is split among its already-chosen targets (CR 601.2d — Magma Opus's "4 damage divided as
    /// you choose among any number of targets"). Keyed by [`Target`], not bare object ids: "any
    /// number of targets" may include a *player*, which combat's [`Intent::AssignDamage`] wire
    /// can't name — so this has its own [`Intent::DivideSpellDamage`] wire, branched onto this
    /// handler in `Game::submit`. The object shares flow onto [`Spell::damage_division`] and the
    /// player shares onto [`Spell::damage_division_players`] (see [`spell_damage_divided`]).
    pub(crate) fn divide_spell_damage(
        &mut self,
        _player: PlayerId,
        assignment: Vec<(Target, i32)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DivideSpellDamage {
            spell,
            targets,
            total,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let assigned: Vec<Target> = assignment.iter().map(|&(t, _)| t).collect();
        let covers_targets = assigned.len() == targets.len()
            && targets.iter().all(|t| assigned.contains(t))
            && assigned.iter().all(|t| targets.contains(t));
        // CR 601.2d: each target must receive at least one point of the divided total.
        let each_at_least_one = assignment.iter().all(|&(_, amt)| amt >= 1);
        let sums_to_total = assignment.iter().map(|&(_, amt)| amt).sum::<i32>() == total;
        if !covers_targets || !each_at_least_one || !sums_to_total || assignment.len() > MAX_TARGETS
        {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            crate::cast::spell_damage_divided(spell, &assignment),
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::DivideCounters`]: settle how a divided-counters spell's total
    /// is split among its already-chosen targets (CR 601.2d — Grove's Bounty's "Distribute X
    /// +1/+1 counters among any number of target creatures you control"). Mirrors
    /// [`Self::divide_spell_damage`] — same [`Intent::AssignDamage`] wire shape, same
    /// [`DamageAssignment`] division shape, same at-least-one/sums-to-total validation.
    pub(crate) fn divide_counters(
        &mut self,
        _player: PlayerId,
        assignment: Vec<(ObjectId, i32)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DivideCounters {
            spell,
            targets,
            total,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let assigned: Vec<ObjectId> = assignment.iter().map(|&(t, _)| t).collect();
        let covers_targets = assigned.len() == targets.len()
            && targets.iter().all(|t| assigned.contains(t))
            && assigned.iter().all(|t| targets.contains(t));
        // CR 601.2d: each target must receive at least one of the divided total.
        let each_at_least_one = assignment.iter().all(|&(_, amt)| amt >= 1);
        let sums_to_total = assignment.iter().map(|&(_, amt)| amt).sum::<i32>() == total;
        if !covers_targets
            || !each_at_least_one
            || !sums_to_total
            || assignment.len() > MAX_BLOCKERS
        {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::SpellCountersDivided {
                spell,
                assignment: DamageAssignment::from_pairs(&assignment),
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::DivideMovedCounters`]: distribute up to `cap` of `from`'s +1/+1
    /// counters across any subset of `legal` (CR 601.2d — Forgotten Ancient's "move any number of
    /// +1/+1 counters ... distributed as you choose among any number of target creatures").
    /// Unlike [`Self::divide_counters`]'s fixed-spell-total division, an empty `assignment` is a
    /// legal "move nothing" ("any number" includes zero) and not every offered destination need
    /// be used — only a subset, summing to at most `cap` rather than exactly a fixed total.
    /// Applies the move directly (remove-then-place through the same +1/+1 replacement pipeline
    /// [`Self::move_counters`] uses) rather than deferring onto a still-resolving spell, since a
    /// triggered ability's stack item carries no [`Spell`]-shaped bookkeeping to defer onto.
    pub(crate) fn divide_moved_counters(
        &mut self,
        _player: PlayerId,
        assignment: Vec<(ObjectId, i32)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DivideMovedCounters {
            from, legal, cap, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let assigned: Vec<ObjectId> = assignment.iter().map(|&(t, _)| t).collect();
        let distinct = assigned
            .iter()
            .enumerate()
            .all(|(i, id)| !assigned[..i].contains(id));
        let all_legal = assigned.iter().all(|id| legal.contains(id));
        let each_at_least_one = assignment.iter().all(|&(_, amt)| amt >= 1);
        let total: i32 = assignment.iter().map(|&(_, amt)| amt).sum();
        if !distinct || !all_legal || !each_at_least_one || total > cap {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        self.move_counters_distributed(from, &assignment, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::ArrangeTop`]: keep `top` on top of the library (in this order),
    /// and send `bottom` to the library bottom (scry) or the graveyard (surveil). Their union
    /// must be a partition of the shown cards.
    /// ponytail: kept cards *may* be freely reordered — this honors the answered `top` order
    /// directly (a real permutation), matching CR 701.42's "in any order".
    pub(crate) fn arrange_top(
        &mut self,
        player: PlayerId,
        top: Vec<ObjectId>,
        bottom: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ArrangeTop {
            cards,
            to_graveyard,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if !is_partition(&top, &bottom, &cards) {
            return Err(Reject::IllegalChoice); // not a split of exactly the shown cards
        }
        self.finish_answer();

        // The untouched remainder of the library, below the looked-at cards (captured before any
        // mutation — no other action is legal while the choice is pending, so it's still intact).
        let count = cards.len();
        let rest: Vec<ObjectId> = self.players[player.0 as usize].library[count..].to_vec();

        let mut events = Vec::new();
        if to_graveyard {
            // Surveil: the bottom pile is put into the graveyard — the same library→graveyard
            // zone change as a mill (each mints a fresh graveyard-object id in order).
            let base = self.next_object_id();
            for (i, &from) in bottom.iter().enumerate() {
                let event = Event::Milled {
                    player,
                    card: base + i as u32,
                    from,
                };
                self.apply(&event);
                events.push(event);
            }
        }

        // Rebuild the library's top in the chosen order: kept cards first, then the untouched
        // remainder, then (for scry) the bottom pile at the very bottom.
        // ponytail: library order isn't event-sourced (neither is `shuffle`) — mutate it directly.
        let mut library = top;
        library.extend(rest);
        if !to_graveyard {
            library.extend(bottom);
        }
        self.players[player.0 as usize].library = library;
        Ok(events)
    }

    /// Answer a [`PendingChoice::SelectFromTop`]: move each `selected` card (up to `up_to`, each
    /// offered and matching the choice's `filter`) to `dest`, and put every non-selected looked-at
    /// card into `rest`. Selecting fewer than `up_to` is legal down to `min` (Dig Through Time's
    /// mandatory "put two of them into your hand"; `min: 0` is the "may" default, always legal
    /// down to zero). `min` is bounded by how many cards were actually looked at (CR 120-style "as
    /// many as possible" on a short library). The remainder of the library below the looked-at
    /// cards is untouched.
    pub(crate) fn select_from_top(
        &mut self,
        player: PlayerId,
        selected: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SelectFromTop {
            cards,
            filter,
            up_to,
            min,
            dest,
            dest_tapped,
            rest,
            mv_budget,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if selected.len() > up_to as usize {
            return Err(Reject::IllegalChoice); // may take at most `up_to`
        }
        if selected.len() < (min as usize).min(cards.len()) {
            return Err(Reject::IllegalChoice); // must take at least `min` (clamped to the look)
        }
        // A total-mana-value budget (Ao, the Dawn Sky's "total mana value 4 or less") caps the
        // summed mana value of the selected cards, independent of the count bounds above.
        if let Some(budget) = mv_budget {
            let total: u32 = selected
                .iter()
                .map(|&id| self.def_of(id).mana_value())
                .sum();
            if total > budget {
                return Err(Reject::IllegalChoice);
            }
        }
        for (i, &id) in selected.iter().enumerate() {
            // Each selected card must be one of the looked-at cards, match the filter, and appear
            // at most once.
            if !cards.contains(&id)
                || !filter.matches(self.def_of(id))
                || selected[..i].contains(&id)
            {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        // Selected cards leave the library for `dest` (each move retain-removes its id from the
        // library, leaving the non-selected looked-at cards — the `bottomed` pile below — still
        // sitting in place at the top).
        let mut events = Vec::new();
        let mut deployed: Option<ObjectId> = None;
        for &from in &selected {
            let event = match dest {
                TopDest::Hand => Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from,
                    card: self.def_of(from),
                },
                TopDest::Battlefield => {
                    let permanent = self.next_object_id();
                    deployed = Some(permanent);
                    Event::SearchedToBattlefield {
                        permanent,
                        from,
                        controller: player,
                        tapped: dest_tapped,
                    }
                }
            };
            self.push_apply(&mut events, event);
        }
        // An Aura among the deployed permanents may need a host chosen (CR 303.4f). Scoped to a
        // lone deployed permanent (Armored Skyhunter's `up_to = 1`) — a multi-permanent batch
        // that happens to include an Aura (no pool card does today) is left to the existing
        // hostless-Aura state-based action, same as before this pause existed. (CR 704, CR 303.4)
        if selected.len() == 1
            && let Some(permanent) = deployed
        {
            self.maybe_pause_attach_deployed_aura(permanent, player);
        }

        // Every non-selected looked-at card goes to `rest`, in a random order (CR "in a random
        // order" — the same PRNG-shuffle-then-bottom idiom the `reveal_until` family uses).
        let bottomed: Vec<ObjectId> = cards
            .iter()
            .copied()
            .filter(|c| !selected.contains(c))
            .collect();
        match rest {
            RestDest::Bottom => self.bottom_pile_in_library(player, &bottomed, &mut events),
            RestDest::Hand => {
                for from in bottomed {
                    let event = Event::SearchedToHand {
                        player,
                        object: self.next_object_id(),
                        from,
                        card: self.def_of(from),
                    };
                    self.push_apply(&mut events, event);
                }
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::DistributeTop`]: route `to_hand` to hand, `to_bottom` to the
    /// library bottom, and `to_exile_may_play` into exile with permission to play this turn — the
    /// same impulse-draw events [`Game::exile_top_may_play_events`] mints, one card at a time. The
    /// three lists must each match the choice's slot size, be drawn from the looked-at `cards`,
    /// and share no card.
    pub(crate) fn distribute_top(
        &mut self,
        player: PlayerId,
        to_hand: Vec<ObjectId>,
        to_bottom: Vec<ObjectId>,
        to_exile_may_play: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DistributeTop {
            cards,
            to_hand: hand_slot,
            to_bottom: bottom_slot,
            to_exile_may_play: exile_slot,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if to_hand.len() != hand_slot as usize
            || to_bottom.len() != bottom_slot as usize
            || to_exile_may_play.len() != exile_slot as usize
        {
            return Err(Reject::IllegalChoice);
        }
        let mut assigned: Vec<ObjectId> = Vec::new();
        for &id in to_hand.iter().chain(&to_bottom).chain(&to_exile_may_play) {
            // Each routed card must be one of the looked-at cards and appear in exactly one slot.
            if !cards.contains(&id) || assigned.contains(&id) {
                return Err(Reject::IllegalChoice);
            }
            assigned.push(id);
        }
        self.finish_answer();

        // The untouched remainder of the library, below the looked-at cards (captured before any
        // mutation — no other action is legal while the choice is pending, so it's still intact).
        let count = cards.len();
        let remainder: Vec<ObjectId> = self.players[player.0 as usize].library[count..].to_vec();

        let mut events = Vec::new();
        for &from in &to_hand {
            let event = Event::SearchedToHand {
                player,
                object: self.next_object_id(),
                from,
                card: self.def_of(from),
            };
            self.push_apply(&mut events, event);
        }
        for &from in &to_exile_may_play {
            let event = Event::ExiledFromLibraryMayPlay {
                player,
                card: self.next_object_id(),
                from,
                until_next_turn: false,
            };
            self.push_apply(&mut events, event);
        }

        // ponytail: Expressive Iteration's "bottom" slot is always exactly one card (no printed
        // "random order," unlike `LookAtTop`'s rest pile) — a direct rebuild is faithful as-is;
        // revisit only if a second `DistributeTop` card bottoms more than one card at once.
        let mut library = remainder;
        library.extend(to_bottom);
        self.players[player.0 as usize].library = library;

        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseManaColor`] (CR 106.4's "add N mana of any one color" —
    /// Lotus Field, Kami of Whispered Hopes): the pending `amount` is added as one `color` credit
    /// — no `Mana::Any` involved, so the credits can't later split across colors at payment time.
    pub(crate) fn choose_mana_color(
        &mut self,
        player: PlayerId,
        color: Color,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseManaColor { amount, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::ManaAdded {
                player,
                mana: Mana::Color(color),
                amount,
                persist: false,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCreatureType`] (CR 614.12/700.9-style "as ~ enters,
    /// choose a creature type" — Patchwork Banner): `subtype` must name one of the pending
    /// choice's offered `options`; the matching entry (not the wire string itself) is stored on
    /// `source`, keeping [`Permanent::chosen_subtype`] a leaked `&'static str` like every other
    /// subtype field. `_player` isn't needed beyond `submit`'s choice-gate actor check — unlike
    /// [`Game::choose_mana_color`], nothing here is scoped by player.
    pub(crate) fn choose_creature_type(
        &mut self,
        _player: PlayerId,
        subtype: String,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCreatureType {
            source, options, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let Some(&chosen) = options.iter().find(|&&t| t == subtype) else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::CreatureTypeChosen {
                object: source,
                subtype: chosen,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseColor`] (CR 614.12/700.9-style "as ~ enters, choose a
    /// color" — Flickering Ward): store the chosen `color` on `source`
    /// ([`Permanent::chosen_color`]). Any of the five colors is a legal answer (no game-state
    /// legality to violate), so `color` is taken as given. `_player` isn't needed beyond `submit`'s
    /// choice-gate actor check.
    pub(crate) fn choose_color(
        &mut self,
        _player: PlayerId,
        color: Color,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseColor { source, .. }) = self.pending_choice.clone() else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::ColorChosen {
                object: source,
                color,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ShuffleFromGraveyard`]: shuffle up to `max` (`0` = unbounded) of
    /// the offered `candidates` from `owner`'s graveyard into `owner`'s library, via the same
    /// [`Event::TuckedToLibrary`] zone move [`Effect::TuckFromGraveyard`] uses, then shuffle that
    /// library (CR 701.19-style mandatory shuffle after cards enter it).
    pub(crate) fn shuffle_from_graveyard(
        &mut self,
        // Already validated against the pending choice's answerer by `Game::submit`'s generic
        // choice-actor check; the graveyard/library affected is `owner` (from the pending
        // choice), read below — this may differ from the answering player (Quandrix Command).
        _player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ShuffleFromGraveyard {
            candidates,
            owner,
            max,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if max != 0 && chosen.len() as u32 > max {
            return Err(Reject::IllegalChoice);
        }
        for (i, &id) in chosen.iter().enumerate() {
            if !candidates.contains(&id) || chosen[..i].contains(&id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &from in &chosen {
            self.push_apply(
                &mut events,
                Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from,
                    to_top: false,
                },
            );
        }
        self.push_apply(&mut events, Event::LibraryShuffled { player: owner });
        Ok(events)
    }

    /// Answer a [`PendingChoice::SearchLibrary`]: move the chosen card (one of the offered
    /// matches) to its destination, or fail to find (`choice = None`). A found pick with more
    /// finds remaining and matches still on offer re-pauses for the next pick instead of
    /// shuffling (CR 701.19f — an "up to N" search shuffles once, after the last pick);
    /// otherwise (search exhausted, no matches left, or a fail-to-find) the library is shuffled
    /// and the search ends. When `overflow` is set (Cultivate), the re-pause routes every find
    /// *after this one* to `overflow` instead of `dest` — the first find still lands on `dest`.
    pub(crate) fn search_library(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SearchLibrary {
            matches,
            dest,
            tapped,
            remaining,
            overflow,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // A found card must be one of the offered matches; `None` (fail to find) is always legal.
        if choice.is_some_and(|c| !matches.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(from) = choice else {
            // Fail to find ends the search outright (CR 701.19c is always legal).
            self.shuffle(player);
            return Ok(events);
        };
        let event = match dest {
            SearchDest::Hand => Event::SearchedToHand {
                player,
                object: self.next_object_id(),
                from,
                card: self.def_of(from),
            },
            SearchDest::Battlefield => Event::SearchedToBattlefield {
                permanent: self.next_object_id(),
                from,
                controller: player,
                tapped,
            },
            // Enlightened Tutor / Sterling Grove: the found card is revealed in place (CR
            // 701.30) — it hasn't left the library yet, so no zone-move event here; the shuffle
            // and top-placement below finish the job.
            SearchDest::LibraryTop => Event::RevealedTopOfLibrary {
                player,
                card: from,
                def: self.def_of(from),
            },
        };
        self.push_apply(&mut events, event);

        let remaining = remaining.saturating_sub(1);
        let still_matching: Vec<ObjectId> = matches.into_iter().filter(|&id| id != from).collect();
        if remaining > 0 && !still_matching.is_empty() {
            // Every find after the first routes to `overflow` if the search has one (Cultivate:
            // first pick lands on `dest` = battlefield tapped, every later pick on `overflow` =
            // hand); a search with no overflow (Land Tax) keeps re-pausing on the same `dest`.
            self.pause_for(PendingChoice::SearchLibrary {
                player,
                matches: still_matching,
                dest: overflow.unwrap_or(dest),
                tapped,
                remaining,
                overflow,
            });
            return Ok(events);
        }
        // Always shuffle at the true end of the search (CR 701.19). ponytail: library order
        // isn't event-sourced (like scry / `shuffle`) — mutate it directly.
        if dest == SearchDest::LibraryTop {
            // "…then shuffle and put that card on top" — `from` never left the library, so put
            // it back on top after the shuffle instead of shuffling it in with everything else.
            self.shuffle_then_put_on_top(player, from);
        } else {
            self.shuffle(player);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PutLandFromHand`]: put the chosen land (one of the offered
    /// candidates) onto the battlefield, or decline (`choice = None`).
    pub(crate) fn put_land_from_hand(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PutLandFromHand {
            candidates, tapped, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(from) = choice {
            self.push_apply(
                &mut events,
                Event::PutOntoBattlefieldFromHand {
                    permanent: self.next_object_id(),
                    from,
                    controller: player,
                    tapped,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::CastCreatureFaceDown`]: cast the chosen hand creature (one of the
    /// offered candidates) face down as a 2/2 creature spell (CR 708.2) without paying its mana
    /// cost, or decline (`choice = None`).
    pub(crate) fn cast_creature_face_down(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::CastCreatureFaceDown { candidates, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            // No mana was spent casting it (cast "without paying its mana cost"), so no colors.
            // Masked (CR 615): Illusionary Mask's face-down creature turns face up when it would
            // assign or deal damage, be dealt damage, or become tapped.
            self.push_face_down_spell_cast(player, card, [false; Color::COUNT], true, &mut events);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseMode`]: resolve the chosen mode of a "choose one" triggered
    /// ability ([`Effect::ChooseOne`]) through the ordinary resolution pipeline, carrying the
    /// trigger's own `source`/`target`/`x` context. The chosen sub-effect may itself pause.
    pub(crate) fn answer_choose_mode(
        &mut self,
        player: PlayerId,
        mode: usize,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseMode {
            source,
            target,
            x,
            modes,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if mode >= modes.len() {
            return Err(Reject::IllegalMode);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.run(
            modes[mode],
            ResolveCtx {
                controller: player,
                source,
                target,
                targets_second: TargetList::default(),
                x,
                spent_mana: [0; 6],
            },
            &mut events,
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseTriggerModes`]: a modal *triggered* ability's "choose N"
    /// (CR 700.2) — `modes` names `choose` distinct (printed-mode index, chosen Player target)
    /// pairs. An empty `modes` is legal only when the choice is `optional` (CR "you may") and
    /// drops the whole ability with no events. Otherwise: exactly `choose` distinct mode indices,
    /// each in range and paired with a target legal for that mode's own [`Effect::target`], and
    /// every chosen target a pairwise-distinct player (Shadrix Silverquill's "each mode must
    /// target a different player").
    /// ponytail: the pairwise-distinct-player rule is enforced unconditionally here — see
    /// [`PendingChoice::ChooseTriggerModes`]'s doc.
    pub(crate) fn answer_choose_trigger_modes(
        &mut self,
        player: PlayerId,
        modes: Vec<(usize, Option<Target>)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseTriggerModes {
            source,
            modes: mode_effects,
            choose,
            optional,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if modes.is_empty() {
            if !optional {
                return Err(Reject::IllegalChoice);
            }
            self.finish_answer();
            return Ok(Vec::new());
        }
        if modes.len() != choose as usize {
            return Err(Reject::IllegalMode);
        }
        let def = self.def_of(source);
        let x = self.ability_source_x(source);
        let mut seen_modes: Vec<usize> = Vec::new();
        let mut seen_players: Vec<PlayerId> = Vec::new();
        let mut resolved: Vec<(Effect, Option<Target>)> = Vec::new();
        for (m, target) in modes {
            let (Some(&effect), false) = (mode_effects.get(m), seen_modes.contains(&m)) else {
                return Err(Reject::IllegalMode);
            };
            seen_modes.push(m);
            let spec = effect.target();
            if spec == TargetSpec::None {
                if target.is_some() {
                    return Err(Reject::IllegalTarget);
                }
                resolved.push((effect, None));
                continue;
            }
            let Some(Target::Player(chosen_player)) = target else {
                return Err(Reject::IllegalTarget);
            };
            if seen_players.contains(&chosen_player)
                || !self
                    .legal_targets_for(spec, source, player, color_identity(def), x)
                    .contains(&target.expect("checked Some above"))
            {
                return Err(Reject::IllegalTarget);
            }
            seen_players.push(chosen_player);
            resolved.push((effect, target));
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.push_ability_group(player, source, &resolved, false, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseExiledWithCard`]: put the chosen card (one of the offered
    /// candidates) into its owner's graveyard, then create a Treasure if it's a land or a 2/2
    /// creature token otherwise (CR 406.3), or decline (`choice = None`).
    pub(crate) fn choose_exiled_with_card(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledWithCard {
            source, candidates, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            let is_land = matches!(self.def_of(card).kind, CardKind::Land { .. });
            let graveyard_event = self.graveyard_or_command(card, self.next_object_id());
            self.push_apply(&mut events, graveyard_event);
            self.push_apply(
                &mut events,
                Event::CardExiledWithSourceLeftExile {
                    source,
                    object: card,
                },
            );
            let token_event = Event::TokenCreated {
                token: self.next_object_id(),
                controller: player,
                // ponytail: the real token is a black Rogue (CR would set color + subtype); token
                // color/creature-subtype isn't modeled yet (the #10 gap) — a plain colorless,
                // subtype-less body stands in either way.
                def: if is_land {
                    treasure_token()
                } else {
                    rogue_token_stub()
                },
            };
            self.push_apply(&mut events, token_event);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseExiledWithCardToCast`]: grant the free-cast permission
    /// (CR 118.5, "without paying its mana cost") for the chosen card, or decline
    /// (`choice = None`). Unlike [`Game::choose_exiled_with_card`], the chosen card stays in the
    /// exiled-with pile — this only grants a permission, it doesn't cash the card out.
    pub(crate) fn choose_exiled_with_card_to_cast(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledWithCardToCast { candidates, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
            // CR 614.6 replacement rider — see `PlayPermissions::stack_object_bottoms_library_on_leave`.
            self.push_apply(
                &mut events,
                Event::CastFromExileFreeBottomsLibraryOnLeave { card },
            );
        }
        Ok(events)
    }

    /// Resolve [`Effect::ExileTopCastMatchingFree`] (Herald of Amity's dig): exile the top
    /// `count` cards of `controller`'s library face-up (public, CR 701.17), then raise a
    /// choose-up-to-one over the exiled cards matching `filter`. A short library exiles only
    /// what's there (CR 120-style "as many as possible"); an empty library raises no choice (a
    /// harmless no-op, mirroring empty-library ArrangeTop). No exiled card matching `filter`
    /// also raises no choice — there's nothing to offer — but "put the rest on the bottom" still
    /// has to run, so the whole batch is bottomed immediately instead.
    pub(crate) fn exile_top_cast_matching_free(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        count: u32,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let top: Vec<ObjectId> = self.players[controller.0 as usize]
            .library
            .iter()
            .take(count as usize)
            .copied()
            .collect();
        if top.is_empty() {
            return; // nothing to exile — legal no-op.
        }
        let exiled: Vec<ObjectId> = top
            .iter()
            .map(|&from| {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player: controller,
                        card,
                        from,
                        face_down: false,
                    },
                );
                card
            })
            .collect();
        let candidates: Vec<ObjectId> = exiled
            .iter()
            .copied()
            .filter(|&id| filter.matches(self.def_of(id)))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledDigToCastFree {
                player: controller,
                source,
                candidates,
                exiled: exiled.clone(),
            },
        );
        if !self.resolution_is_paused() {
            self.bottom_exiled_dig(&exiled, events);
        }
    }

    /// Answer a [`PendingChoice::ChooseExiledDigToCastFree`]: grant the free-cast permission
    /// (CR 118.5) for the chosen card — it stays in exile — or decline (`choice = None`). Either
    /// way, every other card in the exiled batch (non-matching, or simply not chosen) goes to the
    /// bottom of the library (CR "put the rest on the bottom of your library").
    pub(crate) fn choose_exiled_dig_to_cast_free(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledDigToCastFree {
            candidates, exiled, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
        }
        let rest: Vec<ObjectId> = exiled
            .into_iter()
            .filter(|&id| Some(id) != choice)
            .collect();
        self.bottom_exiled_dig(&rest, &mut events);
        Ok(events)
    }

    /// Put every exiled-dig card in `cards` on the bottom of its owner's library in a random
    /// order (CR "in a random order" — shared by Herald of Amity's dig and Cascade). The order is
    /// randomized with the injected splitmix PRNG (Fisher-Yates), keeping the engine deterministic
    /// and pure — no `rand`. Shared by [`Self::exile_top_cast_matching_free`]'s /
    /// [`Self::cascade`]'s no-hit fast paths and [`Self::choose_exiled_dig_to_cast_free`]'s
    /// answer.
    pub(crate) fn bottom_exiled_dig(&mut self, cards: &[ObjectId], events: &mut Vec<Event>) {
        let mut order = cards.to_vec();
        for i in (1..order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for from in order {
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::TuckedToLibrary {
                    card,
                    from,
                    to_top: false,
                },
            );
        }
    }

    /// Put every card in `cards` (already `player`'s own library cards) on the bottom of that
    /// library in a random order (CR "in a random order" — shared by Songbirds' Blessing's
    /// [`Self::reveal_until_may_deploy`] and Creative Technique's
    /// [`Self::reveal_until_exile_cast_free`]). Same Fisher-Yates-with-the-injected-PRNG
    /// shuffle as [`Self::bottom_exiled_dig`], but each card is already in the library — a
    /// same-zone reorder via [`Event::PutOnBottomOfLibrary`], not a zone change.
    pub(crate) fn bottom_pile_in_library(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        let mut order = cards.to_vec();
        for i in (1..order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for card in order {
            self.push_apply(events, Event::PutOnBottomOfLibrary { player, card });
        }
    }

    /// The opponent who makes an "an opponent chooses" decision on `controller`'s behalf (Plargg
    /// and Nassari's nonland pick): the next living player reached walking turn order forward
    /// from `controller`.
    /// ponytail: hardcodes the next opponent in turn order rather than the controller's own pick
    /// (the same approximation [`Self::choose_splitting_opponent`] used to carry for
    /// Abstract Performance and Fact or Fiction, before their shared chooser fixed it) — migrate
    /// Plargg and Nassari to that chooser if a pool card ever needs "an opponent" here to be a
    /// genuine choice. `None` only if `controller` is the sole living player left (unreachable in
    /// a real game).
    pub(crate) fn next_opponent_in_turn_order(&self, controller: PlayerId) -> Option<PlayerId> {
        let n = self.players.len();
        (1..n)
            .map(|i| PlayerId(((controller.0 as usize + i) % n) as u8))
            .find(|&p| !self.players[p.0 as usize].lost)
    }

    /// Exile the top `n` cards of `player`'s library into a pile, returning the new exile-object
    /// ids top-to-bottom. Face-up (public, CR 701.17) unless `face_down` (CR 701.9 — Abstract
    /// Performance's first pile, hidden from every viewer but `player` while it awaits the
    /// opponent's pick; see [`Card::face_down`]). Applied immediately, so a second call reads the
    /// library's new top (Abstract Performance's two consecutive four-card piles).
    fn exile_top_into_pile(
        &mut self,
        player: PlayerId,
        n: usize,
        face_down: bool,
        events: &mut Vec<Event>,
    ) -> Vec<ObjectId> {
        let top: Vec<ObjectId> = self.players[player.0 as usize]
            .library
            .iter()
            .take(n)
            .copied()
            .collect();
        top.into_iter()
            .map(|from| {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player,
                        card,
                        from,
                        face_down,
                    },
                );
                card
            })
            .collect()
    }

    /// Route the cards in `exiled` that weren't `chosen` (granted the free cast) once a
    /// [`PendingChoice::ChooseExiledToCastFree`] is answered: to their owner's hand if
    /// `rest_to_hand` (Abstract Performance's "put the rest into your hand"), else leave them in
    /// exile (Plargg and Nassari's uncast cards stay exiled). A chosen card stays in exile until
    /// it's actually cast under the permission.
    fn route_exiled_rest(
        &mut self,
        exiled: &[ObjectId],
        chosen: &[ObjectId],
        rest_to_hand: bool,
        events: &mut Vec<Event>,
    ) {
        if !rest_to_hand {
            return;
        }
        for &from in exiled {
            if chosen.contains(&from) {
                continue;
            }
            self.push_apply(
                events,
                Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from,
                },
            );
        }
    }

    /// Offer `controller` up to `count` free casts (CR 118.5) over the castable (nonland) cards in
    /// `exiled`, raising [`ChoiceRequest::ChooseExiledToCastFree`]. With no castable card
    /// there's nothing to offer, so the rest routes immediately. Shared by Abstract Performance
    /// (`count = 1`, rest to hand) and Plargg and Nassari (`count = 2`, rest stays exiled).
    pub(crate) fn choose_exiled_to_cast_free_pile(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        exiled: Vec<ObjectId>,
        count: u8,
        rest_to_hand: bool,
        events: &mut Vec<Event>,
    ) {
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledToCastFree {
                player: controller,
                source,
                exiled: exiled.clone(),
                count,
                rest_to_hand,
            },
        );
        if !self.resolution_is_paused() {
            self.route_exiled_rest(&exiled, &[], rest_to_hand, events);
        }
    }

    /// Answer a [`PendingChoice::ChooseExiledToCastFree`]: grant the free-cast permission (CR
    /// 118.5) to each `chosen` card (up to `count`, all distinct candidates), then route the rest.
    pub(crate) fn choose_exiled_to_cast_free(
        &mut self,
        player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledToCastFree {
            candidates,
            exiled,
            count,
            rest_to_hand,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let distinct = chosen
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        if chosen.len() > count as usize
            || distinct != chosen.len()
            || chosen.iter().any(|c| !candidates.contains(c))
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &card in &chosen {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
        }
        self.route_exiled_rest(&exiled, &chosen, rest_to_hand, &mut events);
        Ok(events)
    }

    /// Resolve Dance with Calamity's push-your-luck loop
    /// ([`Effect::ExileTopUntilStopCastFreeUnderBudget`]): raise a first
    /// [`ChoiceRequest::DanceExileMore`] over an empty exile pile. An already-empty library has
    /// nothing to exile, so it resolves the payoff straight away (a zero tally — nothing to cast).
    pub(crate) fn dance_with_calamity(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        budget: u32,
        events: &mut Vec<Event>,
    ) {
        if self.players[controller.0 as usize].library.is_empty() {
            self.finish_dance(controller, source, Vec::new(), 0, budget, events);
            return;
        }
        pending::raise(
            self,
            pending::ChoiceRequest::DanceExileMore {
                player: controller,
                source,
                exiled: Vec::new(),
                total_mv: 0,
                budget,
            },
        );
    }

    /// Answer a [`PendingChoice::DanceExileMore`]: on `yes`, exile the top card of the caster's
    /// library face-up (CR 701.17, public) and add its mana value to the running tally, then
    /// re-raise (unless the library is now empty). On `no` — or once the library empties — stop and
    /// resolve the payoff ([`Self::finish_dance`]).
    pub(crate) fn dance_exile_more(
        &mut self,
        player: PlayerId,
        yes: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DanceExileMore {
            source,
            mut exiled,
            mut total_mv,
            budget,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        if yes && let Some(&from) = self.players[player.0 as usize].library.first() {
            let card = self.next_object_id();
            total_mv += self.def_of(from).mana_value();
            self.push_apply(
                &mut events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player,
                    card,
                    from,
                    face_down: false,
                },
            );
            exiled.push(card);
        }
        // Keep offering as long as the caster wants more and the library still has a card to exile.
        if yes && !self.players[player.0 as usize].library.is_empty() {
            pending::raise(
                self,
                pending::ChoiceRequest::DanceExileMore {
                    player,
                    source,
                    exiled,
                    total_mv,
                    budget,
                },
            );
            return Ok(events);
        }
        self.finish_dance(player, source, exiled, total_mv, budget, &mut events);
        Ok(events)
    }

    /// Resolve Dance with Calamity's payoff once the exile loop stops: if `total_mv <= budget` the
    /// caster may cast any number of the exiled (nonland) cards for free (CR 118.5) — raising
    /// [`ChoiceRequest::ChooseExiledToCastFree`] over the whole pile, uncast cards staying exiled.
    /// On a bust (`total_mv > budget`) nothing is offered and every exiled card stays exiled (a
    /// bust never returns them).
    fn finish_dance(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        exiled: Vec<ObjectId>,
        total_mv: u32,
        budget: u32,
        events: &mut Vec<Event>,
    ) {
        if total_mv > budget {
            return; // bust — the exiled cards stay exiled with no free-cast permission.
        }
        // "any number" — the cap is the whole exiled pile (u8-bounded; a legal Dance can never
        // exile more than a handful of nonland cards under a 13-MV budget).
        let count = exiled.len().min(u8::MAX as usize) as u8;
        self.choose_exiled_to_cast_free_pile(controller, source, exiled, count, false, events);
    }

    /// Resolve Abstract Performance ([`Effect::OpponentSplitsExilePiles`]): exile the top four
    /// (CR 701.9 face-down — hidden from every viewer but `controller`) then the next four
    /// (face-up) of `controller`'s library into two piles, then hand off to the shared
    /// [`Self::choose_splitting_opponent`] chooser to pick who picks a pile.
    pub(crate) fn opponent_splits_exile_piles(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let pile_a = self.exile_top_into_pile(controller, 4, true, events);
        let pile_b = self.exile_top_into_pile(controller, 4, false, events);
        self.choose_splitting_opponent(
            controller,
            source,
            SplittingContinuation::ExilePiles { pile_a, pile_b },
        );
    }

    /// Resolve Fact or Fiction ([`Effect::RevealTopSplitPiles`]): reveal the top five of
    /// `controller`'s library (all public, CR 701.16; a short library reveals only what's there,
    /// CR 120.3 "as many as possible" — an empty library reveals nothing and raises no pause),
    /// then hand off to the shared [`Self::choose_splitting_opponent`] chooser to pick who
    /// partitions them.
    pub(crate) fn reveal_top_split_piles(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let revealed: Vec<ObjectId> = self.players[controller.0 as usize]
            .library
            .iter()
            .take(5)
            .copied()
            .collect();
        for &card in &revealed {
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card,
                    def: self.def_of(card),
                },
            );
        }
        if revealed.is_empty() {
            return; // an empty library reveals nothing — no pile to split.
        }
        self.choose_splitting_opponent(
            controller,
            source,
            SplittingContinuation::Partition { revealed },
        );
    }

    /// The shared "an opponent ..." chooser for [`Effect::OpponentSplitsExilePiles`] and
    /// [`Effect::RevealTopSplitPiles`]: with more than one opponent alive, `controller` picks
    /// which one on a [`ChoiceRequest::ChooseSplittingOpponent`]; with at most one, resume
    /// immediately (the "single-legal-choice" collapse — no real choice to offer). `then` carries
    /// the split data already computed, so it's ready the instant the opponent is known.
    fn choose_splitting_opponent(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        then: SplittingContinuation,
    ) {
        let legal: Vec<PlayerId> = self.living_players().filter(|&p| p != controller).collect();
        match legal.as_slice() {
            [] => {} // ponytail: no opponent to choose or split — unreachable in a real game.
            [only] => self.resume_splitting_opponent(*only, controller, source, then),
            _ => pending::raise(
                self,
                pending::ChoiceRequest::ChooseSplittingOpponent {
                    player: controller,
                    source,
                    legal,
                    then,
                },
            ),
        }
    }

    /// Answer a [`PendingChoice::ChooseSplittingOpponent`]: `opponent` must be one of the choice's
    /// `legal` candidates. Resumes via [`Self::resume_splitting_opponent`].
    fn choose_splitting_opponent_answer(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        legal: Vec<PlayerId>,
        then: SplittingContinuation,
        targets: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        let [Target::Player(opponent)] = targets[..] else {
            return Err(Reject::IllegalChoice);
        };
        if !legal.contains(&opponent) {
            return Err(Reject::IllegalTarget);
        }
        self.finish_answer();
        self.resume_splitting_opponent(opponent, controller, source, then);
        Ok(Vec::new())
    }

    /// Resume [`Self::choose_splitting_opponent`] once the splitting opponent is known:
    /// raise `opponent` on whichever pause `then` names.
    fn resume_splitting_opponent(
        &mut self,
        opponent: PlayerId,
        controller: PlayerId,
        source: ObjectId,
        then: SplittingContinuation,
    ) {
        match then {
            SplittingContinuation::ExilePiles { pile_a, pile_b } => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::OpponentChoosesPile {
                        player: opponent,
                        controller,
                        source,
                        pile_a,
                        pile_b,
                    },
                );
            }
            SplittingContinuation::Partition { revealed } => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::PartitionRevealed {
                        player: opponent,
                        controller,
                        source,
                        revealed,
                    },
                );
            }
        }
    }

    /// Answer a [`PendingChoice::PartitionRevealed`] (Fact or Fiction): `pile_a` names the subset
    /// of `revealed` the chosen opponent puts into pile A (reusing
    /// [`Intent::ChooseSacrifices`]'s "name the subset" wire shape); everything else in `revealed`
    /// forms pile B. Either may be empty. Pauses `controller` on a
    /// [`PendingChoice::ChoosePileForHand`] to pick which pile to keep.
    pub(crate) fn partition_revealed(
        &mut self,
        _player: PlayerId,
        pile_a: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PartitionRevealed {
            controller,
            source,
            revealed,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        for (i, id) in pile_a.iter().enumerate() {
            // Each card in pile A must be one of the revealed five, named at most once.
            if !revealed.contains(id) || pile_a[..i].contains(id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();
        let pile_b: Vec<ObjectId> = revealed
            .into_iter()
            .filter(|id| !pile_a.contains(id))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChoosePileForHand {
                player: controller,
                source,
                pile_a,
                pile_b,
            },
        );
        Ok(Vec::new())
    }

    /// Answer a [`PendingChoice::ChoosePileForHand`] (Fact or Fiction): the chosen pile goes to
    /// `player`'s hand (CR "Put one pile into your hand"), the other is milled into their
    /// graveyard (CR "and the other into your graveyard" — [`Event::Milled`], since these cards
    /// never left the library).
    pub(crate) fn choose_pile_for_hand(
        &mut self,
        player: PlayerId,
        pile: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChoosePileForHand { pile_a, pile_b, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let (to_hand, to_graveyard) = match pile {
            0 => (pile_a, pile_b),
            1 => (pile_b, pile_a),
            _ => return Err(Reject::IllegalChoice),
        };
        self.finish_answer();

        let mut events = Vec::new();
        for from in to_hand {
            self.push_apply(
                &mut events,
                Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from,
                    card: self.def_of(from),
                },
            );
        }
        for from in to_graveyard {
            self.push_apply(
                &mut events,
                Event::Milled {
                    player,
                    card: self.next_object_id(),
                    from,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::OpponentChoosesPile`]: the chosen pile goes to `controller`'s
    /// graveyard; the other pile is offered to `controller` on a
    /// [`PendingChoice::ChooseExiledToCastFree`] (up to one free cast, the rest to hand).
    pub(crate) fn choose_opponent_pile(
        &mut self,
        _player: PlayerId,
        pile: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::OpponentChoosesPile {
            controller,
            source,
            pile_a,
            pile_b,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let (chosen, other) = match pile {
            0 => (pile_a, pile_b),
            1 => (pile_b, pile_a),
            _ => return Err(Reject::IllegalChoice),
        };
        self.finish_answer();

        let mut events = Vec::new();
        for from in chosen {
            self.push_apply(
                &mut events,
                Event::MovedToGraveyard {
                    card: self.next_object_id(),
                    from,
                },
            );
        }
        self.choose_exiled_to_cast_free_pile(controller, source, other, 1, true, &mut events);
        Ok(events)
    }

    /// Resolve Plargg and Nassari ([`Effect::EachPlayerExilesUntilNonlandOpponentPicks`]): each
    /// living player, in APNAP order, exiles cards from the top of their own library until they
    /// exile a nonland (all face-up, public); an opponent then picks one of the exiled nonlands.
    pub(crate) fn each_player_exiles_until_nonland(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let mut exiled: Vec<ObjectId> = Vec::new();
        let mut nonlands: Vec<ObjectId> = Vec::new();
        for p in self.apnap_order() {
            while let Some(&from) = self.players[p.0 as usize].library.first() {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player: p,
                        card,
                        from,
                        face_down: false,
                    },
                );
                exiled.push(card);
                if !matches!(self.def_of(card).kind, CardKind::Land { .. }) {
                    nonlands.push(card);
                    break;
                }
            }
        }
        let Some(opponent) = self.next_opponent_in_turn_order(controller) else {
            return; // ponytail: no opponent to choose — unreachable in a real game.
        };
        pending::raise(
            self,
            pending::ChoiceRequest::OpponentChoosesExiledNonland {
                player: opponent,
                controller,
                source,
                nonlands,
                exiled,
            },
        );
    }

    /// Answer a [`PendingChoice::OpponentChoosesExiledNonland`]: the picked nonland stays exiled;
    /// `controller` is then offered up to two free casts over the *other* exiled cards.
    pub(crate) fn choose_opponent_exiled_nonland(
        &mut self,
        _player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::OpponentChoosesExiledNonland {
            controller,
            source,
            nonlands,
            exiled,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // The opponent must choose one exiled nonland — declining isn't legal (CR "an opponent
        // chooses a nonland card exiled this way").
        let Some(picked) = choice.filter(|c| nonlands.contains(c)) else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        let others: Vec<ObjectId> = exiled.into_iter().filter(|&id| id != picked).collect();
        self.choose_exiled_to_cast_free_pile(controller, source, others, 2, false, &mut events);
        Ok(events)
    }

    /// Resolve [`Effect::RevealUntilMayDeploy`] (Songbirds' Blessing's enchanted-creature-
    /// attacks trigger): reveal `controller`'s own top cards one at a time until the first card
    /// matching `filter` or the library runs out (CR 120.3), collecting every non-match along the
    /// way. The matching card is left unmoved on top of the library and offered on a
    /// [`ChoiceRequest::RevealedCardToBattlefieldOrHand`]; either way (a hit or a whiff), the
    /// collected non-matches are bottomed together, shuffled, via
    /// [`Self::bottom_pile_in_library`].
    pub(crate) fn reveal_until_may_deploy(
        &mut self,
        controller: PlayerId,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let library: Vec<ObjectId> = self.players[controller.0 as usize].library.clone();
        let mut rest: Vec<ObjectId> = Vec::new();
        for card in library {
            let def = self.def_of(card);
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card,
                    def,
                },
            );
            if !filter.matches(def) {
                rest.push(card);
                continue;
            }
            self.bottom_pile_in_library(controller, &rest, events);
            pending::raise(
                self,
                pending::ChoiceRequest::RevealedCardToBattlefieldOrHand {
                    player: controller,
                    card,
                },
            );
            return;
        }
        self.bottom_pile_in_library(controller, &rest, events);
    }

    /// Answer a [`PendingChoice::RevealedCardToBattlefieldOrHand`]: `choice = Some(card)` puts
    /// the revealed card onto the battlefield untapped; `None` puts it into hand instead. An
    /// Aura deployed this way may pause again on [`PendingChoice::ChooseAttachHost`]
    /// (CR 303.4f) — see [`Self::maybe_pause_attach_deployed_aura`].
    pub(crate) fn revealed_card_to_battlefield_or_hand(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::RevealedCardToBattlefieldOrHand { card, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| c != card) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if choice.is_some() {
            let permanent = self.next_object_id();
            self.push_apply(
                &mut events,
                Event::SearchedToBattlefield {
                    permanent,
                    from: card,
                    controller: player,
                    tapped: false,
                },
            );
            self.maybe_pause_attach_deployed_aura(permanent, player);
        } else {
            let def = self.def_of(card);
            self.push_apply(
                &mut events,
                Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from: card,
                    card: def,
                },
            );
        }
        Ok(events)
    }

    /// After an Aura or Equipment is put onto the battlefield without being cast (Songbirds'
    /// Blessing's [`Self::revealed_card_to_battlefield_or_hand`], Armored Skyhunter's
    /// [`Self::select_from_top`] `TopDest::Battlefield` branch), pause its controller to choose a
    /// host among the battlefield creatures it could legally attach to. An Aura's candidates are
    /// creatures it could legally enchant (CR 303.4f), reusing the same `enchant`-restriction
    /// legality check an Aura spell's own cast target uses ([`Game::required_target`]); the
    /// choice is mandatory. Equipment's candidates are creatures its controller controls (CR
    /// 301.5c "you may attach it to a creature you control"); the choice is optional (may
    /// decline, leaving it unattached — legal for Equipment, unlike an unattached Aura). Any
    /// other permanent kind is untouched, no pause. Either way, no legal host means no pause: an
    /// Aura stays unattached and the existing Aura-legality state-based action (CR 704.5m) sweeps
    /// it to the graveyard once this submission's SBA sweep runs; a hostless Equipment simply
    /// sits unattached, which is always legal for Equipment.
    pub(crate) fn maybe_pause_attach_deployed_aura(
        &mut self,
        deployed: ObjectId,
        controller: PlayerId,
    ) {
        let def = self.def_of(deployed);
        let (host_filter, optional) = match def.kind {
            CardKind::Aura => (
                def.enchant
                    .unwrap_or(PermanentFilter::of(TypeSet::CREATURE)),
                false,
            ),
            _ if def.subtypes.contains(&"Equipment") => (
                PermanentFilter {
                    controller: FilterController::You,
                    ..PermanentFilter::of(TypeSet::CREATURE)
                },
                true,
            ),
            _ => return,
        };
        let candidates: Vec<ObjectId> = self
            .battlefield()
            .into_iter()
            .filter(|&id| self.permanent_matches(&host_filter, id, controller, Some(deployed)))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseAttachHost {
                player: controller,
                attachment: deployed,
                candidates,
                optional,
            },
        );
    }

    /// Answer a [`PendingChoice::ChooseAttachHost`]: `host = Some(id)` attaches the deployed
    /// Aura/Equipment to `id` (CR 303.4f / CR 301.5c — the controller chooses among the objects
    /// it could legally attach to). `host = None` only answers an `optional` choice (an
    /// Equipment declining to attach); a mandatory Aura host rejects it.
    pub(crate) fn choose_attach_host(
        &mut self,
        _player: PlayerId,
        host: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseAttachHost {
            attachment,
            candidates,
            optional,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let Some(host) = host else {
            if !optional {
                return Err(Reject::IllegalChoice);
            }
            self.finish_answer();
            return Ok(Vec::new());
        };
        if !candidates.contains(&host) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::AttachedTo {
                object: attachment,
                host: Some(host),
            },
        );
        Ok(events)
    }

    /// Shared core of [`Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost`] (Screams from
    /// Within's immediate dies-return, Ghoulish Impetus's delayed one once its schedule fires):
    /// move this Aura (`source`) from the graveyard to the battlefield unattached — under its
    /// owner's control, the same reanimate-a-card-from-your-own-graveyard idiom
    /// [`Effect::ReturnThisAuraAttachedTo`]'s resolve arm uses — then pause on
    /// [`PendingChoice::ChooseAttachHost`] via
    /// [`Self::maybe_pause_attach_deployed_aura`], the same choose-host surface a deployed Aura
    /// already uses (CR 303.4f). Guard-returns with no events if this Aura has since left the
    /// graveyard (CR 603.10a last-known information — milled/exiled elsewhere in the meantime).
    pub(crate) fn return_aura_from_graveyard_attached_to_chosen_host(
        &mut self,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let card = self.current_id(source);
        if self.zone_of(card) != Zone::Graveyard {
            return;
        }
        let owner = self.owner_of(card);
        let event = self.reanimate_event(card, owner, false);
        let Event::ReanimatedToBattlefield { permanent, .. } = event else {
            unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
        };
        self.push_apply(events, event);
        self.maybe_pause_attach_deployed_aura(permanent, owner);
    }

    /// Resolve [`Effect::RevealUntilExileCastFree`] (Creative Technique's reveal-until-nonland
    /// dig, run after its preceding [`Effect::ShuffleLibrary`] step): reveal `controller`'s own
    /// top cards one at a time — same loop shape as [`Self::reveal_until_may_deploy`] —
    /// until the first card matching `filter` or the library runs out, collecting every non-match
    /// along the way. The matching card is exiled face-up and raises the shared
    /// [`ChoiceRequest::ChooseExiledDigToCastFree`] (a single-candidate batch); either way (a hit
    /// or a whiff), the collected non-matches are bottomed together, shuffled, via
    /// [`Self::bottom_pile_in_library`].
    pub(crate) fn reveal_until_exile_cast_free(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let library: Vec<ObjectId> = self.players[controller.0 as usize].library.clone();
        let mut rest: Vec<ObjectId> = Vec::new();
        for from in library {
            let def = self.def_of(from);
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card: from,
                    def,
                },
            );
            if !filter.matches(def) {
                rest.push(from);
                continue;
            }
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player: controller,
                    card,
                    from,
                    face_down: false,
                },
            );
            self.bottom_pile_in_library(controller, &rest, events);
            pending::raise(
                self,
                pending::ChoiceRequest::ChooseExiledDigToCastFree {
                    player: controller,
                    source,
                    candidates: vec![card],
                    exiled: vec![card],
                },
            );
            return;
        }
        self.bottom_pile_in_library(controller, &rest, events);
    }

    /// Resolve [`Effect::Cascade`] (CR 702.85). Reveal cards from the top of `controller`'s
    /// library one at a time, exiling each face-up (public, reusing the dig's
    /// [`Event::ExiledFromLibraryToChooseCastFree`]), until the just-exiled card is a **nonland**
    /// with mana value strictly less than `mana_value` (the cascading spell's own mana value), or
    /// the library runs out (CR 702.85c "as many as possible"). A hit raises a may-cast-it-free
    /// choice over exactly that card (reusing [`ChoiceRequest::ChooseExiledDigToCastFree`]); with
    /// no hit, the whole reveal is bottomed immediately in a random order.
    pub(crate) fn cascade(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        mana_value: u32,
        events: &mut Vec<Event>,
    ) {
        let mut exiled: Vec<ObjectId> = Vec::new();
        let mut hit: Option<ObjectId> = None;
        while let Some(&from) = self.players[controller.0 as usize].library.first() {
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player: controller,
                    card,
                    from,
                    face_down: false,
                },
            );
            exiled.push(card);
            let def = self.def_of(card);
            if !matches!(def.kind, CardKind::Land { .. }) && def.mana_value() < mana_value {
                hit = Some(card);
                break;
            }
        }
        let Some(hit) = hit else {
            self.bottom_exiled_dig(&exiled, events); // whiff — bottom everything revealed
            return;
        };
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledDigToCastFree {
                player: controller,
                source,
                candidates: vec![hit],
                exiled,
            },
        );
    }

    /// The living players in APNAP order (CR 101.4): the active player first, then each other in
    /// turn order. Used to sequence a multi-player edict's per-player choices.
    pub(crate) fn apnap_order(&self) -> Vec<PlayerId> {
        let n = self.players.len();
        let active = self.active_player.0 as usize;
        (0..n)
            .map(|i| PlayerId(((active + i) % n) as u8))
            .filter(|&p| !self.players[p.0 as usize].lost)
            .collect()
    }

    /// The permanents `player` controls that a sacrifice edict's `filter` can take.
    pub(crate) fn edict_options(&self, player: PlayerId, filter: PermanentFilter) -> Vec<ObjectId> {
        self.controlled_battlefield(player)
            .into_iter()
            .filter(|&id| self.permanent_matches(&filter, id, player, None))
            .collect()
    }

    /// Sacrifice each of `ids` (already validated as legal), pushing both the death event and
    /// the [`Event::Sacrificed`] marker for each — the shared tail
    /// [`ChoiceRequest::ChooseOwnSacrifices`]'s no-real-choice path and
    /// [`Game::choose_own_sacrifices`]'s answer path both run.
    pub(crate) fn sacrifice_ids(&mut self, ids: &[ObjectId], by: PlayerId, events: &mut Vec<Event>) {
        for &id in ids {
            let def = self.def_of(id);
            let event = self.sacrifice_event(id);
            self.push_apply(events, event);
            self.push_apply(
                events,
                Event::Sacrificed {
                    object: id,
                    by,
                    def,
                },
            );
        }
    }

    /// Answer a [`PendingChoice::ChooseOwnSacrifices`]: `sacrifices` must be exactly `count` of
    /// the choice's `options`, each distinct (CR 701.16a) — mandatory, unlike
    /// [`MaySacrifice`](Self::answer_may_sacrifice)'s decline.
    pub(crate) fn choose_own_sacrifices(
        &mut self,
        _player: PlayerId,
        sacrifices: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseOwnSacrifices {
            player,
            options,
            count,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if sacrifices.len() != count as usize {
            return Err(Reject::IllegalChoice); // mandatory: exactly `count`, not fewer or more
        }
        for (i, &id) in sacrifices.iter().enumerate() {
            if !options.contains(&id) || sacrifices[..i].contains(&id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.sacrifice_ids(&sacrifices, player, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::Devour`]: `sacrifices` is any subset (empty declines) of the
    /// choice's `options`, each distinct. Sacrifice them, then place `multiplier × count` +1/+1
    /// counters on `source` through [`Game::counters_after_replacements`] (CR 614 doublers apply).
    pub(crate) fn answer_devour(
        &mut self,
        _player: PlayerId,
        sacrifices: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::Devour {
            player,
            source,
            multiplier,
            options,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        for (i, &id) in sacrifices.iter().enumerate() {
            if !options.contains(&id) || sacrifices[..i].contains(&id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.sacrifice_ids(&sacrifices, player, &mut events);
        let counters =
            self.counters_after_replacements(source, multiplier as i32 * sacrifices.len() as i32);
        if counters > 0 {
            self.push_apply(
                &mut events,
                Event::CountersPlaced {
                    object: source,
                    count: counters,
                    source_name: self.def_of(source).name,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCopyTarget`]: `copy = Some(creature)` (one of the choice's
    /// `candidates`) has `source` become a copy of it — overwriting its `def` (CR 707.2) and
    /// applying the marker's riders (extra +1/+1 counters, until-EOT duration, haste). `None`
    /// declines the "you may" and `source` stays its printed self.
    pub(crate) fn answer_enter_as_copy(
        &mut self,
        player: PlayerId,
        copy: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCopyTarget {
            source,
            candidates,
            until_eot,
            extra_counters,
            gains_haste,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if let Some(chosen) = copy
            && !candidates.contains(&chosen)
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(chosen) = copy else {
            return Ok(events); // declined — the printed permanent stands.
        };
        // The counters/haste name their real source (the copier), captured before the def
        // overwrite renames `source` to the copied creature.
        let source_name = self.def_of(source).name;
        // ponytail: the copyable values are the chosen creature's printed/`CardDef` values (CR
        // 707.2), not a full read of any copy-layer modifications already on it — exact for this
        // pool (no card copies something already under a copy effect).
        // ponytail: `BecameCopy` overwrites `def` *after* `PermanentEntered` fired, so an ETB
        // trigger of the *copied* creature is missed (the trigger watcher saw the pre-copy def).
        // Neither Altered Ego nor Cursed Mirror copies a creature with an ETB; revisit when one does.
        let def = self.def_of(chosen);
        self.push_apply(
            &mut events,
            Event::BecameCopy {
                object: source,
                def,
                until_eot,
            },
        );
        // Altered Ego's "except it enters with X additional +1/+1 counters" — placed on the copy
        // through the CR 614 counter-replacement pipeline (Hardened Scales, a doubler). `x` is the
        // copier's own locked-in cast {X} ([`Permanent::entered_with_x`]).
        let x = self.ability_source_x(source);
        let extra = self.resolve_count(extra_counters, player, source, None, x);
        let n = self.counters_after_replacements(source, extra as i32);
        if n > 0 {
            self.push_apply(
                &mut events,
                Event::CountersPlaced {
                    object: source,
                    count: n,
                    source_name,
                },
            );
        }
        // Cursed Mirror's "except it has haste."
        if gains_haste {
            const HASTE: &[Keyword] = &[Keyword::Haste];
            self.push_apply(
                &mut events,
                Event::TempBoost {
                    object: source,
                    power: 0,
                    toughness: 0,
                    keywords: HASTE,
                    source_name,
                },
            );
        }
        // Copy Enchantment copying an Aura (CR 707.2 read with CR 303.4f): the copy entered
        // unattached above (`BecameCopy` only overwrites `def`), so it must now pause to choose a
        // host among legal enchant targets — the same deployed-Aura attach path a searched-out or
        // reanimated Aura uses.
        if def.kind == CardKind::Aura {
            self.maybe_pause_attach_deployed_aura(source, player);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseTokenToCopy`] (reusing [`Intent::ChooseCopyTarget`]):
    /// `copy = Some(token)` (one of the choice's `candidates`) makes every *other* token the
    /// player controls become a copy of it — an indefinite [`Event::BecameCopy`] per other token
    /// (CR 706/707.2; permanent, CR 400.7). `None` declines the "you may" and converts nothing.
    pub(crate) fn answer_each_other_token_becomes_copy(
        &mut self,
        _player: PlayerId,
        copy: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseTokenToCopy { candidates, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if let Some(chosen) = copy
            && !candidates.contains(&chosen)
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(chosen) = copy else {
            return Ok(events); // declined — nothing is converted.
        };
        // ponytail: the copyable values are the chosen token's `CardDef` values (CR 707.2), not a
        // full read of any copy-layer modifications already on it — exact for this pool, same note
        // as `Game::answer_enter_as_copy` (slice 2). Snapshot the other tokens up front, before
        // any `BecameCopy` applies.
        let def = self.def_of(chosen);
        let others: Vec<ObjectId> = candidates.into_iter().filter(|&id| id != chosen).collect();
        for other in others {
            self.push_apply(
                &mut events,
                Event::BecameCopy {
                    object: other,
                    def,
                    until_eot: false,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCopyCardFromList`] (reusing [`Intent::ChooseCopyTarget`]):
    /// `copy = Some(card)` (one of the choice's `candidates`) has `source` become a copy of that
    /// card until end of turn (an [`Event::BecameCopy`] with `until_eot: true`, CR 706/707.2).
    /// `None` declines the "you may" and copies nothing.
    /// ponytail: the copyable values are the chosen card's printed `CardDef` (CR 707.2), not a
    ///   full read of any copy-layer modifications — exact for this pool, same note as
    ///   `Game::answer_enter_as_copy` (slice 2).
    pub(crate) fn answer_choose_copy_card_from_list(
        &mut self,
        _player: PlayerId,
        copy: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCopyCardFromList {
            source, candidates, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if let Some(chosen) = copy
            && !candidates.contains(&chosen)
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(chosen) = copy else {
            return Ok(events); // declined — nothing is copied.
        };
        self.push_apply(
            &mut events,
            Event::BecameCopy {
                object: source,
                def: self.def_of(chosen),
                until_eot: true,
            },
        );
        Ok(events)
    }

    /// Discard each of `ids` (already validated as legal), moving each to the graveyard and
    /// firing the CR 701.8 discard marker — the shared tail [`Game::answer_discard`]'s effect
    /// discard and [`Game::answer_may_discard`]'s optional discard both run.
    pub(crate) fn discard_ids(
        &mut self,
        ids: &[ObjectId],
        player: PlayerId,
        events: &mut Vec<Event>,
    ) {
        for &id in ids {
            let card = self.next_object_id();
            let def = self.def_of(id);
            self.push_apply(events, Event::MovedToGraveyard { card, from: id });
            self.push_apply(
                events,
                Event::Discarded {
                    card,
                    from: id,
                    def,
                    player,
                },
            );
        }
    }

    /// Answer a [`PendingChoice::MayDiscard`]: `discards` is empty to decline, or names the one
    /// hand card discarded to gain `then`'s effects (CR 608.2c-style "you may … if you do").
    pub(crate) fn answer_may_discard(
        &mut self,
        player: PlayerId,
        discards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayDiscard {
            source,
            options,
            then,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if discards.len() > 1 || discards.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.discard_ids(&discards, player, &mut events);
        // "If you do": the rider only fires when a card was actually discarded. `then` may
        // itself pause, so `run_sequence` is the runner — same reasoning as
        // `Game::answer_may_sacrifice`'s rider.
        if !discards.is_empty() {
            self.run_sequence(
                then,
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            );
        }
        Ok(events)
    }

    /// Resolve a multi-player sacrifice edict ([`Effect::EachPlayerSacrifices`]): each affected
    /// player (per `scope`, APNAP order) loses `life_loss` life, then the affected players choose
    /// their sacrifices one at a time (each raising [`ChoiceRequest::NextSacrificeEdict`]). Once
    /// all have chosen, `follow_up` runs for `controller`.
    ///
    /// [`EdictScope::TargetedPlayers`] (Priest of Forgotten Gods' "any number of target players")
    /// has no scope-derived affected set to compute — `controller` first raises
    /// [`ChoiceRequest::ChooseTargetPlayers`] (CR 601.2c/608.2b: zero is a legal choice); once
    /// answered, [`Self::choose_target_players`] applies the life loss and continues into
    /// [`Self::prompt_next_sacrifice`] exactly as below.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn sacrifice_edict(
        &mut self,
        scope: EdictScope,
        keep_one: bool,
        filter: PermanentFilter,
        life_loss: i32,
        follow_up: &'static [Effect],
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        // Scoped to this one edict (Deadly Brew's "if you sacrificed this way" gate) — overwrite,
        // not accumulate, so a prior edict earlier this game can't leak through.
        self.sacrificed_by_edict_controller = false;
        if scope == EdictScope::TargetedPlayers {
            let legal = self.apnap_order();
            pending::raise(
                self,
                pending::ChoiceRequest::ChooseTargetPlayers {
                    player: controller,
                    source,
                    max: legal.len() as u8,
                    legal,
                    min: 0,
                    keep_one,
                    filter,
                    life_loss,
                    then: follow_up,
                },
            );
            return;
        }
        let affected: Vec<PlayerId> = self
            .apnap_order()
            .into_iter()
            .filter(|&p| scope != EdictScope::EachOpponent || p != controller)
            .collect();
        if life_loss != 0 {
            for &p in &affected {
                self.push_apply(
                    events,
                    Event::LifeChanged {
                        player: p,
                        amount: -life_loss,
                        source: Some(source),
                    },
                );
            }
        }
        self.prompt_next_sacrifice(
            affected, keep_one, filter, follow_up, controller, source, events,
        );
    }

    /// Answer a [`PendingChoice::ChooseTargetPlayers`] (Priest of Forgotten Gods' "any number of
    /// target players"): `players` becomes the edict's affected set, life loss applied, then the
    /// same per-player sacrifice fan-out [`Self::sacrifice_edict`] runs for
    /// `AllPlayers`/`EachOpponent` continues from [`Self::prompt_next_sacrifice`].
    pub(crate) fn choose_target_players(
        &mut self,
        player: PlayerId,
        players: Vec<PlayerId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseTargetPlayers {
            player: chooser,
            source,
            legal,
            min,
            max,
            keep_one,
            filter,
            life_loss,
            then,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != chooser || !valid_target_player_choice(&players, &legal, min, max) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if life_loss != 0 {
            for &p in &players {
                self.push_apply(
                    &mut events,
                    Event::LifeChanged {
                        player: p,
                        amount: -life_loss,
                        source: Some(source),
                    },
                );
            }
        }
        self.prompt_next_sacrifice(
            players,
            keep_one,
            filter,
            then,
            chooser,
            source,
            &mut events,
        );
        Ok(events)
    }

    /// Raise on the next affected player who has a real sacrifice to make (skipping any with
    /// nothing to give up), or — when none remain — run the edict's `follow_up` for `controller`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prompt_next_sacrifice(
        &mut self,
        remaining: Vec<PlayerId>,
        keep_one: bool,
        filter: PermanentFilter,
        follow_up: &'static [Effect],
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        pending::raise(
            self,
            pending::ChoiceRequest::NextSacrificeEdict {
                remaining,
                keep_one,
                filter,
                follow_up,
                controller,
                source,
            },
        );
        if self.resolution_is_paused() {
            return;
        }
        for &effect in follow_up {
            self.run(
                effect,
                ResolveCtx {
                    controller,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                events,
            );
            if self.resolution_is_paused() {
                break;
            }
        }
    }

    /// Answer a [`PendingChoice::SacrificeEdict`]: sacrifice the chosen permanents, then move on
    /// to the next affected player (or the follow-up). For a plain edict `sacrifices` is exactly
    /// one option; for `keep_one` it's every option but the one kept.
    pub(crate) fn choose_sacrifices(
        &mut self,
        _player: PlayerId,
        sacrifices: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SacrificeEdict {
            player: sacrificer,
            options,
            keep_one,
            filter,
            remaining,
            controller,
            source,
            follow_up,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if !valid_sacrifice_choice(&sacrifices, &options, keep_one) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        // Deadly Brew's "if you sacrificed a permanent this way" gate: only the edict's own
        // controller sacrificing (not just any affected player) meets it.
        if sacrificer == controller && !sacrifices.is_empty() {
            self.sacrificed_by_edict_controller = true;
        }

        let mut events = Vec::new();
        // Sacrifices route through the normal death events, so "when this/a creature dies" fires.
        for &id in &sacrifices {
            let def = self.def_of(id);
            let event = self.sacrifice_event(id);
            self.push_apply(&mut events, event);
            self.push_apply(
                &mut events,
                Event::Sacrificed {
                    object: id,
                    by: sacrificer,
                    def,
                },
            );
        }
        self.prompt_next_sacrifice(
            remaining,
            keep_one,
            filter,
            follow_up,
            controller,
            source,
            &mut events,
        );
        Ok(events)
    }

    /// Pause on the next player who controls a nonland permanent (skipping any who control none —
    /// nothing to keep or sacrifice), or — when none remain — return, letting the enclosing spell
    /// resolution finish.
    pub(crate) fn prompt_next_caster_keep(
        &mut self,
        remaining: Vec<PlayerId>,
        caster: PlayerId,
        source: ObjectId,
    ) {
        super::raise(
            self,
            super::ChoiceRequest::NextCasterKeep {
                remaining,
                caster,
                source,
            },
        );
    }

    /// Answer a [`PendingChoice::CasterKeepPermanents`]: `keeps` are the permanents the caster keeps
    /// for `target_player` (up to one of each type — artifact/creature/enchantment). Sacrifice every
    /// *other* nonland permanent `target_player` controls (by that player, so dies triggers fire —
    /// CR 701.16b), then advance to the next player.
    pub(crate) fn answer_caster_keep(
        &mut self,
        player: PlayerId,
        keeps: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::CasterKeepPermanents {
            caster,
            source,
            target_player,
            options,
            remaining,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != caster || !self.valid_caster_keep(&keeps, &options) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        // Sacrifice every nonland permanent not kept, routed through the normal death events so
        // "when this/a creature dies" fires.
        for id in options.into_iter().filter(|id| !keeps.contains(id)) {
            let def = self.def_of(id);
            let event = self.sacrifice_event(id);
            self.push_apply(&mut events, event);
            self.push_apply(
                &mut events,
                Event::Sacrificed {
                    object: id,
                    by: target_player,
                    def,
                },
            );
        }
        self.prompt_next_caster_keep(remaining, caster, source);
        Ok(events)
    }

    /// Whether `keeps` is a legal keep set for a [`PendingChoice::CasterKeepPermanents`]: each id is
    /// among `options`, no id repeats, the kept permanents can each be assigned a *distinct* type
    /// slot they possess, and the caster keeps one of *every* type the player controls that a
    /// distinct assignment can reach. The choice "an artifact, a creature, an enchantment, and a
    /// planeswalker" is mandatory (CR 601-style: choose one of each type if able), so the caster
    /// can't sacrifice a type they were required to spare — `keeps` must be a *maximum* system of
    /// distinct representatives over `options` (an artifact creature still fills only one slot).
    fn valid_caster_keep(&self, keeps: &[ObjectId], options: &[ObjectId]) -> bool {
        if keeps.iter().any(|id| !options.contains(id)) {
            return false;
        }
        if (1..keeps.len()).any(|i| keeps[..i].contains(&keeps[i])) {
            return false;
        }
        // Match each kept permanent to a distinct type slot it has, and require the keep set to be
        // maximal — as many slots as `options` can simultaneously fill. Only three slots are
        // reachable (no planeswalker permanent in the pool), so small brute force suffices.
        let slots = [TypeSet::ARTIFACT, TypeSet::CREATURE, TypeSet::ENCHANTMENT];
        let keep_masks: Vec<TypeSet> = keeps
            .iter()
            .map(|&id| self.def_of(id).kind.types())
            .collect();
        if !assign_to_distinct_slots(&keep_masks, &slots, 0) {
            return false;
        }
        let option_masks: Vec<TypeSet> = options
            .iter()
            .map(|&id| self.def_of(id).kind.types())
            .collect();
        keeps.len() == max_distinct_slots(&option_masks, &slots)
    }

    /// Pause on the next player who controls a creature (skipping any who control none — nothing to
    /// counter), or — when none remain — return, letting the ability resolution finish.
    pub(crate) fn prompt_next_counter_target(
        &mut self,
        remaining: Vec<PlayerId>,
        chooser: PlayerId,
        source: ObjectId,
    ) {
        super::raise(
            self,
            super::ChoiceRequest::NextCounterTarget {
                remaining,
                chooser,
                source,
            },
        );
    }

    /// Answer a [`PendingChoice::ChooseCounterTargetForPlayer`]: `chosen` is the up-to-one creature
    /// the chooser counters for `target_player` (empty declines — CR 603.3d). Put one +1/+1 counter
    /// on it through the replacement pipeline [`Effect::PutCounters`] uses, then advance to the next
    /// player.
    pub(crate) fn answer_choose_counter_target(
        &mut self,
        player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCounterTargetForPlayer {
            chooser,
            source,
            options,
            remaining,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != chooser || chosen.len() > 1 || chosen.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(&object) = chosen.first() {
            let n = self.counters_after_replacements(object, 1);
            if n > 0 {
                self.push_apply(
                    &mut events,
                    Event::CountersPlaced {
                        object,
                        count: n,
                        source_name: self.def_of(source).name,
                    },
                );
            }
        }
        self.prompt_next_counter_target(remaining, chooser, source);
        Ok(events)
    }

    /// Pause on the next affected player who has a graveyard card to exile (skipping any with an
    /// empty graveyard), or — when none remain — return, letting the enclosing sequence resume.
    pub(crate) fn prompt_next_graveyard_exile(
        &mut self,
        remaining: Vec<PlayerId>,
        source: ObjectId,
    ) {
        super::raise(
            self,
            super::ChoiceRequest::NextGraveyardExile { remaining, source },
        );
    }

    /// Answer a [`PendingChoice::ExileFromGraveyard`]: exile the one chosen graveyard card (routed
    /// through the normal zone-move so a "cards exiled from your graveyard" watch trigger fires),
    /// tallying it if nonland, then move on to the next affected player.
    pub(crate) fn choose_graveyard_exile(
        &mut self,
        _player: PlayerId,
        exiles: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ExileFromGraveyard {
            options,
            remaining,
            source,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // Mandatory: exactly one of the offered cards (declining isn't legal when they have one).
        if exiles.len() != 1 || !options.contains(&exiles[0]) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let id = exiles[0];
        if !matches!(self.def_of(id).kind, CardKind::Land { .. }) {
            self.nonland_cards_exiled_this_way += 1;
        }
        let card = self.next_object_id();
        self.push_apply(&mut events, Event::MovedToExile { card, from: id });
        self.prompt_next_graveyard_exile(remaining, source);
        Ok(events)
    }

    /// Pause on the next player to vote, or — when none remain — return, letting the enclosing
    /// sequence resume into the tally-scaled outcome steps. Unlike a graveyard fan-out, no seat is
    /// ever skipped: every living player votes (CR 701.32a).
    pub(crate) fn prompt_next_vote(
        &mut self,
        remaining: Vec<PlayerId>,
        source: ObjectId,
        options: &'static [&'static str],
    ) {
        super::raise(
            self,
            super::ChoiceRequest::NextVote {
                remaining,
                source,
                options,
            },
        );
    }

    /// Answer a [`PendingChoice::CastVote`]: `choice` is the index into the ballot's `options`
    /// (0 = past, 1 = present). Tally the vote, then move on to the next player.
    pub(crate) fn answer_vote(
        &mut self,
        player: PlayerId,
        choice: usize,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::CastVote {
            player: voter,
            source,
            options,
            remaining,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != voter {
            return Err(Reject::NotYourPriority);
        }
        let Some(&ballot) = options.get(choice) else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        // ponytail: past/present hardcoded — Fateful Tempest is the pool's only council's-dilemma
        // card. Generalize to a label→tally map when a differently-balloted voting card lands.
        match ballot {
            "past" => self.council_past_votes += 1,
            "present" => self.council_present_votes += 1,
            other => panic!("unknown council's-dilemma ballot {other:?}"),
        }
        self.prompt_next_vote(remaining, source, options);
        Ok(Vec::new())
    }

    /// Answer a [`PendingChoice::MaySacrifice`]: `sacrifices` is empty to decline, or names the
    /// one permanent (one of the choice's `options`) sacrificed to gain `then`'s effects (CR
    /// 601.2f-style "you may … if you do").
    pub(crate) fn answer_may_sacrifice(
        &mut self,
        player: PlayerId,
        sacrifices: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MaySacrifice {
            source,
            options,
            then,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if sacrifices.len() > 1 || sacrifices.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &sacrifices {
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
        // "If you do": the rider only fires when a permanent was actually given up. `then` may
        // itself pause (Springbloom Druid's rider is a library search) — `run_sequence` is the
        // general "run this effect list, deferring a pausing tail" runner (the same one
        // `Effect::Sequence` uses), so a pausing rider defers correctly.
        if !sacrifices.is_empty() {
            self.run_sequence(
                then,
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayReturnFromGraveyard`]: `choice` is empty to decline, or names
    /// the one graveyard card (one of the choice's `options`) returned to `player`'s hand
    /// ([`Effect::MayReturnFromGraveyard`] — Deadly Brew's rider).
    pub(crate) fn answer_may_return_from_graveyard(
        &mut self,
        _player: PlayerId,
        choice: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayReturnFromGraveyard { options, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.len() > 1 || choice.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &choice {
            self.push_apply(
                &mut events,
                Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: id,
                },
            );
        }
        Ok(events)
    }

    /// Answer a discard choice — either a cleanup [`PendingChoice::DiscardToHandSize`] or an
    /// [`Effect::Discard`]'s [`PendingChoice::DiscardCards`]: move the chosen cards to the
    /// graveyard. A cleanup discard then resumes the interrupted step-transition (carrying the turn
    /// to the next player); an effect discard leaves any deferred sequence tail for
    /// [`Game::resume_deferred_sequence`].
    pub(crate) fn answer_discard(
        &mut self,
        player: PlayerId,
        cards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let (chooser, hand, count, or_one_matching, is_cleanup) = match self.pending_choice.clone()
        {
            Some(PendingChoice::DiscardToHandSize {
                player,
                hand,
                count,
            }) => (player, hand, count, None, true),
            Some(PendingChoice::DiscardCards {
                player,
                hand,
                count,
                or_one_matching,
            }) => (player, hand, count, or_one_matching, false),
            _ => return Err(Reject::IllegalChoice),
        };
        // Exactly `count` distinct cards, each currently in this player's hand — or, when the
        // effect carries a land-escape-valve filter, a single matching card instead (Compulsive
        // Research's "unless they discard a land card").
        let distinct = cards.iter().collect::<std::collections::HashSet<_>>().len();
        let all_in_hand = cards.iter().all(|c| hand.contains(c));
        let full_discard = cards.len() == count && distinct == cards.len();
        let land_escape = or_one_matching
            .is_some_and(|filter| cards.len() == 1 && filter.matches(self.def_of(cards[0])));
        if player != chooser || !all_in_hand || !(full_discard || land_escape) {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        // CR 701.8: every discard fires "whenever you discard" watchers — a cleanup hand-size
        // trim counts exactly the same as an effect discard.
        self.discard_ids(&cards, player, &mut events);
        // A cleanup discard resumes the step-transition loop it interrupted; an effect discard's
        // sequence tail (if any) is resumed by [`Game::resume_deferred_sequence`] after this returns.
        if is_cleanup {
            events.extend(self.advance_step());
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::DeclineUntap`] (CR 502.2 — Rubinia Soulsinger's "you may choose
    /// not to untap"): untap every offered permanent the active player didn't keep tapped, then
    /// resume the interrupted untap step (the same step-transition resume as a cleanup discard).
    /// Leaving a permanent tapped is exactly what sustains a "remains tapped" control condition —
    /// the SBA sweep after this answer reverts any steal whose source the player chose to untap.
    pub(crate) fn answer_decline_untap(
        &mut self,
        player: PlayerId,
        keep_tapped: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DeclineUntap {
            player: chooser,
            permanents,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // The answer must come from the asked player and only name permanents that were offered.
        if player != chooser || !keep_tapped.iter().all(|id| permanents.contains(id)) {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        for id in permanents {
            if !keep_tapped.contains(&id) {
                self.push_apply(&mut events, Event::Untapped { object: id });
            }
        }
        events.extend(self.advance_step());
        Ok(events)
    }
}

/// Whether each permanent (given by its type `masks`) can be assigned a *distinct* `slot` type it
/// possesses — a system of distinct representatives for Tragic Arrogance's "one of each type" keep
/// (an artifact creature has two type bits but fills only one slot). Small brute-force recursion;
/// the pool never keeps more than three permanents (three reachable slots).
fn assign_to_distinct_slots(masks: &[TypeSet], slots: &[TypeSet], used: u32) -> bool {
    let Some((first, rest)) = masks.split_first() else {
        return true;
    };
    slots.iter().enumerate().any(|(i, &slot)| {
        let bit = 1 << i;
        used & bit == 0
            && first.intersects(slot)
            && assign_to_distinct_slots(rest, slots, used | bit)
    })
}

/// The maximum number of `slots` that distinct permanents (given by their type `masks`) can
/// simultaneously fill — Tragic Arrogance's mandatory keep count for a player (you must spare one
/// of every type you can reach). Each slot may take at most one permanent and each permanent at
/// most one slot (an artifact creature covers artifact *or* creature, not both). Brute-force
/// recursion over the ≤3 reachable slots.
fn max_distinct_slots(masks: &[TypeSet], slots: &[TypeSet]) -> usize {
    let Some((slot, rest)) = slots.split_first() else {
        return 0;
    };
    // Skip this slot, or fill it with any not-yet-used permanent that has its type.
    let skip = max_distinct_slots(masks, rest);
    masks
        .iter()
        .enumerate()
        .filter(|(_, mask)| mask.intersects(*slot))
        .map(|(i, _)| {
            let remaining: Vec<TypeSet> = masks
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, &m)| m)
                .collect();
            1 + max_distinct_slots(&remaining, rest)
        })
        .max()
        .unwrap_or(0)
        .max(skip)
}

#[cfg(test)]
mod tests {
    use crate::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    fn source_creature(game: &mut Game) -> ObjectId {
        game.spawn_on_battlefield(
            P0,
            CardDef {
                name: "Source",
                id: "",
                default_print: "",
                cost: Cost::FREE,
                kind: CardKind::Creature {
                    power: 1,
                    toughness: 1,
                    also: TypeSet::NONE,
                },
                legendary: false,
                uncounterable: false,
                enchant: None,
                enchant_graveyard: false,
                modal: false,
                modal_choose: 1,
                modal_choose_max: None,
                modal_choose_max_if_commander: false,
                keywords: &[],
                conditional_keywords: &[],
                abilities: &[],
                identity_pips: &[],
                colors: &[],
                enters_tapped: false,
                enters_tapped_unless: None,
                approximates: None,
                oracle: None,
                set: "",
                subtypes: &[],
                otags: &[],
                cycling: None,
                flashback: None,
                echo: None,
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
                suspend: None,
                devour: None,
                demonstrate: false,
                enter_as_copy: None,
                encore: None,
                hand_ability: None,
                may_choose_not_to_untap: false,
            },
        )
    }

    #[test]
    fn choose_order_rejects_a_non_permutation() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: vec![
                    Effect::DrawCards {
                        count: Amount::Fixed(1),
                    },
                    Effect::DrawCards {
                        count: Amount::Fixed(2),
                    },
                ],
            },
        );
        assert_eq!(
            game.choose_order(P0, vec![0, 0]),
            Err(Reject::IllegalChoice)
        );
        assert!(
            game.pending_choice.is_some(),
            "invalid answer restores pause"
        );
    }

    #[test]
    fn choose_order_rejects_the_wrong_player() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: vec![Effect::DrawCards {
                    count: Amount::Fixed(1),
                }],
            },
        );
        assert_eq!(game.choose_order(P1, vec![0]), Err(Reject::IllegalChoice));
        assert!(
            game.pending_choice.is_some(),
            "wrong player restores the pause"
        );
    }

    #[test]
    fn choose_order_accepts_a_valid_permutation() {
        let mut game = Game::with_players(2, 0);
        let source = source_creature(&mut game);
        crate::pending::raise_choice(
            &mut game,
            PendingChoice::OrderTriggers {
                player: P0,
                source,
                effects: vec![
                    Effect::DrawCards {
                        count: Amount::Fixed(1),
                    },
                    Effect::DrawCards {
                        count: Amount::Fixed(2),
                    },
                ],
            },
        );
        assert!(game.choose_order(P0, vec![1, 0]).is_ok());
        assert!(game.pending_choice.is_none());
        // order [1, 0] pushes effect 1 then effect 0 — bottom-first stack view.
        assert_eq!(
            game.stack(),
            vec![
                StackEntry::Ability {
                    controller: P0,
                    source,
                    effect: Effect::DrawCards {
                        count: Amount::Fixed(2)
                    },
                    target: None,
                },
                StackEntry::Ability {
                    controller: P0,
                    source,
                    effect: Effect::DrawCards {
                        count: Amount::Fixed(1)
                    },
                    target: None,
                },
            ]
        );
    }
}
