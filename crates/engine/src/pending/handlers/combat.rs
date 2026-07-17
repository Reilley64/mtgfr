//! Combat damage and divide-* answers.

use crate::*;

impl Game {
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
}
