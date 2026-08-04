//! Combat damage and divide-* answers.

use crate::*;

impl Game {
    pub(crate) fn assign_damage(
        &mut self,
        _player: PlayerId,
        assignment: Vec<(ObjectId, i32)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::AssignCombatDamage {
            source, recipients, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let assigned: Vec<ObjectId> = assignment.iter().map(|&(b, _)| b).collect();
        let covers_recipients = assigned.len() == recipients.len()
            && recipients.iter().all(|b| assigned.contains(b))
            && assigned.iter().all(|b| recipients.contains(b));
        let nonneg = assignment.iter().all(|&(_, amt)| amt >= 0);
        let total: i32 = assignment.iter().map(|&(_, amt)| amt).sum();
        let power = self.power(source);
        // CR 702.19b lets an *attacking* trampler hold damage back for the defending player, so its
        // division may come up short. CR 702.19c gives a blocking trampler nowhere to put the
        // excess, so a blocker's division still has to spend its whole power.
        let tramples_over =
            self.has_keyword(source, Keyword::Trample) && self.combat.attackers.contains(&source);
        let total_ok = if tramples_over {
            total <= power
        } else {
            total == power
        };
        if !covers_recipients || !nonneg || !total_ok || assignment.len() > MAX_BLOCKERS {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }
        if !self.lethal_order_respected(source, &recipients, &assignment) {
            return Err(Reject::IllegalChoice); // CR 510.1c — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::CombatDamageDivided {
                source,
                assignment: DamageAssignment::from_pairs(&assignment),
            },
        );

        // Resume the combat damage step's turn-based action (CR 510.1): chain to the next
        // multi-blocked attacker's division, or — once every one is settled — deal the batch. The
        // step's own priority window is opened here, since `advance_step` returned early to raise
        // this choice and never reached it. (CR 117, CR 402.5, CR 508)
        self.divide_or_deal_combat_damage(self.step == Step::FirstStrikeCombatDamage, &mut events);
        if self.pending_choice.is_none() {
            self.consecutive_passes = 0;
            self.priority = self.active_player;
        }
        Ok(events)
    }

    /// CR 510.1c: a creature dividing its combat damage may not assign any to a recipient until
    /// every recipient ahead of it in the *damage assignment order* has been assigned lethal
    /// damage (CR 510.1d defers to the same reading for a blocker dividing among the attackers it
    /// blocks). "Lethal damage" is toughness minus damage already marked, or 1 for a deathtouch
    /// source (CR 702.2b).
    ///
    /// The engine records no damage assignment order — CR 509.2 has the dividing player announce
    /// one as blockers are declared, and it may be any permutation of the recipients. So the rule
    /// is enforced in the form that is order-independent: an assignment is realizable under *some*
    /// order exactly when **at most one** recipient is left short of lethal while still taking
    /// damage, since every recipient at lethal sorts ahead of that one and every zero sorts behind
    /// it. Two half-killed blockers is the shape the rule forbids.
    ///
    /// CR 702.22j/k are exceptions: when banding moved the choice to the other seat, that seat
    /// divides freely and no order constrains it. [`Game::banding_division_shifter`] reads the
    /// same condition over the recipients that picked the assigner in the first place.
    ///
    /// ponytail: damage assigned by *another* creature in the same batch does not count toward a
    /// recipient's lethal reading — the rule allows it, and tracking it needs a batch-wide ledger
    /// the divisions are answered one at a time without. A trampler holding damage back (CR
    /// 702.19b) is likewise not made to bring every blocker to lethal first.
    fn lethal_order_respected(
        &self,
        source: ObjectId,
        recipients: &[ObjectId],
        assignment: &[(ObjectId, i32)],
    ) -> bool {
        if self.banding_division_shifter(recipients).is_some() {
            return true;
        }
        let deathtouch = self.has_keyword(source, Keyword::Deathtouch);
        let short_of_lethal = assignment
            .iter()
            .filter(|&&(recipient, amount)| {
                let lethal = if deathtouch {
                    1
                } else {
                    (self.toughness(recipient) - self.permanent(recipient).marked_damage).max(0)
                };
                amount > 0 && amount < lethal
            })
            .count();
        short_of_lethal <= 1
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
        player: PlayerId,
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
        self.move_counters_distributed(player, from, &assignment, &mut events);
        Ok(events)
    }
}
