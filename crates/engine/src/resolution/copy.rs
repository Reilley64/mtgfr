//! Spell/ability copy choreography — CR 706 / 707 / 114.6 (copying spells, copying
//! abilities, changing targets, minting free copies of exiled cards).
//!
//! Peeled out of [`Game::run`] (card-dsl-and-card-pool spec deepen). Each `run_copy` arm needs `&mut self`
//! either to mint a new stack object ([`Effect::Copy(CopyEffect::TargetSpell)`], `CopyThisSpell`),
//! reuse [`Self::mint_spell_copies`] behind [`Effect::Copy(CopyEffect::RetargetSpellCopy)`]'s pause queue,
//! or pause on a retarget/pay-cost choice ([`Effect::Copy(CopyEffect::MayPayToCopyThis)`],
//! [`Effect::Copy(CopyEffect::ChangeTargetOfTargetSpellOrAbility)`]).

use crate::*;

impl Game {
    /// Resolve one of the copy-family choreography arms behind [`Game::run`]. Each match
    /// arm is a 1:1 relocation of its (formerly inline) [`Game::run`] body — no behavior
    /// change.
    pub(crate) fn run_copy(&mut self, effect: Effect, ctx: ResolveCtx, events: &mut Vec<Event>) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        match effect {
            // Twincast: put a copy of the target spell on the stack under this controller, then
            // offer CR 707.10c's "you may choose new targets for the copy" — same
            // `choose_spell_targets` machinery a multi-target spell uses at cast (auto-fills a
            // single legal target, else pauses on `ChooseSpellTargets`), just run here because
            // the copy doesn't exist until this event applies.
            Effect::Copy(CopyEffect::TargetSpell) => {
                let original = expect_object_target(target, "a spell copy");
                // CR 707.10: if the target spell has left the stack (countered/resolved), the copy
                // effect does nothing.
                if !matches!(self.objects[original as usize], Object::Spell(_)) {
                    return;
                }
                let original_def = self.def_of(original);
                let copy = self.next_object_id();
                self.push_apply(
                    events,
                    Event::SpellCopied {
                        copy,
                        original,
                        controller,
                    },
                );
                // The copy's target spec/count come from the original's own primary spell effect
                // (the copy shares the same `def`) — the same lookup
                // `ChangeTargetOfTargetSpellOrAbility`'s optional (Wild Ricochet) bend shares.
                let Some((spec, count)) = self.spell_primary_target(original_def) else {
                    return;
                };
                // The copy's own controller both chooses and anchors legality — a fresh copy's
                // "you" is its new controller (CR 707.10a), unlike retargeting the ORIGINAL spell
                // (whose own controller never changes).
                self.choose_spell_targets(copy, spec, count, controller, controller, events);
            }
            // Storm/Gravestorm-style rider: mint `count` copies of *this* resolving spell
            // (`source`, not a chosen target). CR 706.9's "when you cast this spell" trigger
            // never re-fires for an uncast copy — guard that here rather than modeling a real
            // cast trigger, since this rider runs as one of the spell's own resolution effects.
            Effect::Copy(CopyEffect::ThisSpell {
                count,
                cast_from_graveyard_only,
                optional,
            }) => {
                if self.spell(source).copy {
                    return;
                }
                // "If this spell was cast from a graveyard" (Sevinne's Reclamation's flashback
                // rider) — the mint never happens for an ordinary cast.
                if cast_from_graveyard_only && !self.spell(source).flashback {
                    return;
                }
                // "You may copy this spell": pause for a yes/no before minting (mirrors
                // `MaySacrifice`/`MayReturnFromGraveyard`'s resolution-time "declining runs
                // nothing" shape); the mandatory storm/Gravestorm case (`optional = false`) skips
                // straight to the mint below. `answer_may` mints inline on acceptance rather than
                // placing a new triggered ability — this rider is part of `source`'s own
                // resolution, not a separate stack object.
                if optional {
                    pending::raise(
                        self,
                        pending::ChoiceRequest::MayYesNo {
                            player: controller,
                            source,
                            effect: Effect::Copy(CopyEffect::ThisSpell {
                                count,
                                cast_from_graveyard_only: false,
                                optional: false,
                            }),
                        },
                    );
                    return;
                }
                self.mint_spell_copies(count, controller, source, target, x, events);
            }
            // Internal continuation step for `CopyThisSpell` above — never authored in a card
            // TOML (`copy` is a runtime-minted object id, meaningless in a template). Offers the
            // same CR 707.10c retarget choice `CopyTargetSpell` offers, for one already-minted
            // copy. `required_target` (not a bare `Timing::Spell` ability scan) so a copied
            // permanent spell — an Aura's "enchant creature" is a cast-target requirement, not a
            // spell-timed effect (CR 303.4a/601.2c) — gets a real retarget spec too (Changing
            // Loyalty's Replicate copies, CR 702.108b/707.10a).
            Effect::Copy(CopyEffect::RetargetSpellCopy { copy }) => {
                let def = self.def_of(copy);
                let spec = self.required_target(def, None);
                if spec == TargetSpec::None {
                    return;
                }
                // A permanent's cast-target requirement carries no `Timing::Spell` effect to read
                // a count off — it's always exactly one target (CR 303.4a). An instant/sorcery
                // keeps its own effect's declared count (Twinflame's Strive-scaled retarget, etc).
                let count = def
                    .abilities
                    .iter()
                    .find(|a| matches!(a.timing, Timing::Spell))
                    .map_or(TargetCount::default(), |a| a.effect.target_count());
                self.choose_spell_targets(copy, spec, count, controller, controller, events);
            }
            // Chain Lightning's reflexive rider: "that player or that permanent's controller may
            // pay {cost}." Reads the enclosing `Sequence`'s shared `target` (the preceding
            // `DealDamage` step's own target) to find the payer — a player target pays
            // themself; a permanent target's controller pays. A missing target (CR 608.2b) is
            // unreachable in practice — the enclosing spell's own upfront target-legality check
            // already fizzles the whole ability before this step could run without one — but
            // stays a defensive no-op rather than a panic, matching this resolution's other
            // guard arms.
            Effect::Copy(CopyEffect::MayPayToCopyThis { cost, count }) => {
                let payer = match target {
                    Some(Target::Player(p)) => Some(p),
                    Some(Target::Object(id)) => Some(self.controller_of(id)),
                    None => None,
                };
                let Some(payer) = payer else {
                    return;
                };
                pending::raise_choice(
                    self,
                    PendingChoice::PayCost {
                        player: payer,
                        source,
                        cost,
                        effect: Effect::Copy(CopyEffect::ThisSpell {
                            count,
                            cast_from_graveyard_only: false,
                            optional: false,
                        }),
                    },
                );
            }
            // Willbender (CR 114.6 / 702.37f) / Wild Ricochet (CR 114.6a). The bent spell is this
            // ability's own chosen target (CR 603.3d for Willbender's trigger; the cast target for
            // Wild Ricochet), already re-checked legal by CR 608.2b before this ran (so a spell that
            // left the stack fizzles). Guard the shape defensively.
            Effect::Copy(CopyEffect::ChangeTargetOfTargetSpellOrAbility { optional, .. }) => {
                let Some(Target::Object(spell)) = target else {
                    return;
                };
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                // Wild Ricochet (CR 114.6a): "you may choose new targets for target instant or
                // sorcery spell" — no must-differ requirement (the bent spell's current target(s)
                // stay legal, so re-picking them is how a player declines), and reaches every one
                // of its independent target clauses, reusing the exact clause-chaining machinery a
                // fresh cast or `CopyTargetSpell`'s own copy-retarget already runs. This ability's
                // own controller chooses; legality is evaluated from the bent spell's own
                // controller's perspective (retargeting never changes whose "you" the spell's own
                // text refers to) — same anchor/chooser split Willbender's mandatory path below
                // already keeps.
                if optional {
                    let def = self.def_of(spell);
                    let Some((spec, count)) = self.spell_primary_target(def) else {
                        return;
                    };
                    let anchor = self.spell(spell).controller;
                    self.choose_spell_targets(spell, spec, count, anchor, controller, events);
                    return;
                }
                // Willbender: the bent spell's own single target clause, and its currently-legal
                // targets computed for the SPELL's controller (CR 114.6 — the new target must be
                // legal for *that* spell), minus its current target (CR 114.6b — the target must
                // change).
                let def = self.def_of(spell);
                let spec = def
                    .abilities
                    .iter()
                    .find(|a| matches!(a.timing, Timing::Spell))
                    .map_or(TargetSpec::None, |a| a.effect.target());
                let spell_controller = self.spell(spell).controller;
                let current = self.spell_target(spell);
                let legal: Vec<Target> = self
                    .legal_targets_for(
                        spec,
                        spell,
                        spell_controller,
                        color_identity(def),
                        self.spell(spell).x,
                    )
                    .into_iter()
                    .filter(|&t| Some(t) != current)
                    .collect();
                // CR 114.6b: no legal alternate — the target is left unchanged (no pause).
                if legal.is_empty() {
                    return;
                }
                // Willbender's controller chooses; the answer overwrites the stored single target
                // via `Event::SpellTargetsChosen` (the same write-back a multi-target choice uses).
                crate::pending::raise_choice(
                    self,
                    PendingChoice::ChooseSpellTargets {
                        player: controller,
                        spell,
                        min: 1,
                        max: 1,
                        legal,
                        clause: 0,
                    },
                );
            }
            // Thunderclap Drake's delayed one-shot: copy the spell that fired the armed
            // `ScheduleNextCastTrigger` watch (baked in as `triggering_spell` at trigger
            // placement — see `fill_triggering_spell`), not a chosen target or this ability's
            // own spell.
            Effect::Copy(CopyEffect::CopyTriggeringSpell {
                triggering_spell,
                count,
                may_choose_new_targets,
                last_known_information,
            }) => {
                let Some(original) = triggering_spell else {
                    return;
                };
                let still_on_stack = matches!(self.objects[original as usize], Object::Spell(_));
                if !still_on_stack {
                    // CR 603.4: the triggering spell may have left the stack (countered in
                    // response) before this trigger resolved. A true Storm keyword (CR 702.40a)
                    // copies it anyway, from its last-known copiable characteristics — the object
                    // is still there under a different `Object` variant (`SpellCopied`'s apply
                    // already handles a non-Spell `original`, the same fallback Surge to
                    // Victory's "copy the exiled card" relies on) — only bail outright if the
                    // object left the game entirely (`def_of` would panic). Every other consumer
                    // (Thunderclap Drake) keeps the plain "nothing left to copy" no-op.
                    if !last_known_information
                        || matches!(self.objects[original as usize], Object::Removed)
                    {
                        return;
                    }
                }
                if may_choose_new_targets {
                    self.mint_spell_copies(count, controller, original, target, x, events);
                    return;
                }
                // CR 707.10c declined: mint each copy keeping the triggering spell's own
                // targets, no retarget pause. Not exercised by any pool card yet.
                let n = self.resolve_count(count, controller, source, target, x);
                for _ in 0..n {
                    let copy = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::SpellCopied {
                            copy,
                            original,
                            controller,
                        },
                    );
                }
            }
            // Mirrorwing Dragon's watch payoff (CR 707.10): "that player copies that spell for
            // each other creature they control that the spell could target. Each copy targets a
            // different one of those creatures." Same CR 603.4 "already left the stack" guard as
            // `CopyTriggeringSpell` above.
            Effect::Copy(CopyEffect::CopyTriggeringSpellForEachOtherCreatureYouControl {
                triggering_spell,
            }) => {
                let Some(original) = triggering_spell else {
                    return;
                };
                if !matches!(self.objects[original as usize], Object::Spell(_)) {
                    return;
                }
                // "That player copies" — the copies are made under the SPELL's controller, not
                // this triggered ability's controller (`controller` here is Mirrorwing's).
                let spell_controller = self.spell(original).controller;
                let def = self.def_of(original);
                let spec = def
                    .abilities
                    .iter()
                    .find(|a| matches!(a.timing, Timing::Spell))
                    .map_or(TargetSpec::None, |a| a.effect.target());
                let legal = self.legal_targets_for(
                    spec,
                    original,
                    spell_controller,
                    color_identity(def),
                    self.spell(original).x,
                );
                // ponytail: "could target" is read as "a creature the spell's controller
                // controls, other than the original target (`source`, this triggered ability's
                // Mirrorwing), that's a legal target of the spell's own spec" — exact for the
                // pool's single-target instant/sorcery consumers.
                let creatures: Vec<ObjectId> = legal
                    .into_iter()
                    .filter_map(|t| match t {
                        Target::Object(other)
                            if other != source && self.controller_of(other) == spell_controller =>
                        {
                            Some(other)
                        }
                        _ => None,
                    })
                    .collect();
                for creature in creatures {
                    let copy = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::SpellCopied {
                            copy,
                            original,
                            controller: spell_controller,
                        },
                    );
                    // "Each copy targets a different one of those creatures": the creatures are
                    // already enumerated distinctly above, so each copy's target is set directly
                    // here instead of pausing on a per-copy retarget choice (ponytail on the
                    // effect's own doc comment: engine-chosen assignment, not player-chosen).
                    self.push_apply(
                        events,
                        Event::SpellTargetsChosen {
                            spell: copy,
                            targets: TargetList::from_targets(&[Target::Object(creature)]),
                            clause: 0,
                        },
                    );
                }
            }
            // Unbound Flourishing (CR 707.10): "copy that ability" — copy the activated ability
            // that fired the watch (its source baked in as `triggering_ability`). The copy goes on
            // the stack above the original (CR 707.10c), carrying its effect/target and its chosen
            // `{X}` unchanged (CR 706.10 — an already-doubled X isn't re-doubled).
            Effect::Copy(CopyEffect::CopyTriggeringAbility {
                triggering_ability,
                may_choose_new_targets,
            }) => {
                let Some(original) = triggering_ability else {
                    return;
                };
                // CR 603.4/707.10c: the triggering ability may have left the stack (countered in
                // response) before this watch's trigger resolved — nothing left to copy. The watch
                // sits directly above the original (CR 603.3b), so it's the topmost stack ability
                // with that source.
                let Some((copied_effect, copied_target, copied_x, copied_activated)) =
                    self.stack.iter().rev().find_map(|item| match *item {
                        StackItem::Ability {
                            source,
                            effect,
                            target,
                            x,
                            activated,
                            ..
                        } if source == original => Some((effect, target, x, activated)),
                        _ => None,
                    })
                else {
                    return;
                };
                // CR 707.10c: "you may choose new targets for the copy" — a real re-pick when the
                // copied ability actually targets (Nin, the Pain Artist's "target creature" is
                // exactly the targeted `{X}`-cost activated ability this makes observable);
                // `place_targeted_ability` re-derives the target fresh (protection included, CR
                // 702.16b), threading the copy's own `{X}`/activated-ness onto the eventual push.
                // Declining (`may_choose_new_targets = false`) or a targetless copy keeps the
                // original's target(s) unchanged below — CR 707.10c's declined case.
                if may_choose_new_targets && copied_effect.target() != TargetSpec::None {
                    self.place_targeted_ability(
                        controller,
                        original,
                        copied_effect,
                        copied_x,
                        copied_activated,
                        events,
                    );
                    return;
                }
                // The copy is the same kind of ability as the original (CR 707.10c) — an activated
                // copy stays activated, a triggered copy stays triggered. Its spent-mana multiset
                // is empty: a copy is created, not activated, so no mana was spent on it (the same
                // line converge's copy ruling draws for spells).
                self.push_ability_group_with_x(
                    controller,
                    original,
                    &[(copied_effect, copied_target)],
                    copied_x,
                    [0; 6],
                    copied_activated,
                    events,
                );
            }
            // Surge to Victory's combat-damage-copy step: mint one free copy of the card the
            // arming watch names — the exile already happened when the watch was armed, so this
            // only mints. `card` is `None` only if this were ever misfired with no armed card,
            // which `fire_combat_damage_copy_triggers` never does.
            Effect::Copy(CopyEffect::MintFreeCopyOfExiledCard { card }) => {
                let Some(card) = card else {
                    return;
                };
                self.mint_spell_copies(Amount::Fixed(1), controller, card, None, 0, events);
            }
            _ => unreachable!("copy family choreo received a non-family effect"),
        }
    }

    /// Mint `count` copies of `source` (`Effect::Copy(CopyEffect::ThisSpell)`'s mandatory mint, and
    /// `Game::answer_may`'s inline "yes" for its `optional` gate): mint every copy up front —
    /// minting itself never pauses, and `source` (this resolving spell) is still on the stack for
    /// all of them, unlike a queued per-copy mint that would resume after `source` has already
    /// left (moved to the graveyard by this same resolution's trailing steps). Each copy's own CR
    /// 707.10c retarget (which *does* pause) is then queued one at a time behind
    /// `run_sequence`'s pause/resume machinery via `RetargetSpellCopy`, so one copy's
    /// `ChooseSpellTargets` doesn't clobber another's.
    pub(crate) fn mint_spell_copies(
        &mut self,
        count: Amount,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        events: &mut Vec<Event>,
    ) {
        let n = self.resolve_count(count, controller, source, target, x);
        if n == 0 {
            return;
        }
        let copies: Vec<Effect> = (0..n)
            .map(|_| {
                let copy = self.next_object_id();
                self.push_apply(
                    events,
                    Event::SpellCopied {
                        copy,
                        original: source,
                        controller,
                    },
                );
                Effect::Copy(CopyEffect::RetargetSpellCopy { copy })
            })
            .collect();
        self.run_sequence(
            Box::leak(copies.into_boxed_slice()),
            ResolveCtx {
                controller,
                source,
                target,
                targets_second: TargetList::default(),
                x,
                spent_mana: [0; 6],
            },
            events,
        );
    }
}
