//! Stack resolution payoffs — applying effects when spells and abilities resolve.
//!
//! Primary: CR 608 (resolving spells and abilities). Deferred / gaps: see
//! `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// Expand a multi-target spell ability into one `(ability, target)` step per chosen target
/// (CR 601.2c) — or, when its count is fully declinable (`min == 0`), the caster chose none, and
/// [`Effect::has_target_independent_step`] says some step still does something without it, a
/// single `None`-target step instead of zero. That keeps a `Sequence` ability alive when its
/// leading multi-target clause is declined but a later, untargeted step still needs to run
/// (Zimone's Hypothesis' "you may put a +1/+1 counter on a creature" primer ahead of the
/// untargeted mass parity-bounce — declining the counter shouldn't cancel the bounce). Mirrors
/// the same rule already applied to a declined single "up to one" *triggered* target (Kinetic
/// Ooze) — this is its multi-target-spell counterpart. The `run()` step-level guard already
/// no-ops an individual `None`-target step for us (see its own doc); this only has to stop
/// dropping the *whole* ability outright.
fn multi_target_steps(a: Ability, targets: TargetList) -> Vec<(Ability, Option<Target>)> {
    if targets.iter().next().is_some() {
        return targets.iter().map(|t| (a, Some(t))).collect();
    }
    if a.effect.target_count().min == 0 && a.effect.has_target_independent_step() {
        return vec![(a, None)];
    }
    Vec::new()
}

impl Game {
    /// Resolve the top item of the stack, applying its events into `events`. Resolution
    /// applies incrementally so newly-minted object ids stay in sync with the arena.
    pub(crate) fn resolve_top(&mut self, events: &mut Vec<Event>) {
        match *self.stack.last().expect("stack is non-empty") {
            StackItem::Spell(object) => self.resolve_spell(object, events),
            StackItem::Ability {
                controller,
                source,
                effect,
                target,
                targets_second,
                x: activation_x,
                spent_mana,
                activated: _,
            } => {
                // CR 608.2b: an ability whose stored target is no longer legal fizzles —
                // it leaves the stack with no effect. Targeted abilities pass no source
                // colors, mirroring the enumeration `legal_targets` used at activation.
                // The target-legality `{X}` is the ability's source's own entered X (see
                // `Game::ability_source_x`) — needed for a `mv_max_x` re-check (Kinetic Ooze),
                // 0 for every other ability; distinct from the *activation* `{X}` below (Unbound
                // Flourishing's copied {X} ability, CR 107.3), which no pool `mv_max_x` reads.
                let legality_x = self.ability_source_x(source);
                if !self.target_still_legal(
                    effect.target(),
                    source,
                    target,
                    controller,
                    [false; Color::COUNT],
                    legality_x,
                ) {
                    self.push_apply(events, Event::AbilityResolved { source });
                    return;
                }
                // The ability leaves the stack as it resolves (CR 608), *before* its effect
                // runs — an effect that itself pushes a new stack item (Rootha's
                // `CopyTargetSpell`, minting a copy spell on top of the stack) must land on top
                // of whatever's left once this ability is gone, not underneath it.
                self.push_apply(events, Event::AbilityResolved { source });
                // A triggered ability carries `x = 0`; an activated (or copied) ability whose
                // cost contains `{X}` resolves its `Amount::X` against the chosen value
                // (CR 107.3). A pausing effect leaves a PendingChoice behind.
                self.run(
                    effect,
                    ResolveCtx {
                        controller,
                        source,
                        target,
                        targets_second,
                        x: activation_x,
                        spent_mana,
                    },
                    events,
                );
            }
        }
    }

    /// Resolve a cast spell: a creature/enchantment enters; an instant/sorcery runs its
    /// effects then goes to the graveyard.
    pub(crate) fn resolve_spell(&mut self, object: ObjectId, events: &mut Vec<Event>) {
        let spell = *self.spell(object);
        // A bestowed spell (CR 702.103d) resolves as an Aura — it enters attached to its target
        // through the same path a `CardKind::Aura` spell uses, not as a creature (its printed
        // `kind` stays `Creature` for when it later stops being attached, CR 702.103i).
        let kind = if spell.bestowed {
            CardKind::Aura
        } else {
            spell.def.kind
        };
        match kind {
            CardKind::Creature { .. }
            | CardKind::Enchantment
            | CardKind::Artifact
            | CardKind::Planeswalker { .. } => {
                // Animate Dead (CR 303.4a/608.2b): its own cast-time "enchant creature card in a
                // graveyard" target can fizzle the same way an Aura's battlefield host can — an
                // opponent exiling the chosen graveyard card in response leaves it with no legal
                // object, so it goes to the graveyard (or ceases to exist, if it's a copy)
                // instead of entering unattached. The pool's only non-Aura kind with a cast-time
                // target, so this re-check is scoped to `enchant_graveyard` rather than folded
                // into the `CardKind::Aura` fizzle branch below.
                if spell.def.enchant_graveyard
                    && !self.target_still_legal(
                        TargetSpec::CreatureCardInAnyGraveyard,
                        object,
                        spell.targets.primary(),
                        spell.controller,
                        color_identity(spell.def),
                        spell.x,
                    )
                {
                    if spell.copy {
                        self.push_apply(events, Event::SpellCeasedToExist { spell: object });
                        return;
                    }
                    self.push_apply(
                        events,
                        Event::MovedToGraveyard {
                            card: self.next_object_id(),
                            from: object,
                        },
                    );
                    return;
                }
                let entered = self.next_object_id();
                self.push_apply(
                    events,
                    Event::PermanentEntered {
                        permanent: entered,
                        from: object,
                    },
                );
                // Devour N (CR 702.82): pause as the creature enters so its controller may
                // sacrifice any number of the other creatures they control; the counters are
                // applied when that choice is answered (see `Game::answer_devour`). With no other
                // creature to give up there's nothing to choose — resolution runs on unpaused.
                if let Some(multiplier) = spell.def.devour {
                    self.begin_devour(spell.controller, entered, multiplier);
                    if self.resolution_is_paused() {
                        return;
                    }
                }
                // Enter-as-a-copy (CR 706/707.2 — Altered Ego, Cursed Mirror): pause as the
                // permanent enters (before the enters-with-counters / ETB steps, CR 616) so its
                // controller may have it become a copy of a battlefield creature; the copy, extra
                // counters, and haste are applied when that choice is answered (see
                // `Game::answer_enter_as_copy`). With no creature to copy there's nothing to
                // choose — resolution runs on unpaused.
                if let Some(marker) = spell.def.enter_as_copy {
                    self.begin_enter_as_copy(spell.controller, entered, marker);
                    if self.resolution_is_paused() {
                        return;
                    }
                }
                // "Enters with N +1/+1 counters" (hydras: N = the spell's {X}) — placed as the
                // permanent enters and grown by any counter-replacement static (Hardened Scales,
                // a doubler), reading the just-entered permanent's controller. "Enters with N
                // `kind` counters" (mana_bloom/astral_cornucopia) instead places the raw amount
                // in the kind-keyed map — no replacement static touches a named kind.
                if let Some((amount, kind)) = enters_with_counters(spell.def) {
                    let counters = self.resolve_count(
                        amount,
                        spell.controller,
                        entered,
                        spell.targets.primary(),
                        spell.x,
                    );
                    match kind {
                        None => {
                            let n = self.counters_after_replacements(entered, counters as i32);
                            if n > 0 {
                                self.push_apply(
                                    events,
                                    Event::CountersPlaced {
                                        object: entered,
                                        count: n,
                                        source_name: spell.def.name,
                                    },
                                );
                            }
                        }
                        Some(kind) if counters > 0 => {
                            self.push_apply(
                                events,
                                Event::KindCountersPlaced {
                                    object: entered,
                                    kind,
                                    count: counters as i32,
                                },
                            );
                        }
                        Some(_) => {}
                    }
                }
                // "Nontoken creatures you control enter with an additional +1/+1 counter on
                // them for each creature that died under your control this turn." (Gorma, the
                // Gullet, CR 614.1c): scan the caster's own battlefield for every static
                // `CreaturesYouControlEnterWithCounters` ability on another permanent that matches
                // the just-entered permanent (a static never modifies its own permanent's entry —
                // see `Game::additional_enter_counters`'s doc), sum, and place through the same
                // doubler/Hardened-Scales replacement pipeline as any other counter placement.
                // ponytail: only wired at this cast-resolution choke — a reanimated or blinked-in
                // nontoken creature doesn't pick up the bonus (no pool card observes that path;
                // extend to `ReanimateToBattlefield`'s own PermanentEntered if one needs it).
                let bonus = self.additional_enter_counters(entered, spell.controller);
                let n = self.counters_after_replacements(entered, bonus);
                if n > 0 {
                    self.push_apply(
                        events,
                        Event::CountersPlaced {
                            object: entered,
                            count: n,
                            source_name: spell.def.name,
                        },
                    );
                }
                // Opal Palace's spend-to-cast rider: additional +1/+1 counters this specific spell
                // was told to enter with at cast payment (captured by
                // `Effect::CommanderEntersWithBonusCounters`, keyed by this spell's stack id). Runs
                // through the same counter-replacement statics (Hardened Scales, a doubler) as the
                // printed `enters_with_counters` above.
                if let Some(pos) = self
                    .pending_enter_bonus_counters
                    .iter()
                    .position(|&(id, _)| id == object)
                {
                    let (_, bonus) = self.pending_enter_bonus_counters.remove(pos);
                    let n = self.counters_after_replacements(entered, bonus as i32);
                    if n > 0 {
                        self.push_apply(
                            events,
                            Event::CountersPlaced {
                                object: entered,
                                count: n,
                                source_name: spell.def.name,
                            },
                        );
                    }
                }
                // "Escapes with N +1/+1 counters" (CR 702.19c — Woe Strider): unlike a hydra's
                // unconditional `enters_with_counters`, this only applies when the permanent was
                // actually cast via escape (a card with escape usually has a normal cast mode (CR 702.19, CR 601)
                // too, which gets no counters).
                if spell.escape
                    && let Some(escape) = spell.def.escape
                    && escape.plus_one_plus_one_counters > 0
                {
                    let n = self.counters_after_replacements(
                        entered,
                        escape.plus_one_plus_one_counters as i32,
                    );
                    if n > 0 {
                        self.push_apply(
                            events,
                            Event::CountersPlaced {
                                object: entered,
                                count: n,
                                source_name: spell.def.name,
                            },
                        );
                    }
                }
            }
            CardKind::Aura => {
                let host = expect_object_target(spell.targets.primary(), "an Aura");
                // CR 303.4f: if the target is illegal as the Aura would resolve, the Aura
                // stays on the stack and goes to the graveyard — it doesn't enter. The re-check
                // is the same enchant filter cast-target legality used (an "Enchant creature you
                // control" Aura fizzles if its host's controller changed in response).
                // ponytail: an escaping Aura whose target turns illegal should exile here too
                // (CR 702.19d), not go to the graveyard — always graveyard-bound for now. No pool
                // escape Aura's target realistically fizzles in a test, so this residual is (CR 702.19, CR 303.4, CR 601.2c)
                // untested; extend with an `spell.escape` check if one needs it. (CR 702.19, CR 601)
                if !self.attachment_host_legal(object, host) {
                    // CR 707.10a/111.7: a copy that fails to resolve never becomes a card — it
                    // just ceases to exist, mirroring `finish_instant_sorcery_resolution`'s own
                    // copy guard (Changing Loyalty's Replicate copies, retargeted onto a creature
                    // that's since become illegal).
                    if spell.copy {
                        self.push_apply(events, Event::SpellCeasedToExist { spell: object });
                        return;
                    }
                    self.push_apply(
                        events,
                        Event::MovedToGraveyard {
                            card: self.next_object_id(),
                            from: object,
                        },
                    );
                    return;
                }
                let permanent = self.next_object_id();
                self.push_apply(
                    events,
                    Event::PermanentEntered {
                        permanent,
                        from: object,
                    },
                );
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: permanent,
                        host: Some(host),
                    },
                );
            }
            CardKind::Spell { .. } => {
                // A modal spell runs only its chosen modes, in printed order, each with its own
                // target (CR 700.2 — modes are validated at cast, so `nth_mode` is `Some`); a
                // non-modal spell runs every one of its spell abilities against its single target.
                let steps: Vec<(Ability, Option<Target>)> = if spell.def.modal {
                    // A chosen mode's own effect may itself be multi-target (Prismari Charm
                    // mode 1's "one or two targets"): its per-mode `target` slot is `None` (the
                    // cast gate routed its targets through `spell.targets` instead — see
                    // `Game::modal_multi_target`), so expand it the same way the non-modal branch
                    // below expands a multi-target spell.
                    spell
                        .modes
                        .chosen()
                        .filter_map(|(i, target)| nth_mode(spell.def, i).map(|a| (a, target)))
                        .flat_map(|(a, target)| {
                            if a.effect.target_count().is_single() {
                                vec![(a, target)]
                            } else {
                                multi_target_steps(a, spell.targets)
                            }
                        })
                        .collect()
                } else {
                    // A non-modal spell runs each spell ability against its target(s). A
                    // multi-target effect (Aether Gale's `ReturnToHand { count: {6, 6} }`) is
                    // expanded into one single-target step per chosen target, so the shared
                    // resolution loop below re-checks each for legality (CR 608.2b) and applies
                    // the effect independently; a single-target ability keeps its lone target.
                    // Each multi-target ability, in printed order, reads its *own* independent
                    // target clause (Magma Opus's damage clause 0, tap clause 1).
                    let mut clause = 0usize;
                    spell
                        .def
                        .abilities
                        .iter()
                        .copied()
                        .filter(|a| matches!(a.timing, Timing::Spell))
                        .flat_map(|a| {
                            if a.effect.target_count().is_single() {
                                return vec![(a, spell.targets.primary())];
                            }
                            // ponytail: two independent clauses (0 → `targets`, 1 → `targets_second`);
                            // a third clause would need a `[TargetList; N]` — no pool spell prints one.
                            let list = if clause == 0 {
                                spell.targets
                            } else {
                                spell.targets_second
                            };
                            clause += 1;
                            multi_target_steps(a, list)
                        })
                        .collect()
                };
                // CR 608.2b: if the spell has at least one targeted step and every one of them
                // is now illegal, the whole spell fails to resolve — including its untargeted
                // rider steps (Infernal Grasp's "you lose 2 life" doesn't charge if the destroy
                // half's target already left the battlefield). A spell with no targeted steps at
                // all is untouched by this check (`targeted.peek()` is `None`).
                let mut targeted = steps
                    .iter()
                    .filter(|(a, _)| a.effect.target() != TargetSpec::None)
                    .peekable();
                let all_targets_illegal = targeted.peek().is_some()
                    && !targeted.any(|(a, t)| {
                        self.target_still_legal(
                            a.effect.target(),
                            object,
                            *t,
                            spell.controller,
                            color_identity(spell.def),
                            spell.x,
                        )
                    });
                if !all_targets_illegal {
                    for (ability, target) in steps {
                        // CR 608.2b/c: a step whose stored target is no longer legal is skipped —
                        // the spell fizzles for that effect (an instant/sorcery still finishes to
                        // the graveyard below). Same enumeration as the cast gate, so zone changes,
                        // protection, and player elimination all count.
                        if !self.target_still_legal(
                            ability.effect.target(),
                            object,
                            target,
                            spell.controller,
                            color_identity(spell.def),
                            spell.x,
                        ) {
                            continue;
                        }
                        self.run(
                            ability.effect,
                            ResolveCtx {
                                controller: spell.controller,
                                source: object,
                                target,
                                targets_second: TargetList::default(),
                                x: spell.x,
                                spent_mana: [0; 6],
                            },
                            events,
                        );
                    }
                }
                // A resolution-time optional rider on this spell's own ability (Sevinne's
                // Reclamation's "you may copy this spell") paused mid-resolution — leave this
                // spell as a live `Object::Spell` on the stack until that choice is answered
                // (`Game::pending_spell_finish`), rather than moving it to its post-resolution
                // zone out from under its own still-open decision.
                if self.resolution_is_paused() {
                    self.pending_spell_finish = Some(object);
                    return;
                }
                self.finish_instant_sorcery_resolution(object, events);
            }
            CardKind::Land { .. } => {
                unreachable!("lands are played directly to the battlefield, never resolved")
            }
        }
    }

    /// Move a resolved instant/sorcery `object` to its post-resolution zone: ceases to exist if
    /// it's a copy (CR 707.10a), exile if it was cast via flashback/escape (CR 702.34e/702.19d),
    /// its owner's hand if it was bought back (CR 702.27d), else the graveyard. Split out of
    /// [`Self::resolve_spell`] so [`Game::resume_deferred_sequence`] can also call it once a
    /// [`Game::pending_spell_finish`] pause clears.
    pub(crate) fn finish_instant_sorcery_resolution(
        &mut self,
        object: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let spell = *self.spell(object);
        // A copy ceases to exist (CR 707.10a); a cast instant/sorcery goes to the graveyard.
        if spell.copy {
            self.push_apply(events, Event::SpellCeasedToExist { spell: object });
            return;
        }
        // CR 715.3d: an adventure spell is exiled "on an adventure" (as the creature front face,
        // not the spent adventure face), and its owner may cast the creature from exile later.
        if self
            .play_permissions
            .adventure_fronts
            .iter()
            .any(|&(id, _)| id == object)
        {
            self.push_apply(
                events,
                Event::ExiledOnAdventure {
                    card: self.next_object_id(),
                    from: object,
                    owner: self.owner_of(object),
                },
            );
            return;
        }
        // CR 702.34e/CR 702.19d — a flashback or escape spell is exiled as it leaves the
        // stack, not put into the graveyard. (Only reachable for an instant/sorcery: a
        // permanent's escape spell resolves through the `CardKind::Creature`/`Aura` arms (CR 702.19, CR 303.4, CR 601)
        // above instead, entering the battlefield rather than reaching this graveyard
        // path — no pool escape card is a non-permanent, so this branch is exercised only (CR 702.19)
        // by flashback today.)
        if spell.flashback || spell.escape {
            self.push_apply(
                events,
                Event::MovedToExile {
                    card: self.next_object_id(),
                    from: object,
                },
            );
            return;
        }
        // Buyback (CR 702.27d): "If you do, put this card into your hand as it resolves" (Capsize)
        // — the resolved spell returns to its owner's hand instead of the graveyard below.
        if spell.bought_back {
            self.push_apply(
                events,
                Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                },
            );
            return;
        }
        // Rousing Refrain's "Exile [this card] with three time counters on it" (CR 702.62): an
        // `Effect::ExileSelfWithTimeCounters` step this resolution ran marked the spell to exile
        // itself (with counters) rather than reach the graveyard below.
        if let Some(counters) = self.self_exile_time_counters.take() {
            self.push_exile_with_time_counters(object, counters, events);
            return;
        }
        // Quintorius, Loremaster's CR 614.6 rider: "If that spell would be put into a graveyard,
        // put it on the bottom of its owner's library instead." `object` is the spell's live id;
        // the flag was recorded against its pre-cast exile id, so match through `current_id`.
        if let Some(pos) = self
            .play_permissions
            .stack_object_bottoms_library_on_leave
            .iter()
            .position(|&flagged| self.current_id(flagged) == object)
        {
            self.play_permissions
                .stack_object_bottoms_library_on_leave
                .remove(pos);
            self.push_apply(
                events,
                Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top: false,
                },
            );
            return;
        }
        self.push_apply(
            events,
            Event::MovedToGraveyard {
                card: self.next_object_id(),
                from: object,
            },
        );
    }

    /// Move the card object `from` to exile with `counters` time counters on it (CR 702.62 —
    /// suspend), the shared choke for Rousing Refrain's on-resolution self-exile and a suspend
    /// cast from hand. The new exile object carries the counters in
    /// [`Game::exile_time_counters`](crate::Game).
    pub(crate) fn push_exile_with_time_counters(
        &mut self,
        from: ObjectId,
        counters: u32,
        events: &mut Vec<Event>,
    ) {
        let card = self.next_object_id();
        self.push_apply(events, Event::MovedToExile { card, from });
        self.push_apply(
            events,
            Event::TimeCountersPlaced {
                card,
                count: counters,
            },
        );
    }

    /// Counter `spell` (CR 701.5a): move it from the stack to its owner's graveyard, so it never
    /// resolves. A no-op if `spell` already left the stack (CR 608.2b) — a response emptied that
    /// stack slot (countered/resolved) before this counter could act. Shared by the unconditional
    /// [`Effect::CounterTargetSpell`] arm and the [`PendingChoice::PayOrCounter`] decline handler.
    /// ponytail: a still-on-stack *copy* is treated like any spell and sent to a graveyard rather
    /// than ceasing to exist (CR 707.10a); no pool card counters a copy, so the distinction never
    /// surfaces.
    pub(crate) fn counter_spell(&self, spell: ObjectId) -> Vec<Event> {
        if !matches!(self.objects[spell as usize], Object::Spell(_)) {
            return Vec::new();
        }
        // CR 701.5g: "this spell can't be countered" — the counter fizzles and the spell
        // stays on the stack, unaffected.
        if self.def_of(spell).uncounterable {
            return Vec::new();
        }
        // CR 702.34e/CR 702.19d: a flashback or escape spell exiles "as it leaves the stack" —
        // countered is one such departure, same as resolving (see
        // `finish_instant_sorcery_resolution`'s twin check). Checked before the Quintorius rider
        // below: a flashback/escape spell never reaches a graveyard in the first place, so
        // Quintorius's "would be put into a graveyard" redirect doesn't apply to it either.
        let countered = self.spell(spell);
        if countered.flashback || countered.escape {
            return vec![Event::MovedToExile {
                card: self.next_object_id(),
                from: spell,
            }];
        }
        // Quintorius, Loremaster's CR 614.6 rider (see `finish_instant_sorcery_resolution`'s
        // twin check) — "would be put into a graveyard" covers the countered case too. `&self`
        // can't drain the flag here; it lingers until the unconditional cleanup clear, and a
        // countered spell can't also resolve, so it never double-matches.
        if self
            .play_permissions
            .stack_object_bottoms_library_on_leave
            .iter()
            .any(|&flagged| self.current_id(flagged) == spell)
        {
            return vec![Event::TuckedToLibrary {
                card: self.next_object_id(),
                from: spell,
                to_top: false,
            }];
        }
        vec![Event::MovedToGraveyard {
            card: self.next_object_id(),
            from: spell,
        }]
    }

    /// CR 608.2b: whether a spell/ability's stored target is still a legal choice for
    /// `spec` as it would resolve. Untargeted resolutions are trivially fine. Re-runs
    /// the same enumeration the cast/activation gate used ([`Game::legal_targets_for`]),
    /// so "legal" cannot drift between choice time and resolution.
    fn target_still_legal(
        &self,
        spec: TargetSpec,
        source: ObjectId,
        target: Option<Target>,
        controller: PlayerId,
        source_colors: [bool; Color::COUNT],
        x: u32,
    ) -> bool {
        if spec == TargetSpec::None {
            return true;
        }
        let Some(chosen) = target else {
            // A targeted spec with no stored choice never re-checks (nothing was targeted).
            return true;
        };
        self.legal_targets_for(spec, source, controller, source_colors, x)
            .contains(&chosen)
    }

    /// The shared core of "double `object`'s +1/+1 counters" (CR 614): as many more as it
    /// already has, through the same replaceable-step pipeline [`Effect::PutCounters`] uses.
    /// `None` when doubling is a no-op — zero counters, or a replacement effect zeroes the
    /// result out — the same "no event for a no-op doubling" rule
    /// [`Effect::DoubleCounters`] and [`Effect::DoubleCountersOnAttachedCreature`] both follow.
    pub(crate) fn doubled_counters_event(
        &self,
        object: ObjectId,
        source_name: &'static str,
    ) -> Option<Event> {
        let current = self.permanent(object).plus_counters;
        let n = self.counters_after_replacements(object, current);
        (n > 0).then_some(Event::CountersPlaced {
            object,
            count: n,
            source_name,
        })
    }

    /// Resolve one effect — the sole call-site verb for Effect → board mutation (ADR 0004).
    /// A pausing effect sets `pending_choice` (via its `begin_*` helper); every other effect
    /// mints events, applies them, and appends to `events`. Callers never choose between a
    /// pure mint path and a mut path; composites, snapshots, and RNG all go through here.
    ///
    /// ponytail: a pausing effect is assumed self-contained — a spell that scries *then*
    /// does more (Preordain's "then draw a card") would need the remaining effects
    /// deferred to the choice's answer. No such multi-effect card is in the pool.
    pub(crate) fn run(&mut self, effect: Effect, ctx: ResolveCtx, events: &mut Vec<Event>) {
        let ResolveCtx {
            controller,
            source,
            target,
            targets_second,
            x,
            spent_mana,
        } = ctx;
        // A targeted step with no chosen target only reaches resolution via an "up to one"
        // ability placed with no target (declined, or none legal — CR 601.2c/603.3c, see
        // `Game::place_targeted_ability`/`Game::choose_targets`): a no-op for this step, distinct
        // from the enclosing `Sequence`'s other, target-independent steps (Kinetic Ooze's
        // X-threshold riders), which still run. `Sequence`/`Conditional` themselves are excluded:
        // their own `target()` is *derived* from their steps (see `Effect::target`), not a real
        // requirement, so this guard must let them through to dispatch their steps individually.
        if target.is_none()
            && effect.target() != TargetSpec::None
            && !matches!(effect, Effect::Sequence { .. } | Effect::Conditional { .. })
        {
            return;
        }
        match effect {
            // Scry/surveil pause on an ArrangeTop choice: the non-kept pile goes to the
            // library bottom (scry) or the graveyard (surveil).
            Effect::Scry { count } => {
                let count = self.resolve_count(count, controller, source, target, x);
                self.begin_arrange_top(controller, count, false)
            }
            Effect::Surveil { count } => self.begin_arrange_top(controller, count, true),
            // Look at the top N, select up to `up_to` matching cards into `dest`, rest to `rest`
            // (Quandrix Apprentice). Pauses on a SelectFromTop choice.
            Effect::LookAtTop {
                count,
                filter,
                up_to,
                min,
                dest,
                dest_tapped,
                rest,
                mv_budget,
            } => self.begin_look_at_top(
                controller,
                count,
                filter,
                up_to,
                min,
                dest,
                dest_tapped,
                rest,
                mv_budget,
            ),
            // Exile the top N face-up, pause on a choose-up-to-one over the matching cards to
            // grant free-cast permission, then bottom the rest (Herald of Amity's ETB dig).
            // Pauses on a ChooseExiledDigToCastFree choice.
            Effect::ExileTopCastMatchingFree { count, filter } => {
                self.begin_exile_top_cast_matching_free(controller, source, count, filter, events)
            }
            // Songbirds' Blessing: reveal-until-Aura, pausing on a battlefield-or-hand choice
            // over the match.
            Effect::RevealUntilMayDeploy { filter } => {
                self.begin_reveal_until_may_deploy(controller, filter, events)
            }
            // Creative Technique: reveal-until-nonland, pausing on the shared exiled-dig
            // may-cast-free choice over the match.
            Effect::RevealUntilExileCastFree { filter } => {
                self.begin_reveal_until_exile_cast_free(controller, source, filter, events)
            }
            // Creative Technique's "Shuffle your library, then reveal…" lead-in step.
            Effect::ShuffleLibrary => {
                self.push_apply(events, Event::LibraryShuffled { player: controller })
            }
            // Dance with Calamity: the player-driven exile-until-stop loop, then a free cast of any
            // number of the exiled cards if the tally stayed under budget. Pauses on a
            // DanceExileMore choice.
            Effect::ExileTopUntilStopCastFreeUnderBudget { budget } => {
                self.begin_dance_with_calamity(controller, source, budget, events)
            }
            // Cascade (CR 702.85): reveal-until a cheaper nonland, may cast it free, bottom the
            // rest in random order. Pauses on a ChooseExiledDigToCastFree choice (reused from the
            // dig) when a hit is found.
            Effect::Cascade { mana_value } => {
                self.begin_cascade(controller, source, mana_value, events)
            }
            // Look at the top N, route one card each to hand / bottom / exile-may-play
            // (Expressive Iteration). Pauses on a DistributeTop choice.
            Effect::DistributeTop {
                count,
                to_hand,
                to_bottom,
                to_exile_may_play,
            } => {
                self.begin_distribute_top(controller, count, to_hand, to_bottom, to_exile_may_play)
            }
            // A library search (fetchlands / tutors) pauses on a SearchLibrary choice. Usually
            // the ability's own controller searches; Path to Exile/Assassin's Trophy hand the
            // search to the exiled/destroyed permanent's controller instead (CR 701.19 doesn't
            // require the searcher to be the ability's controller).
            Effect::SearchLibrary {
                filter,
                to_zone,
                tapped,
                searcher,
                count,
                overflow,
            } => {
                let searching_player = match searcher {
                    SearchScope::You => controller,
                    SearchScope::TargetController => self.controller_of(expect_object_target(
                        target,
                        "a search effect's target-controller",
                    )),
                };
                self.begin_search_library(
                    searching_player,
                    filter,
                    to_zone,
                    tapped,
                    count,
                    overflow,
                )
            }
            // A multi-player sacrifice edict (Deadly Brew, Promise of Loyalty) pauses per
            // affected player.
            Effect::EachPlayerSacrifices {
                scope,
                keep_one,
                filter,
                life_loss,
                then,
            } => self.begin_sacrifice_edict(
                scope, keep_one, filter, life_loss, then, controller, source, events,
            ),
            // A multi-player graveyard-exile fan-out (Augusta) pauses per affected player; its
            // reflexive counter payoff rides in the enclosing `Sequence`, resumed once all answer.
            // ponytail: this "when you do" is CR 603.3b's separate reflexive trigger, modeled here
            // as a same-resolution sequenced payoff (no response window). `Effect::ReflexiveTrigger`
            // is the real-stack-object primitive; migrate to it when Augusta's "you do" condition
            // (its own exile fan-out, not a token creation) is threadable through it.
            Effect::EachPlayerExilesFromGraveyard => self.begin_each_player_exile(source),
            // Relic of Progenitus: "Target player exiles a card from their graveyard." The one-
            // player special case of the fan-out above — no `follow_up`, no payoff.
            Effect::TargetPlayerExilesFromGraveyard { .. } => {
                let Some(Target::Player(player)) = target else {
                    panic!(
                        "target player exiles from graveyard resolves with a chosen player target"
                    );
                };
                self.begin_target_player_exile(player, source)
            }
            // The caster-directed keep-one-of-each-type sweep (Tragic Arrogance): for each player,
            // the caster picks up to one nonland permanent of each type to keep; the rest are
            // sacrificed. Pauses per player on a CasterKeepPermanents choice answered by the caster.
            Effect::CasterKeepsOneOfEachTypePerPlayer => {
                self.begin_caster_keeps_per_player(controller, source)
            }
            // Nils' end step: for each player, its controller puts a +1/+1 counter on up to one
            // creature that player controls. Pauses per player on a ChooseCounterTargetForPlayer.
            Effect::EachPlayerControllerChoosesCounterTarget => {
                self.begin_each_player_counter_target(controller, source)
            }
            // Council's dilemma (Fateful Tempest): a per-player vote round pauses each seat on a
            // CastVote choice; the tally-scaled payoff rides in the enclosing `Sequence`, resumed
            // once every player has voted (the same deferred-tail path as the graveyard fan-out).
            Effect::CouncilsDilemmaVote { options } => {
                self.begin_councils_dilemma_vote(controller, source, options)
            }
            // Abstract Performance: split the top eight into two piles, an opponent picks one,
            // pausing on an OpponentChoosesPile choice.
            Effect::OpponentSplitsExilePiles => {
                self.begin_opponent_splits_exile_piles(controller, source, events)
            }
            // Fact or Fiction: reveal the top five, an opponent splits them into two piles,
            // pausing on a PartitionRevealed choice.
            Effect::RevealTopSplitPiles => {
                self.begin_reveal_top_split_piles(controller, source, events)
            }
            // Plargg and Nassari: each player exiles from the top until a nonland, an opponent
            // picks one, pausing on an OpponentChoosesExiledNonland choice.
            Effect::EachPlayerExilesUntilNonlandOpponentPicks => {
                self.begin_each_player_exiles_until_nonland(controller, source, events)
            }
            // Brudiclad: "you may choose a token you control; if you do, each other token you
            // control becomes a copy of that token." Pauses on a ChooseTokenToCopy choice; with no
            // token to choose there's nothing to convert (guarded like begin_may_sacrifice).
            Effect::EachOtherTokenBecomesCopyOfChosen => {
                self.begin_each_other_token_becomes_copy(controller, source)
            }
            // Spirit of Resilience: "put a +1/+1 counter on this creature, then you may have this
            // creature become a copy of an artifact or creature card from among those cards until
            // end of turn." Places the counter, then pauses on a ChooseCopyCardFromList choice
            // over the artifact/creature cards that left; no copyable card means no pause.
            Effect::PutCounterThenMayBecomeCopyOfCardFromList { cards } => {
                self.begin_put_counter_then_may_become_copy(controller, source, cards, events)
            }
            // A resolution-time optional sacrifice (Witherbloom Charm mode 0) pauses on a
            // MaySacrifice choice; declining runs nothing.
            Effect::MaySacrifice { filter, then } => {
                self.begin_may_sacrifice(controller, source, filter, then)
            }
            // A forced sacrifice the affected player directs (Lotus Field's ETB "sacrifice two
            // lands", Smothering Abomination's upkeep "sacrifice a creature") pauses on a
            // ChooseOwnSacrifices choice; with count-or-fewer legal permanents it resolves
            // immediately instead (CR 700.2's "as many as possible").
            Effect::SacrificeOwn { filter, count } => {
                self.begin_choose_own_sacrifices(controller, source, filter, count, events)
            }
            // Annihilator N (Eldrazi Conscription): the defending player, not the controller,
            // directs the forced sacrifice — same ChooseOwnSacrifices machinery, any permanent.
            Effect::DefendingPlayerSacrifices { count, defender } => {
                let defender = defender.expect("filled from attack context when placed");
                self.begin_choose_own_sacrifices(
                    defender,
                    source,
                    PermanentFilter::default(),
                    count as u32,
                    events,
                )
            }
            // A resolution-time optional graveyard return (Deadly Brew's rider) pauses on a
            // MayReturnFromGraveyard choice; declining runs nothing. "If you sacrificed a
            // permanent this way" (Deadly Brew) gates the whole rider on the edict's own
            // controller having actually sacrificed — unmet, it's the same "runs nothing" as
            // declining, no pause at all.
            Effect::MayReturnFromGraveyard {
                filter,
                if_you_sacrificed_this_way,
            } => {
                if if_you_sacrificed_this_way && !self.sacrificed_by_edict_controller {
                    return;
                }
                self.begin_may_return_from_graveyard(controller, source, filter)
            }
            // A resolution-time optional discard (Quintorius, History Chaser's +1) pauses on a
            // MayDiscard choice; declining runs nothing.
            Effect::MayDiscard { then } => self.begin_may_discard(controller, source, then),
            // Proliferate (CR 701.27) pauses on a Proliferate choice over every counter-bearing
            // permanent; `times` (Expansion Algorithm's {X}) may re-pause after this iteration.
            Effect::Proliferate { times } => {
                let n = self.resolve_count(times, controller, source, target, x);
                self.begin_proliferate(controller, source, n as u8);
            }
            // Guardian of Faith's ETB (CR 702.26): pause to choose any number of the *other*
            // creatures its controller controls to phase out. Nothing to choose with no other
            // creature — skip past (like `begin_proliferate`'s empty board).
            Effect::PhaseOut => self.begin_phase_out(controller, source),
            // Kinetic Ooze's X≥10 rider (CR 601.2c/603.3d): double the +1/+1 counters on each of the
            // "other target creatures" chosen at placement (read from this ability's second target
            // clause). A target that has left the battlefield since is skipped (CR 608.2b).
            Effect::DoubleCountersOnTargetCreatures { .. } => {
                let source_name = self.source_name_of(source);
                for chosen in targets_second.iter() {
                    let Some(object) = chosen.object_id() else {
                        continue;
                    };
                    if self.as_permanent(object).is_none() {
                        continue;
                    }
                    if let Some(event) = self.doubled_counters_event(object, source_name) {
                        self.push_apply(events, event);
                    }
                }
            }
            // Move all counters of a kind (Nexus Mentality / Forgotten Ancient): `target` is
            // already resolved (the moved-from permanent); pause on a ChooseTarget for the
            // second permanent, mirroring `Fight`'s cast/resolution split.
            Effect::MoveCounters {
                to_filter,
                all_kinds,
                distributed,
                ..
            } => {
                let from = expect_object_target(target, "a move-counters effect's source");
                let legal: Vec<ObjectId> = self
                    .legal_targets_for(
                        TargetSpec::Permanent(to_filter),
                        source,
                        controller,
                        [false; Color::COUNT],
                        x,
                    )
                    .into_iter()
                    .filter_map(|t| (t != Target::Object(from)).then_some(t.object_id()?))
                    .collect();
                if legal.is_empty() {
                    return;
                }
                // Forgotten Ancient's "distributed as you choose among any number of target
                // creatures" (CR 601.2d): pause on a target→amount map capped at `from`'s live
                // +1/+1 count, rather than choosing one destination for the whole pile.
                if distributed {
                    let cap = self.permanent(from).plus_counters;
                    if cap <= 0 {
                        return; // nothing to move — "any number" tops out at zero.
                    }
                    crate::pending::raise_choice(
                        self,
                        PendingChoice::DivideMovedCounters {
                            player: controller,
                            from,
                            legal,
                            cap,
                        },
                    );
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::MoveCounters {
                            target: TargetSpec::None,
                            to_filter,
                            all_kinds,
                            distributed,
                            from: Some(Target::Object(from)),
                        },
                        legal: legal.into_iter().map(Target::Object).collect(),
                        optional: false,
                    },
                );
            }
            // Perpetual Timepiece ("Shuffle any number of target cards from your graveyard into
            // your library", `target_player = false`) and Quandrix Command mode 3 ("Target
            // player shuffles up to three target cards from their graveyard into their
            // library", `target_player = true`) both pause on a ShuffleFromGraveyard choice —
            // the graveyard owner is the ability's controller or the targeted player.
            Effect::ShuffleTargetCardsFromGraveyardIntoLibrary { max, target_player } => {
                let owner = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!("target-player shuffle resolves with a chosen player target");
                    };
                    player
                } else {
                    controller
                };
                self.begin_shuffle_from_graveyard(controller, owner, source, max)
            }
            // Chaos Warp: the target's owner shuffles it into their library, then reveals the
            // new top card and — if it's a permanent card — puts it onto the battlefield under
            // the owner (not necessarily this effect's controller). Deterministic (the shuffle's
            // PRNG is the only randomness) but needs the *actual* post-shuffle order, so this
            // runs here rather than through `execute_effect`'s pure event-building path.
            Effect::ShuffleTargetPermanentIntoLibraryThenReveal { .. } => {
                let object = expect_object_target(target, "a permanent to tuck");
                let owner = self.owner_of(object);
                // CR 111.7: a token can't exist in a library — it ceases to exist instead.
                if self.permanent(object).token {
                    self.push_apply(
                        events,
                        Event::TokenCeasedToExist {
                            token: object,
                            controller: owner,
                            def: self.def_of(object),
                        },
                    );
                    return;
                }
                self.push_apply(
                    events,
                    Event::TuckedToLibrary {
                        card: self.next_object_id(),
                        from: object,
                        to_top: false,
                    },
                );
                self.push_apply(events, Event::LibraryShuffled { player: owner });
                // CR 120.3: an empty library reveals nothing — a clean no-op.
                let Some(&card) = self.players[owner.0 as usize].library.first() else {
                    return;
                };
                let def = self.def_of(card);
                self.push_apply(
                    events,
                    Event::RevealedTopOfLibrary {
                        player: owner,
                        card,
                        def,
                    },
                );
                if CardFilter::Permanent.matches(def) {
                    self.push_apply(
                        events,
                        Event::SearchedToBattlefield {
                            permanent: self.next_object_id(),
                            from: card,
                            controller: owner,
                            tapped: false,
                        },
                    );
                }
            }
            // "Counter target spell unless its controller pays {N}" (CR 701.5c-style): pause on
            // a PayOrCounter choice for the *target spell's* controller instead of countering
            // outright. `unless_pays: None` falls through to the catch-all's unconditional counter.
            Effect::CounterTargetSpell {
                unless_pays: Some(amount),
                ..
            } => {
                let original = expect_object_target(target, "a spell to counter");
                // If the target already left the stack (countered/resolved in response), there's
                // nothing to hold hostage — same no-op as the unconditional counter (CR 608.2b).
                if !matches!(self.objects[original as usize], Object::Spell(_)) {
                    return;
                }
                let generic = self.resolve_count(amount, controller, source, target, x);
                pending::raise(
                    self,
                    pending::ChoiceRequest::PayOrCounter {
                        player: self.controller_of(original),
                        cost: Cost {
                            generic: generic as u8,
                            ..Cost::FREE
                        },
                        spell: original,
                    },
                );
            }
            // Rhystic Study's "you may draw a card unless that player pays {1}": pause the
            // ability's own controller on whether they want to draw at all (the card's ruling —
            // declining is quiet, no pay window is ever offered). Only a "yes" here raises the
            // triggering opponent's own pay-or-let-it-happen pause (`Game::answer_may`).
            Effect::MayDrawUnlessPays { cost, caster } => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::MayDrawUnlessPays { cost, caster },
                    },
                );
            }
            // Questing Phelddagrif's blue rider: "Target opponent may draw a card." Unlike
            // `MayDrawUnlessPays` above, the *targeted* player answers (no pay window rides
            // behind it) — see `Game::answer_may`.
            Effect::TargetPlayerMayDraw { count, opponent } => {
                let Some(Target::Player(player)) = target else {
                    panic!("target-player-may-draw resolves with a chosen player target");
                };
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player,
                        source,
                        effect: Effect::TargetPlayerMayDraw { count, opponent },
                    },
                );
            }
            // Hinder's destination rider (CR 701.5b — `countered_dest`): pause this ability's
            // controller on a top/bottom pick before the countered card moves, unless it's not
            // going to a graveyard anyway — already left the stack / uncounterable (CR 608.2b /
            // 701.5g), or exiles instead (flashback/escape, CR 702.34e/702.19d; Quintorius's CR
            // 614.6 bottom-library redirect) — those cases fall through to the ordinary
            // `counter_spell`, which has nothing left for this rider to redirect.
            Effect::CounterTargetSpell {
                unless_pays: None,
                countered_dest: Some(CounteredDest::LibraryTopOrBottom),
                ..
            } => {
                let original = expect_object_target(target, "a spell to counter");
                let is_spell = matches!(self.objects[original as usize], Object::Spell(_));
                let goes_to_graveyard = is_spell
                    && !self.def_of(original).uncounterable
                    && !self.spell(original).flashback
                    && !self.spell(original).escape
                    && !self
                        .play_permissions
                        .stack_object_bottoms_library_on_leave
                        .iter()
                        .any(|&flagged| self.current_id(flagged) == original);
                if !goes_to_graveyard {
                    let evs = self.counter_spell(original);
                    self.apply_all(&evs);
                    events.extend(evs);
                    return;
                }
                pending::raise_choice(
                    self,
                    PendingChoice::ChooseCounteredSpellDestination {
                        player: controller,
                        spell: original,
                    },
                );
            }
            // Patchwork Banner's "As this artifact enters, choose a creature type": pause on a
            // ChooseCreatureType for the controller, over the pool's known creature types.
            Effect::ChooseCreatureType => pending::raise(
                self,
                pending::ChoiceRequest::ChooseCreatureType {
                    player: controller,
                    source,
                    options: CREATURE_TYPES,
                },
            ),
            // Flickering Ward's "As this Aura enters, choose a color": pause on a ChooseColor for (CR 702.21, CR 303.4)
            // the controller over the fixed five colors.
            Effect::ChooseColor => pending::raise(
                self,
                pending::ChoiceRequest::ChooseColor {
                    player: controller,
                    source,
                },
            ),
            // "Choose one —" on a triggered ability (CR 700.2): pause on a ChooseMode for the
            // controller. The chosen mode resolves later through this same pipeline (see
            // `answer_choose_mode`), carrying this ability's `source`/`target`/`x` context so a
            // mode that needs them still has them. An empty mode list is a defensive no-op.
            Effect::ChooseOne { modes } => {
                if modes.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseMode {
                        player: controller,
                        source,
                        target,
                        x,
                        modes,
                    },
                );
            }
            // Fight (CR 701.12): `target` is already the opponent's creature (chosen at cast);
            // pause on a ChooseTarget for the controller's own creature (mirrors
            // `place_targeted_ability`). No legal creature you control: the fight fizzles
            // (CR 601.2c — no damage, no pause) rather than picking an illegal target.
            Effect::Fight {
                ally_is_shared_target: false,
                ..
            } => {
                let legal = self.legal_targets_for(
                    TargetSpec::CreatureYouControl,
                    source,
                    controller,
                    [false; Color::COUNT],
                    x,
                );
                if legal.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::Fight {
                            enemy: target,
                            ally_is_shared_target: false,
                        },
                        legal,
                        optional: false,
                    },
                );
            }
            // Primal Might's mirror shape (CR 701.12): `target` is already the ally (the pumped
            // creature you control, chosen at cast by a preceding Sequence step); pause on an
            // *optional* ChooseTarget for the enemy ("fights up to one target creature you don't
            // control"). Guard-returns with no pause if the ally has since left the battlefield
            // or stopped being a creature (CR 608.2b — a fizzled shared target) or there's no
            // legal enemy — the pump still stands either way.
            Effect::Fight {
                ally_is_shared_target: true,
                ..
            } => {
                let ally = expect_object_target(target, "primal might's pumped ally");
                if !self.is_creature_on_battlefield(ally) {
                    return;
                }
                let legal = self.legal_targets_for(
                    TargetSpec::Permanent(PermanentFilter {
                        controller: FilterController::Opponent,
                        ..PermanentFilter::of(TypeSet::CREATURE)
                    }),
                    source,
                    controller,
                    [false; Color::COUNT],
                    x,
                );
                if legal.is_empty() {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player: controller,
                        source,
                        effect: Effect::Fight {
                            enemy: Some(Target::Object(ally)),
                            ally_is_shared_target: false,
                        },
                        legal,
                        optional: true,
                    },
                );
            }
            // Twincast: put a copy of the target spell on the stack under this controller, then
            // offer CR 707.10c's "you may choose new targets for the copy" — same
            // `choose_spell_targets` machinery a multi-target spell uses at cast (auto-fills a
            // single legal target, else pauses on `ChooseSpellTargets`), just run here because
            // the copy doesn't exist until this event applies.
            Effect::CopyTargetSpell => {
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
                // (the copy shares the same `def`). No pool copy source copies a modal or
                // multi-target spell, so the first `Timing::Spell` ability is authoritative.
                let Some(ability) = original_def
                    .abilities
                    .iter()
                    .find(|a| matches!(a.timing, Timing::Spell))
                else {
                    return;
                };
                let spec = ability.effect.target();
                if spec == TargetSpec::None {
                    return;
                }
                self.choose_spell_targets(
                    copy,
                    spec,
                    ability.effect.target_count(),
                    controller,
                    events,
                );
            }
            // Storm/Gravestorm-style rider: mint `count` copies of *this* resolving spell
            // (`source`, not a chosen target). CR 706.9's "when you cast this spell" trigger
            // never re-fires for an uncast copy — guard that here rather than modeling a real
            // cast trigger, since this rider runs as one of the spell's own resolution effects.
            Effect::CopyThisSpell {
                count,
                cast_from_graveyard_only,
                optional,
            } => {
                if self.spell(source).copy {
                    return;
                }
                // "If this spell was cast from a graveyard" (Sevinne's Reclamation's flashback (CR 702.34, CR 403.5, CR 601)
                // rider) — the mint never happens for an ordinary cast.
                if cast_from_graveyard_only && !self.spell(source).flashback {
                    return;
                }
                // "You may copy this spell": pause for a yes/no before minting (mirrors
                // `MaySacrifice`/`MayReturnFromGraveyard`'s resolution-time "declining runs
                // nothing" shape); the mandatory storm/Gravestorm case (`optional = false`) skips
                // straight to the mint below. `answer_may` mints inline on acceptance rather than
                // placing a new triggered ability — this rider is part of `source`'s own (CR 603, CR 113)
                // resolution, not a separate stack object.
                if optional {
                    pending::raise(
                        self,
                        pending::ChoiceRequest::MayYesNo {
                            player: controller,
                            source,
                            effect: Effect::CopyThisSpell {
                                count,
                                cast_from_graveyard_only: false,
                                optional: false,
                            },
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
            Effect::RetargetSpellCopy { copy } => {
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
                self.choose_spell_targets(copy, spec, count, controller, events);
            }
            // Willbender (CR 114.6 / 702.37f): "change the target of target spell … with a single
            // target." The bent spell is this trigger's own chosen target (CR 603.3d), already
            // re-checked legal by CR 608.2b before this ran (so a spell that left the stack fizzles
            // the trigger). Guard the shape defensively.
            Effect::ChangeTargetOfTargetSpellOrAbility { .. } => {
                let Some(Target::Object(spell)) = target else {
                    return;
                };
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                // The bent spell's own single target clause, and its currently-legal targets
                // computed for the SPELL's controller (CR 114.6 — the new target must be legal for
                // *that* spell), minus its current target (CR 114.6b — the target must change).
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
            Effect::CopyTriggeringSpell {
                triggering_spell,
                count,
                may_choose_new_targets,
            } => {
                let Some(original) = triggering_spell else {
                    return;
                };
                // CR 603.4: the triggering spell may have left the stack (countered in response)
                // before this delayed trigger resolved — nothing left to copy.
                if !matches!(self.objects[original as usize], Object::Spell(_)) {
                    return;
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
            Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { triggering_spell } => {
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
            Effect::CopyTriggeringAbility {
                triggering_ability,
                // ponytail: CR 707.10c's "you may choose new targets" re-pick isn't offered — the
                // copy keeps the original's targets. No pool card is a targeted `{X}` activated
                // ability, so this is never observable; see the effect's own doc for the upgrade.
                may_choose_new_targets: _,
            } => {
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
            // Demonstrate (CR 702.147a): "you may copy it" — a real, respondable trigger above
            // the cast spell (`spell` baked in at placement, see `CardDef::demonstrate`). The
            // spell may have been countered in response before this trigger resolved (CR 707.10c
            // guard, same shape as `CopyTriggeringSpell`'s above): nothing left to copy.
            Effect::Demonstrate { spell } => {
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::MayYesNo {
                        player: controller,
                        source,
                        effect: Effect::Demonstrate { spell },
                    },
                );
            }
            // Opal Palace's spend-to-cast rider: the commander spell (baked in as
            // `triggering_spell` when the `SpendManaToCast` trigger fired) is still on the stack, so
            // record the additional-counter count keyed by its id for `resolve_spell` to place as it
            // enters. Guard-return if that spell already left the stack (countered in response, CR
            // 603.4) — nothing to enter, so nothing to record.
            Effect::CommanderEntersWithBonusCounters {
                triggering_spell,
                count,
            } => {
                let Some(spell) = triggering_spell else {
                    return;
                };
                if !matches!(self.objects[spell as usize], Object::Spell(_)) {
                    return;
                }
                let n = self.resolve_count(count, controller, source, target, x);
                if n == 0 {
                    return;
                }
                self.pending_enter_bonus_counters.push((spell, n));
            }
            // Renegade Bull's attack trigger: "exile up to one target instant or sorcery card
            // from your graveyard and copy it. You may cast the copy without paying its mana
            // cost." "Up to one": no chosen target (declined, or none legal — CR 603.3c already
            // drops the ability before this runs) is a no-op. Exile the chosen card, then grant
            // the free-cast permission (CR 118.5) for it — the same `CastFromExileFreePermissionGranted`
            // plumbing `CastExiledWithThisFree` (Quintorius) grants — so the controller can
            // genuinely *cast* it (CR 601) at their next opportunity, firing real "whenever you
            // cast" watchers off it (including this card's own first ability above).
            Effect::ExileTargetGraveyardSpellCastFree { .. } => {
                let Some(object) = target.and_then(Target::object_id) else {
                    return;
                };
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                self.push_apply(
                    events,
                    Event::CastFromExileFreePermissionGranted {
                        card: exiled,
                        player: controller,
                    },
                );
            }
            // Surge to Victory: "Exile target instant or sorcery card from your graveyard."
            // Mandatory single target (unlike Renegade Bull's "up to one" above), so a legal
            // target is guaranteed by the time this runs (CR 608.2b already fizzled the whole
            // ability otherwise). Snapshot the exiled card's id + mana value for the following
            // team-pump (`Amount::ExiledCardManaValueThisWay`) and combat-damage-copy arm
            // (`ScheduleThisTurnCombatDamageCopy`) steps sharing this resolution's `Sequence`.
            Effect::ExileTargetGraveyardCardRecordManaValue { .. } => {
                let object =
                    expect_object_target(target, "exile target graveyard card, record mana value");
                let mana_value = self.def_of(object).mana_value();
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                self.surge_exiled_card = Some((exiled, mana_value));
            }
            // Surge to Victory's combat-damage-copy step: mint one free copy of the card the
            // arming watch names — the exile already happened when the watch was armed, so this
            // only mints. `card` is `None` only if this were ever misfired with no armed card,
            // which `fire_combat_damage_copy_triggers` never does.
            Effect::MintFreeCopyOfExiledCard { card } => {
                let Some(card) = card else {
                    return;
                };
                self.mint_spell_copies(Amount::Fixed(1), controller, card, None, 0, events);
            }
            // A discard pauses on a card-pick choice (the discarding player chooses which to
            // pitch): the ability's controller, or a chosen target player (Prismari Command).
            Effect::Discard {
                count,
                target_player,
                or_one_matching,
            } => {
                let discarder = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!("target-player discard resolves with a chosen player target");
                    };
                    player
                } else {
                    controller
                };
                self.begin_discard(discarder, count, or_one_matching)
            }
            // "You may put a land from hand onto the battlefield" pauses on a card-pick choice
            // (up to one hand land, or decline).
            Effect::PutLandFromHand { tapped } => self.begin_put_land_from_hand(controller, tapped),
            // Illusionary Mask's "you may cast a creature card in hand … face down as a 2/2"
            // pauses on a card-pick choice over the hand creatures whose mana cost the mana
            // spent on this ability's `{X}` could pay (`ctx.spent_mana`, CR 107.3).
            Effect::CastCreatureFaceDown => {
                self.begin_cast_creature_face_down(controller, spent_mana)
            }
            // Rupture Spire's own ETB trigger: "sacrifice it unless you pay {1}." Pauses on the
            // same pay-or-sacrifice shape Echo's `PayEchoOrSacrifice` uses, under its own variant
            // (this is a real triggered ability, not Echo — CR 603.3b, not CR 702.31).
            Effect::SacrificeSelfUnlessPay { cost } => {
                self.begin_sacrifice_unless_pay(controller, source, cost)
            }
            // Treva's Ruins' own ETB trigger: "sacrifice it unless you return a non-Lair land you
            // control." Pauses on a candidate-land pick (or sacrifices outright with none).
            Effect::SacrificeSelfUnlessReturnLand { filter } => {
                self.begin_sacrifice_unless_return_land(controller, source, filter, events)
            }
            // "Put a card exiled with this" pauses on a card-pick choice over this source's
            // exiled-with pile (up to one, or decline).
            Effect::CashOutExiledWithThis => {
                self.begin_cash_out_exiled_with_this(controller, source)
            }
            // Quintorius's activated ability pauses on a card-pick choice over this source's (CR 602, CR 113)
            // linked exile pile, granting the free-cast permission for the chosen card instead
            // of cashing it out.
            Effect::CastExiledWithThisFree => {
                self.begin_cast_exiled_with_this_free(controller, source)
            }
            // A sequence runs its steps in order, sharing this target/{X}; a pausing step defers
            // the rest until answered.
            Effect::Sequence { steps } => self.run_sequence(steps, ctx, events),
            // A per-step gate: run `then` only if `condition` holds (negated by `negate`) right
            // now (mid-resolution), sharing this target/{X}. Reuses the same intervening-if
            // evaluator triggers use, except `TargetPowerAtLeast` (Yavimaya Bloomsage's power-7
            // check), `SourceEnteredWithXAtLeast` (Kinetic Ooze's X-threshold riders), and
            // `ColorWasSpentToCastThis` (Court Hussar's "unless {W} was spent to cast it"):
            // `TriggerContext` carries neither a target nor a source id, so those are
            // special-cased directly against the shared `target`/this resolution's own `source`
            // here — the same "condition_holds can't reach it" shape as
            // `ability_condition_holds`'s source-based special cases.
            Effect::Conditional {
                condition,
                then,
                negate,
            } => {
                let holds = match condition {
                    Condition::TargetPowerAtLeast { at_least } => target
                        .and_then(Target::object_id)
                        .is_some_and(|object| self.power(object) >= at_least as i32),
                    Condition::SourceEnteredWithXAtLeast { at_least } => {
                        self.ability_source_x(source) >= at_least
                    }
                    Condition::ColorWasSpentToCastThis { color } => self
                        .as_permanent(source)
                        .is_some_and(|p| p.spent_colors[color.index()]),
                    _ => self.condition_holds(condition, TriggerContext::of(controller)),
                };
                if holds != negate {
                    self.run_sequence(then, ctx, events);
                }
            }
            // Feral Appetite: exile the targeted graveyard card, then — CR "if a creature card
            // is exiled this way" — run `then` (mints the Pest token) only if the just-exiled
            // card's own printed type is a creature. Reads the def before the move, the same
            // shape `ExileTargetFromGraveyardCreateTokenCopy` reads `def_of` before it exiles.
            Effect::ExileTargetGraveyardCardThenIfCreature { then } => {
                let object =
                    expect_object_target(target, "exile target graveyard card, then if creature");
                let is_creature = matches!(self.def_of(object).kind, CardKind::Creature { .. });
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                if is_creature {
                    self.run_sequence(then, ctx, events);
                }
            }
            // Marauding Raptor: deal the damage (unchanged via `execute_effect`), then — CR "if a
            // Dinosaur is dealt damage this way, this creature gets +2/+0 until end of turn" —
            // run `then` only if the entering permanent's subtypes intersect `then_if_subtype`
            // AND the damage actually landed (a `DamageMarked` event was produced — none means a
            // protection/prevention shield stopped it, CR 119.3 "is dealt damage").
            Effect::DealDamageToEnteringPermanent {
                entering,
                then_if_subtype,
                then,
                ..
            } => {
                let evs = self.execute_effect(effect, controller, source, target, x);
                let damage_landed = evs.iter().any(|e| matches!(e, Event::DamageMarked { .. }));
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
            // Untap the permanent this same resolution's own search step already put onto the
            // battlefield (Fabled Passage's "then … untap that land") — reads it back from the
            // SearchedToBattlefield event already recorded in `events` (see the variant doc).
            // No such event yet (the search failed to find, or hasn't run): nothing to untap.
            Effect::UntapSearchedLand => {
                let found = events.iter().rev().find_map(|e| match e {
                    Event::SearchedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                });
                if let Some(permanent) = found {
                    self.push_apply(events, Event::Untapped { object: permanent });
                }
            }
            // Ajani's Chosen: attach the triggering Aura to the token this same resolution's
            // preceding `CreateToken` step already minted — read back from `events`. A non-Aura
            // entering (`entering` is `None`, or its kind isn't Aura) or a missing token is a
            // no-op (guard-return).
            Effect::AttachTriggeringAuraToMintedToken { entering } => {
                let Some(entering) = entering else {
                    return;
                };
                if !matches!(self.def_of(entering).kind, CardKind::Aura) {
                    return;
                }
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: entering,
                        host: Some(token),
                    },
                );
            }
            // A reflexive "when you do" trigger (CR 603.3b — Forum Filibuster): the "you do" is
            // that this resolution's preceding `CreateToken` step minted a token (read back from
            // `events`, the same idiom as `AttachTriggeringAuraToMintedToken` above). No such
            // token: no reflexive trigger (guard-return). Otherwise enqueue each `then` effect as
            // its own reflexive triggered ability — a separate, respondable stack object placed
            // the next time a player would get priority — threading the minted token in.
            Effect::ReflexiveTrigger { then } => {
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.queue_reflexive_trigger(controller, source, then, token);
            }
            // The reflexive ability's own resolution: return the chosen graveyard card (CR 601.2c
            // target, may be `None` — "up to one") to the battlefield attached to the minted
            // `token`. Guard-return (CR 608.2b) if the token has left the battlefield since — with
            // the host gone the returned card can't be attached, so nothing happens.
            Effect::ReturnFromGraveyardAttachedToToken { token, .. } => {
                let Some(token) = token.filter(|&t| self.as_permanent(t).is_some()) else {
                    return;
                };
                let Some(card) = target.and_then(Target::object_id) else {
                    return;
                };
                let event = self.reanimate_event(card, controller, false);
                let Event::ReanimatedToBattlefield { permanent, .. } = event else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                self.push_apply(events, event);
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: permanent,
                        host: Some(token),
                    },
                );
            }
            // Animate Dead: attach this Aura to the creature this same resolution's preceding
            // `ReanimateToBattlefield` step already put onto the battlefield — read back from
            // `events`. No such event yet: nothing to attach to (guard-return).
            Effect::AttachSelfToReanimated => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: source,
                        host: Some(permanent),
                    },
                );
            }
            // Fractal Harness: attach this Equipment to the token this same resolution's
            // preceding `CreateToken` step already minted — read back from `events`, the same
            // idiom as `AttachSelfToReanimated` above. No such token yet: nothing to attach to
            // (guard-return).
            Effect::AttachSelfToMintedToken => {
                let Some(token) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: source,
                        host: Some(token),
                    },
                );
            }
            // Scriv, the Obligator: attach the Aura token this same resolution's preceding
            // `CreateToken` step just minted to the ability's chosen target (a creature an opponent
            // controls) — the mirror of `AttachSelfToMintedToken` above, attaching the *minted
            // token* rather than the source. No token minted yet, a non-Aura token, or a
            // non-object target: nothing to attach (guard-return).
            // ponytail: only an Aura can be attached (CR 303); a non-Aura minted token is a no-op
            // rather than a phantom attachment. The pool mints only the Contract Aura here.
            Effect::AttachMintedAuraToTarget { .. } => {
                let Some(host) = target.and_then(Target::object_id) else {
                    return;
                };
                let Some(aura) = events.iter().rev().find_map(|e| match e {
                    Event::TokenCreated { token, .. } => Some(*token),
                    _ => None,
                }) else {
                    return;
                };
                if !matches!(self.def_of(aura).kind, CardKind::Aura) {
                    return;
                }
                self.push_apply(
                    events,
                    Event::AttachedTo {
                        object: aura,
                        host: Some(host),
                    },
                );
            }
            // Fractal Harness's attack trigger: double the +1/+1 counters on the creature this
            // Equipment is attached to (CR 614) — a no-target sibling of `DoubleCounters` pinned
            // to `source`'s own `attached_to` instead of a chosen target. An unattached Equipment
            // (unequipped, or between equip targets) has nothing to double (guard-return).
            Effect::DoubleCountersOnAttachedCreature => {
                let Some(object) = self.permanent(source).attached_to else {
                    return;
                };
                if let Some(event) = self.doubled_counters_event(object, self.def_of(source).name) {
                    self.push_apply(events, event);
                }
            }
            // Gift of Immortality: schedule the delayed return of this Aura (CR 603.7), attached
            // to the creature this same resolution's preceding `ReanimateDyingEnchantedCreature`
            // step just reanimated — read back from `events`, mirroring `AttachSelfToReanimated`'s
            // idiom above. No such event yet (the enchanted creature wasn't reanimated): nothing
            // to schedule (guard-return).
            Effect::ScheduleReturnThisAuraAttachedToReanimated => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnThisAuraAttachedTo {
                            creature: Some(permanent),
                        },
                    },
                );
            }
            // Screams from Within: the immediate dies-return, choosing a new host (unlike Gift
            // of Immortality's same-creature return above). Pauses via the shared helper — see
            // its doc comment.
            Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.begin_return_aura_from_graveyard_attached_to_chosen_host(source, events)
            }
            // Ghoulish Impetus: schedule the same choose-host return above at the next end step
            // (CR 603.7), mirroring `ScheduleReturnThisAuraAttachedToReanimated`'s emit shape. No
            // read-back needed — this Aura's own `source` is all the delayed payload needs.
            Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost,
                    },
                );
            }
            // Mass destruction (unchanged batch via `execute_effect`), then snapshot which
            // permanents it destroyed onto `Game::destroyed_this_way` — overwriting any prior
            // call's snapshot, so it's scoped to this one `DestroyAll` step, not the whole turn
            // — for a following `Sequence` step to count (Ceaseless Conflict's "for each
            // nontoken creature you controlled that was destroyed this way" token rider, Culling
            // Ritual's "for each permanent destroyed this way" mana rider; see
            // `Amount::PermanentsDestroyedThisWay`). Built from `execute_effect`'s own destroy
            // events (already filtered/indestructible-checked) rather than re-deriving the (CR 702.12)
            // filter — reads each event's pre-move battlefield state before `apply_all`
            // tombstones it. `TokenCeasedToExist` already carries its own controller/def
            // (a token vanishes rather than moving, so there's no `from` to read back).
            Effect::DestroyAll { .. } => {
                let evs = self.execute_effect(effect, controller, source, target, x);
                self.destroyed_this_way.clear();
                for e in &evs {
                    match *e {
                        Event::TokenCeasedToExist {
                            controller: died_controller,
                            def,
                            ..
                        } => {
                            self.destroyed_this_way.push(state::DestroyedThisWay {
                                def,
                                controller: died_controller,
                                token: true,
                            });
                        }
                        Event::MovedToGraveyard { from, .. }
                        | Event::MovedToCommandZone { from, .. } => {
                            if let Some(p) = self.as_permanent(from) {
                                self.destroyed_this_way.push(state::DestroyedThisWay {
                                    def: p.def,
                                    controller: self.controller_of(from),
                                    token: false,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                self.apply_all(&evs);
                events.extend(evs);
            }
            // Mass exile (unchanged batch via `execute_effect`), then snapshot each exiled
            // creature's controller and power onto `Game::power_exiled_this_way` — overwriting
            // any prior call's snapshot, so it's scoped to this one `ExileAll` step — for a
            // following `EachPlayerCreatesFractalFromExiledPower` step to sum per player
            // (Oversimplify's "total power of creatures they controlled that were exiled this
            // way"). Power is read off each event's pre-move object id before `apply_all` moves
            // it, same ordering `DestroyAll`'s snapshot above uses.
            Effect::ExileAll { .. } => {
                let evs = self.execute_effect(effect, controller, source, target, x);
                self.power_exiled_this_way.clear();
                for e in &evs {
                    match *e {
                        Event::TokenCeasedToExist {
                            token,
                            controller: died_controller,
                            ..
                        } => {
                            self.power_exiled_this_way.push(state::PowerExiledThisWay {
                                controller: died_controller,
                                power: self.power(token),
                            });
                        }
                        Event::MovedToExile { from, .. }
                        | Event::MovedToCommandZone { from, .. } => {
                            self.power_exiled_this_way.push(state::PowerExiledThisWay {
                                controller: self.controller_of(from),
                                power: self.power(from),
                            });
                        }
                        _ => {}
                    }
                }
                self.apply_all(&evs);
                events.extend(evs);
            }
            // Self-mill (Perpetual Timepiece, Fateful Tempest's past-vote step), snapshotting the
            // total mana value of the milled cards onto `Game::milled_mana_value_this_way` —
            // overwriting any prior call, so scoped to this one `MillSelf` step — for a following
            // `Sequence` step to read via `Amount::TotalManaValueMilledThisWay` (Fateful Tempest's
            // "damage … equal to the total mana value of cards milled this way"). Each card's mana
            // value is read off its pre-move library id before `apply_all` moves it to the
            // graveyard, the same ordering `DestroyAll`/`ExileAll`'s snapshots above use.
            Effect::MillSelf { count } => {
                let n = self.resolve_count(count, controller, source, target, x);
                let evs = self.mill_events(controller, n);
                self.milled_mana_value_this_way = evs
                    .iter()
                    .filter_map(|e| match e {
                        Event::Milled { from, .. } => Some(self.def_of(*from).mana_value()),
                        _ => None,
                    })
                    .sum();
                self.apply_all(&evs);
                events.extend(evs);
            }
            // "Exile [this card] with N time counters on it" (Rousing Refrain): mark the resolving
            // spell so `finish_instant_sorcery_resolution` sends it to exile with time counters
            // instead of the graveyard (the resolving spell, `source`, is the card exiled).
            Effect::ExileSelfWithTimeCounters { counters } => {
                self.self_exile_time_counters = Some(counters);
            }
            // "Each player creates a 0/0 green and blue Fractal creature token and puts a number
            // of +1/+1 counters on it equal to the total power of creatures they controlled that
            // were exiled this way." (Oversimplify): mint one `token` per living player in APNAP
            // order, applying each mint before computing its counters — `counters_after_replacements`
            // reads the token's controller off game state, mirroring `CreateToken`'s `enters_with`
            // below. No player choice, so this resolves in one pass, never pausing.
            Effect::EachPlayerCreatesFractalFromExiledPower { token } => {
                for player in self.apnap_order() {
                    let minted = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::TokenCreated {
                            token: minted,
                            controller: player,
                            def: token,
                    creator: source,
                },
                    );
                    let power: i32 = self
                        .power_exiled_this_way
                        .iter()
                        .filter(|snap| snap.controller == player)
                        .map(|snap| snap.power)
                        .sum();
                    let n = self.counters_after_replacements(minted, power);
                    if n > 0 {
                        self.push_apply(
                            events,
                            Event::CountersPlaced {
                                object: minted,
                                count: n,
                                source_name: self.def_of(source).name,
                            },
                        );
                    }
                }
            }
            // "Each player discards their hand, then draws seven cards." (Wheel of Fortune):
            // loop APNAP order, each living player discarding their whole hand (`discard_ids` —
            // no choice, so no `PendingChoice`, unlike a partial-hand `Effect::Discard`) then
            // drawing `count`.
            Effect::EachPlayerDiscardsHandThenDraws { count } => {
                let n = self.resolve_count(count, controller, source, target, x);
                for player in self.apnap_order() {
                    let hand = self.hand_of(player);
                    self.discard_ids(&hand, player, events);
                    for event in self.draw_events(player, n) {
                        self.push_apply(events, event);
                    }
                }
            }
            // Mint the token(s) (unchanged batch via `execute_effect`), then — "Put X +1/+1
            // counters on it" (Deekah's Magecraft Fractal) — place `enters_with` counters on each
            // minted token, routed through the same doubler/Hardened-Scales replacement pipeline
            // as a spell's own `EntersWithCounters` (`Game::resolve_spell`'s enters-with path).
            // `counters_after_replacements` reads the token's controller off game state, so the
            // mint must already be applied — mirrors `resolve_spell` applying `PermanentEntered`
            // before reading its counters.
            Effect::CreateToken { enters_with, .. } => {
                let evs = self.execute_effect(effect, controller, source, target, x);
                self.apply_all(&evs);
                let minted: Vec<ObjectId> = evs
                    .iter()
                    .filter_map(|e| match e {
                        Event::TokenCreated { token, .. } => Some(*token),
                        _ => None,
                    })
                    .collect();
                events.extend(evs);
                let n_raw = self.resolve_amount(enters_with, controller, source, target, x);
                if n_raw > 0 {
                    for id in minted {
                        let n = self.counters_after_replacements(id, n_raw);
                        if n > 0 {
                            self.push_apply(
                                events,
                                Event::CountersPlaced {
                                    object: id,
                                    count: n,
                                    source_name: self.def_of(source).name,
                                },
                            );
                        }
                    }
                }
            }
            // Advanced Reconstruction's base ability: "exile a card from your graveyard at
            // random. You may play the exiled card this turn." The card is picked by the
            // injected RNG here (needs `&mut self`, unlike `ExileFromGraveyardMayPlay`'s
            // trigger-supplied card), then reuses that same event/permission plumbing.
            Effect::ExileRandomFromGraveyardMayPlay => {
                let graveyard = self.graveyard_cards(controller);
                // CR 701.19a: if there's nothing to exile, this is a no-op.
                if graveyard.is_empty() {
                    return;
                }
                let idx = (self.next_u64() % graveyard.len() as u64) as usize;
                let from = graveyard[idx];
                self.push_apply(
                    events,
                    Event::ExiledFromGraveyardMayPlay {
                        player: controller,
                        card: self.next_object_id(),
                        from,
                    },
                );
            }
            // Inkshield (CR 615): arm a this-turn combat-damage prevention shield protecting the
            // ability's controller ("dealt to *you*"), carrying the Inkling profile minted per
            // point prevented. The tokens are created at the prevention itself (in `damage_player`),
            // not here — at resolution no combat damage has been prevented yet. Runtime
            // orchestration state (like the delayed combat-damage watches), not an event.
            Effect::PreventCombatDamageToYouCreatingTokens { token } => self
                .combat_extras
                .combat_damage_prevention_shields
                .push((controller, token)),
            // Moment's Peace (#150): arm the this-turn table-wide combat-damage shield — every
            // player's combat damage, not just this ability's controller's, and no token mint.
            // Runtime orchestration state (like Inkshield's shield above), not an event.
            Effect::PreventAllCombatDamageThisTurn => {
                self.combat_extras.prevent_all_combat_damage_this_turn = true;
            }
            _ => {
                let evs = self.execute_effect(effect, controller, source, target, x);
                self.apply_all(&evs);
                events.extend(evs);
            }
        }
    }

    /// Mint `count` copies of `source` (`Effect::CopyThisSpell`'s mandatory mint, and
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
                Effect::RetargetSpellCopy { copy }
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

    /// Shared reanimation core: mint a `ReanimatedToBattlefield` event putting graveyard `card`
    /// onto the battlefield under `new_controller`'s control (enters via the usual ETB path —
    /// summoning-sick, ETB triggers fire). Both a chosen-target reanimation
    /// ([`Effect::ReanimateToBattlefield`]) and a look-back one
    /// ([`Effect::ReanimateDyingEnchantedCreature`]) mint through here.
    pub(crate) fn reanimate_event(
        &self,
        card: ObjectId,
        new_controller: PlayerId,
        finality: bool,
    ) -> Event {
        Event::ReanimatedToBattlefield {
            permanent: self.next_object_id(),
            from: card,
            controller: new_controller,
            finality,
            tapped: false,
        }
    }

    /// Private mint: the events one non-pausing effect would produce for `controller`
    /// against `target`. Pure — [`Game::run`] applies (and applies before minting more ids).
    /// Pausing / composite effects never reach this: [`Game::run`] intercepts them.
    fn execute_effect(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            Effect::DealDamage {
                amount, divided, ..
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
                match chosen {
                    // Damage to a creature is marked (an SBA later checks it against toughness), (CR 704, CR 120.3)
                    // unless protection from the source's color prevents it (CR 702.16d).
                    Target::Object(object) => {
                        if self.damage_prevented_by_protection(object, Some(source)) {
                            return Vec::new();
                        }
                        // Phantom Centaur's self-shield prevents this damage outright and
                        // removes one of its own +1/+1 counters instead (CR 615).
                        if self.phantom_shield_active(object) {
                            return self.phantom_shield_counter_removal(object).into_iter().collect();
                        }
                        // Damage to a planeswalker removes that many loyalty counters instead of
                        // being marked (CR 120.3c/306.9) — checked ahead of Tajic's creature-only
                        // prevention below, since a planeswalker is never "another creature".
                        if matches!(self.def_of(object).kind, CardKind::Planeswalker { .. }) {
                            return vec![Event::LoyaltyChanged {
                                object,
                                amount: -amount,
                            }];
                        }
                        // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                        if self.noncombat_damage_prevented_to_creature(object) {
                            return Vec::new();
                        }
                        vec![Event::DamageMarked {
                            object,
                            amount,
                            source: Some(source),
                        }]
                    }
                    // Damage to a player is life loss. ponytail: the commander-damage tally is
                    // combat-only (CR 903.10a), so a burn spell never adds to it.
                    Target::Player(player) => {
                        let mut events = vec![Event::LifeChanged {
                            player,
                            amount: -amount,
                            source: Some(source),
                        }];
                        // 0 damage is never dealt (CR 120.8) — no marker, no trigger.
                        if amount > 0 {
                            events.push(Event::DamageDealtToPlayer {
                                source,
                                player,
                                amount,
                            });
                        }
                        events
                    }
                }
            }
            Effect::DrawCards { count } => self.draw_events(
                controller,
                self.resolve_count(count, controller, source, target, x),
            ),
            Effect::TargetPlayerDraws { count, .. } => {
                let Some(Target::Player(player)) = target else {
                    panic!("target-player-draws resolves with a chosen player target");
                };
                self.draw_events(
                    player,
                    self.resolve_count(count, controller, source, target, x),
                )
            }
            // Goblin Guide's attack trigger: reveal the defender's top card; land, to hand.
            Effect::RevealTopToHand { filter, defender } => {
                let defender = defender.expect("filled from attack context when placed");
                let Some(&card) = self.players[defender.0 as usize].library.first() else {
                    return Vec::new(); // an empty library reveals nothing (CR 120.3-ish).
                };
                let def = self.def_of(card);
                let mut events = vec![Event::RevealedTopOfLibrary {
                    player: defender,
                    card,
                    def,
                }];
                if filter.matches(def) {
                    events.push(Event::SearchedToHand {
                        player: defender,
                        object: self.next_object_id(),
                        from: card,
                        card: def,
                    });
                }
                events
            }
            // Open the Way: reveal from the top until X lands are found (or the library runs
            // out, CR 120-style "as many as possible"); each land goes to `matched_dest`
            // (battlefield tapped), every other revealed card to `rest_dest` (bottom of
            // library). Deterministic given the library, so no player choice is involved.
            Effect::RevealUntil {
                filter,
                count,
                matched_dest,
                matched_tapped,
                rest_dest,
            } => {
                let goal = self.resolve_count(count, controller, source, target, x);
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                let mut matched = 0;
                for &card in &self.players[controller.0 as usize].library {
                    if matched >= goal {
                        break; // cards past the stop point stay on top, untouched.
                    }
                    let def = self.def_of(card);
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def,
                    });
                    if !filter.matches(def) {
                        match rest_dest {
                            RestDest::Bottom => {
                                events.push(Event::PutOnBottomOfLibrary {
                                    player: controller,
                                    card,
                                });
                            }
                            RestDest::Hand => {
                                events.push(Event::SearchedToHand {
                                    player: controller,
                                    object: next,
                                    from: card,
                                    card: def,
                                });
                                next += 1;
                            }
                        }
                        continue;
                    }
                    matched += 1;
                    match matched_dest {
                        SearchDest::Battlefield => {
                            events.push(Event::SearchedToBattlefield {
                                permanent: next,
                                from: card,
                                controller,
                                tapped: matched_tapped,
                            });
                        }
                        SearchDest::Hand => {
                            events.push(Event::SearchedToHand {
                                player: controller,
                                object: next,
                                from: card,
                                card: def,
                            });
                        }
                        // ponytail: no pool card sets `matched_dest = "library_top"` on
                        // `reveal_until`/`reveal_top_cards` — this routine already processes the
                        // library strictly top-down, so once every miss ahead of a match has been
                        // routed away by `rest_dest`, the match sits on top with nothing further
                        // to do. Give this a real move event if a card ever needs it.
                        SearchDest::LibraryTop => {}
                    }
                    next += 1;
                }
                events
            }
            // Animist's Awakening: reveal exactly the top `count` cards (not "until N match" —
            // `RevealUntil`'s sibling), stopping early on a short library (CR 120.3 "as many as
            // possible"). Every match goes to `matched_dest`, deployed untapped instead of
            // `matched_tapped` when `deploy_untapped_if` holds (spell mastery); every other
            // revealed card goes to `rest_dest`.
            Effect::RevealTopCards {
                count,
                filter,
                matched_dest,
                matched_tapped,
                rest_dest,
                deploy_untapped_if,
            } => {
                let goal = self.resolve_count(count, controller, source, target, x);
                let tapped = matched_tapped
                    && !deploy_untapped_if.is_some_and(|condition| {
                        self.condition_holds(condition, TriggerContext::of(controller))
                    });
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for &card in self.players[controller.0 as usize]
                    .library
                    .iter()
                    .take(goal as usize)
                {
                    let def = self.def_of(card);
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def,
                    });
                    if !filter.matches(def) {
                        match rest_dest {
                            RestDest::Bottom => {
                                events.push(Event::PutOnBottomOfLibrary {
                                    player: controller,
                                    card,
                                });
                            }
                            RestDest::Hand => {
                                events.push(Event::SearchedToHand {
                                    player: controller,
                                    object: next,
                                    from: card,
                                    card: def,
                                });
                                next += 1;
                            }
                        }
                        continue;
                    }
                    match matched_dest {
                        SearchDest::Battlefield => {
                            events.push(Event::SearchedToBattlefield {
                                permanent: next,
                                from: card,
                                controller,
                                tapped,
                            });
                        }
                        SearchDest::Hand => {
                            events.push(Event::SearchedToHand {
                                player: controller,
                                object: next,
                                from: card,
                                card: def,
                            });
                        }
                        // ponytail: no pool card sets `matched_dest = "library_top"` on
                        // `reveal_until`/`reveal_top_cards` — see the sibling arm in
                        // `RevealUntil`'s resolution above for why this is a genuine no-op today.
                        SearchDest::LibraryTop => {}
                    }
                    next += 1;
                }
                events
            }
            // Keen Duelist's upkeep trigger: both players reveal their top card, each loses life
            // to the *other's* mana value, then each puts their own revealed card into hand.
            Effect::RevealTopAndDrainMutual => {
                let Some(Target::Player(opponent)) = target else {
                    panic!("reveal-top-and-drain-mutual resolves with a chosen opponent target");
                };
                let you = self.players[controller.0 as usize].library.first().copied();
                let them = self.players[opponent.0 as usize].library.first().copied();
                let mut events = Vec::new();
                if let Some(card) = you {
                    events.push(Event::RevealedTopOfLibrary {
                        player: controller,
                        card,
                        def: self.def_of(card),
                    });
                }
                if let Some(card) = them {
                    events.push(Event::RevealedTopOfLibrary {
                        player: opponent,
                        card,
                        def: self.def_of(card),
                    });
                }
                if let Some(card) = them {
                    events.push(Event::LifeChanged {
                        player: controller,
                        amount: -(self.def_of(card).mana_value() as i32),
                        source: Some(source),
                    });
                }
                if let Some(card) = you {
                    events.push(Event::LifeChanged {
                        player: opponent,
                        amount: -(self.def_of(card).mana_value() as i32),
                        source: Some(source),
                    });
                }
                let mut next = self.next_object_id();
                if let Some(card) = you {
                    events.push(Event::SearchedToHand {
                        player: controller,
                        object: next,
                        from: card,
                        card: self.def_of(card),
                    });
                    next += 1;
                }
                if let Some(card) = them {
                    events.push(Event::SearchedToHand {
                        player: opponent,
                        object: next,
                        from: card,
                        card: self.def_of(card),
                    });
                }
                events
            }
            Effect::GainLife { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                vec![Event::LifeChanged {
                    player: controller,
                    amount: self.life_gain_after_replacements(controller, amount),
                    source: Some(source),
                }]
            }
            Effect::LoseLife { amount } => vec![Event::LifeChanged {
                player: controller,
                amount: -self.resolve_amount(amount, controller, source, target, x),
                source: Some(source),
            }],
            // Swords to Plowshares' rider: the *target's* controller (its owner, per the
            // engine's control/ownership conflation) gains life, not this ability's controller.
            Effect::GainLifeTargetController { amount } => {
                let object = expect_object_target(target, "a controller-gains-life amount");
                let gainer = self.owner_of(object);
                let amount = self.resolve_amount(amount, controller, source, target, x);
                vec![Event::LifeChanged {
                    player: gainer,
                    amount: self.life_gain_after_replacements(gainer, amount),
                    source: Some(source),
                }]
            }
            // Reality Shift's rider (CR 701.34): the *target's* controller manifests their top
            // library card — puts it onto the battlefield face down as a 2/2. Reads the target's
            // owner (control/ownership conflation, same as `GainLifeTargetController`), which stays
            // correct across the target's own exile (`owner_of` follows `Object::Moved`).
            Effect::Manifest => {
                let object = expect_object_target(target, "a manifest");
                let player = self.owner_of(object);
                let Some(&card) = self.players[player.0 as usize].library.first() else {
                    return Vec::new(); // an empty library manifests nothing (CR 701.34d).
                };
                vec![Event::Manifested {
                    permanent: self.next_object_id(),
                    from: card,
                    controller: player,
                }]
            }
            // Add `repeat` copies of the mana batch — one ManaAdded event per mana kind.
            // ponytail: a pool holds at most 255 of any one mana (u8); a burst this large never
            // arises in the soc pool, so an over-255 repeat saturates rather than widening the type.
            // `single_color` is handled by `Game::activate_ability` before a mana ability ever (CR 605, CR 113)
            // reaches here (it pauses on `ChooseManaColor` instead) — ignored via `..`.
            Effect::AddMana {
                mana: produced,
                identity,
                opponent_colors,
                repeat,
                restriction,
                persist_until_end_of_turn,
                ..
            } => {
                // Wrap the static batch as [`Mana::Restricted`] if this ability's mana is
                // spend-restricted (Troyan, Gutsy Explorer) — a no-op otherwise. A granted mana
                // ability's batch (Galazeth Prismari) already arrives pre-wrapped from
                // `Game::granted_mana_abilities` with `restriction: None` here, so this is
                // harmless to call regardless.
                let produced = produced.restricted_by(restriction);
                let repeat = self
                    .resolve_count(repeat, controller, source, target, x)
                    .min(u8::MAX as u32) as u8;
                let mut events = Vec::new();
                let mut push = |mana: Mana, amount: u8| {
                    let amount = amount.saturating_mul(repeat);
                    if amount > 0 {
                        events.push(Event::ManaAdded {
                            player: controller,
                            mana,
                            amount,
                            persist: persist_until_end_of_turn,
                        });
                    }
                };
                for (color, &n) in Color::ALL.iter().zip(produced.colored.iter()) {
                    push(Mana::Color(*color), n);
                }
                push(Mana::Colorless, produced.colorless);
                push(Mana::Any, produced.any);
                // Dual credits (filter lands' "{W}{W}/{W}{B}/{B}{B}", a painland's colored mode).
                for (&(a, b), &n) in COLOR_PAIRS.iter().zip(produced.either.iter()) {
                    push(Mana::Either(a, b), n);
                }
                // Fixed 2-4 color-choice credits (Treva's Ruins' "{T}: Add {G}, {W}, or {U}"),
                // keyed by their WUBRG bitmask — the static-batch twin of the `either` loop above.
                for (mask, &n) in produced.of_colors.iter().enumerate() {
                    push(Mana::OfColors(mask as u8), n);
                }
                // Spend-restricted credits (Troyan's own restriction above, or a granted mana
                // ability's pre-wrapped batch — Galazeth's Treasures-style grant).
                for slot in produced.restricted {
                    if let Some((base, restriction)) = slot.key {
                        push(Mana::Restricted { base, restriction }, slot.amount);
                    }
                }
                // "One mana of any color in your commander's color identity" (CR 903.4, Arcane
                // Signet): resolved to a real credit now, since the identity depends on
                // `controller`'s commander — it can't be baked into the static `mana` batch above.
                if identity > 0
                    && let Some(credit) = self.commander_identity_credit(controller)
                {
                    push(credit, identity);
                }
                // "One mana of any color that a land an opponent controls could produce"
                // (Fellwar Stone, Exotic Orchard): resolved to a real credit now, since the
                // producible set depends on the current board — it can't be baked into the
                // static `mana` batch above. No credit at all (`None`) if no opponent land
                // produces a color.
                if opponent_colors > 0
                    && let Some(credit) = self.opponent_producible_colors_credit(controller)
                {
                    push(credit, opponent_colors);
                }
                events
            }
            // Pump / destroy / counters target a creature, so the chosen target is an object.
            Effect::PumpUntilEndOfTurn {
                power,
                toughness,
                keywords,
                ..
            } => {
                let object = expect_object_target(target, "a pump");
                vec![Event::TempBoost {
                    object,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                    keywords,
                    source_name,
                }]
            }
            // Self-pump: the ability's own source, no target (prowess). The source is already
            // known at resolution, so there's nothing to choose.
            Effect::PumpSelfUntilEndOfTurn {
                power,
                toughness,
                keywords,
            } => {
                // CR 608.2c: nothing to boost if the source has already left the battlefield —
                // e.g. it paid its own "Sacrifice a creature" cost (Fallen Ideal's granted
                // ability, where the host may sacrifice itself).
                if self.as_permanent(source).is_none() {
                    return Vec::new();
                }
                vec![Event::TempBoost {
                    object: source,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                    keywords,
                    source_name,
                }]
            }
            // Mass pump: every creature the controller controls, no target (Selfless Spirit,
            // Moonshaker Cavalry).
            Effect::PumpCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                keywords,
                filter,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.is_creature_on_battlefield(id)
                            && self.controller_of(id) == controller
                            && self.permanent_matches(&filter, id, controller, Some(source))
                    })
                    .map(|object| Event::TempBoost {
                        object,
                        power,
                        toughness,
                        keywords,
                        source_name,
                    })
                    .collect()
            }
            // Keyword-only mass grant to every permanent (creature or not) the controller
            // controls matching `filter`, no P/T (Silkguard's Auras/Equipment clause). The
            // noncreature-permanent twin of the mass pump above — same "you control" scan, no
            // creature gate.
            Effect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { keywords, filter } => {
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.controller_of(id) == controller
                            && self.permanent_matches(&filter, id, controller, Some(source))
                    })
                    .map(|object| Event::TempBoost {
                        object,
                        power: 0,
                        toughness: 0,
                        keywords,
                        source_name,
                    })
                    .collect()
            }
            // Mass base-P/T SET: every creature the controller controls has its base P/T set to
            // `power`/`toughness` until end of turn (Biomass Mutation). Same "you control" scan as
            // the mass pump, but a 7b base SET rather than a 7c delta.
            Effect::SetBasePtCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                other,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        (!other || id != source)
                            && self.is_creature_on_battlefield(id)
                            && self.controller_of(id) == controller
                    })
                    .map(|object| Event::BasePtSetUntilEndOfTurn {
                        object,
                        power,
                        toughness,
                    })
                    .collect()
            }
            // Single-target base-P/T SET: the chosen creature's base P/T is set until end of turn
            // (Quandrix Charm mode 2). The targeted twin of the mass set above.
            Effect::SetBasePtTargetUntilEndOfTurn {
                power, toughness, ..
            } => {
                let object = expect_object_target(target, "a base-P/T set");
                vec![Event::BasePtSetUntilEndOfTurn {
                    object,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                }]
            }
            // Manland self-animation (Restless Spire): the source land becomes a creature until end
            // of turn — an added type/subtype (613.4), a base-P/T SET (613.3(7b)), and granted
            // keywords, all on the source. Nothing to do if the source has left (CR 608.2c).
            Effect::AnimateSelfUntilEndOfTurn {
                add_types,
                add_subtypes,
                base_power,
                base_toughness,
                keywords,
                add_colors,
            } => {
                if self.as_permanent(source).is_none() {
                    return Vec::new();
                }
                let mut events = vec![
                    Event::TypesAddedUntilEndOfTurn {
                        object: source,
                        types: add_types,
                        subtypes: add_subtypes,
                        colors: add_colors,
                    },
                    Event::BasePtSetUntilEndOfTurn {
                        object: source,
                        power: base_power,
                        toughness: base_toughness,
                    },
                ];
                if !keywords.is_empty() {
                    events.push(Event::TempBoost {
                        object: source,
                        power: 0,
                        toughness: 0,
                        keywords,
                        source_name,
                    });
                }
                events
            }
            // "each other creature that's attacking one of your opponents gets +1/+1 until end
            // of turn." Fired by the enchanted creature's own attack trigger; `source` is the
            // Aura, so its host is the "other"-excluded creature.
            Effect::PumpOtherAttackersAttackingYourOpponents { power, toughness } => {
                let Some(host) = self.attached_to(source) else {
                    return Vec::new();
                };
                self.combat
                    .attackers
                    .iter()
                    .copied()
                    .filter(|&a| a != host)
                    .filter(|&a| self.is_creature_on_battlefield(a))
                    .filter(|&a| self.defender_of(a).is_some_and(|d| d != controller))
                    .map(|object| Event::TempBoost {
                        object,
                        power,
                        toughness,
                        keywords: &[],
                        source_name,
                    })
                    .collect()
            }
            // Contract (Scriv, the Obligator): "Whenever enchanted creature attacks, it gets
            // +2/+0 until end of turn if it's attacking one of your opponents. Otherwise, its
            // controller loses 2 life." `source` is the Aura, `controller` its own controller;
            // the host is `source`'s attachment, "one of your opponents" is the host's declared
            // defender being someone other than the Aura's controller. An unattached Aura (mid-SBA) (CR 704, CR 303.4, CR 108.3)
            // has no host (guard-return).
            Effect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
                power,
                toughness,
                life,
            } => {
                let Some(host) = self.attached_to(source) else {
                    return Vec::new();
                };
                let attacking_your_opponent =
                    self.defender_of(host).is_some_and(|d| d != controller);
                if attacking_your_opponent {
                    return vec![Event::TempBoost {
                        object: host,
                        power,
                        toughness,
                        keywords: &[],
                        source_name,
                    }];
                }
                vec![Event::LifeChanged {
                    player: self.controller_of(host),
                    amount: -(life as i32),
                    source: Some(source),
                }]
            }
            // Mass keyword strip: every creature an opponent of the controller controls loses
            // `keywords` and can't have them until end of turn (arcane_lighthouse).
            Effect::StripKeywordsFromOpponentsCreatures { keywords } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id) && self.controller_of(id) != controller
                })
                .map(|object| Event::KeywordsStripped { object, keywords })
                .collect(),
            // Static abilities are read during recompute, never resolved from the stack.
            Effect::AnthemStatic { .. }
            | Effect::KeywordAnthemStatic { .. }
            | Effect::TappedForManaBonus { .. }
            | Effect::PreventNoncombatDamageToOtherCreaturesYouControl
            | Effect::PreventDamageToSelfRemovingCounter
            | Effect::TriggerDoublingStatic { .. }
            | Effect::GrantManaAbility { .. }
            | Effect::GrantToAttached { .. }
            | Effect::SetAttachedBasePT { .. }
            | Effect::SetAttachedTypes { .. }
            | Effect::ControlAttached
            | Effect::ReduceSpellCost { .. }
            | Effect::AttackTax { .. }
            | Effect::CounterScaledAttackTax
            | Effect::CantBeAttackedBy { .. }
            | Effect::CounterReplacement { .. }
            | Effect::TokenReplacement { .. }
            | Effect::LifeGainReplacement { .. }
            | Effect::CastXReplacement { .. }
            | Effect::EntersWithCounters { .. }
            | Effect::CreaturesYouControlEnterWithCounters { .. }
            | Effect::NoMaximumHandSize
            | Effect::PlayFromGraveyardOncePerTurn => Vec::new(),
            // Equip resolves by attaching the Equipment (the ability's source) to the chosen
            // creature, replacing any prior attachment.
            Effect::Equip => {
                let host = expect_object_target(target, "equip");
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            // Shielded by Faith / Prison Term: attach this Aura (the ability's source) to the
            // entering creature — moving it off any host it's already attached to (CR 704.5n
            // simply drops the old attachment once `apply` overwrites `attached_to`). `entering`
            // is filled at trigger placement; `None` only in an unplaced card template, which
            // never reaches resolution. Re-checks the Aura's own `enchant` filter against the
            // entering permanent (CR 303.4f-style legality) — a no-op if it isn't a legal host,
            // even though the "you may" was accepted (FIDELITY_BACKLOG #156).
            Effect::AttachSelfToEntering { entering } => {
                let host = entering.expect("filled in from the entering trigger at placement");
                if !self.attachment_host_legal(source, host) {
                    return Vec::new();
                }
                vec![Event::AttachedTo {
                    object: source,
                    host: Some(host),
                }]
            }
            Effect::DestroyTarget {
                cant_be_regenerated,
                ..
            } => {
                let object = expect_object_target(target, "destroy");
                // Indestructible ignores "destroy" (CR 702.12b).
                if self.has_keyword(object, Keyword::Indestructible) {
                    return Vec::new();
                }
                // A regeneration shield replaces the next "destroy" this turn with a regeneration
                // (CR 701.15b), unless "can't be regenerated" turns it off (CR 701.15d).
                // ponytail: only this effect-driven destroy consults the shield; the CR 704.5g
                // lethal-marked-damage state-based destroy (also a "destroy" a shield should
                // replace) does not — unobserved, since no pool card grants a shield. Upgrade:
                // consult the shield in `apply`'s SBA death sweep for the lethal-damage case too. (CR 704, CR 120.3)
                if !cant_be_regenerated && self.permanent(object).regeneration_shields > 0 {
                    return vec![Event::Regenerated { object }];
                }
                // A destroyed commander may divert to the command zone.
                vec![self.graveyard_or_command(object, self.next_object_id())]
            }
            // Mass destruction: every matching permanent goes to the graveyard (a commander
            // diverts; a token ceases to exist). Ids are minted sequentially, matching the order
            // `apply` will push them (as the SBA death sweep does). (CR 704)
            Effect::DestroyAll { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    // Indestructible survives a board wipe's "destroy" (CR 702.12b).
                    if self.has_keyword(id, Keyword::Indestructible) {
                        continue;
                    }
                    if p.token {
                        events.push(Event::TokenCeasedToExist {
                            token: id,
                            controller: p.owner,
                            def: p.def,
                        });
                        continue;
                    }
                    events.push(self.graveyard_or_command(id, next));
                    next += 1;
                }
                events
            }
            // Mass exile: every matching permanent goes to exile (a commander diverts; a token
            // ceases to exist). Unlike `DestroyAll`, there's no indestructible guard — exile (CR 702.12, CR 111.7, CR 406.5)
            // isn't "destroy" (CR 701.18a vs CR 702.12b) — and no graveyard branch, just the
            // exile-or-command-zone choke point `ExileTarget` already uses (CR 903.9b).
            Effect::ExileAll { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    if p.token {
                        events.push(Event::TokenCeasedToExist {
                            token: id,
                            controller: p.owner,
                            def: p.def,
                        });
                        continue;
                    }
                    events.push(self.exile_or_command(id, next));
                    next += 1;
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
            Effect::DamageEachCreature {
                amount,
                opponents_only,
            } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.is_creature_on_battlefield(id))
                .filter(|&id| !opponents_only || self.controller_of(id) != controller)
                // Protection from the source's color prevents that creature's share (CR 702.16d).
                .filter(|&id| !self.damage_prevented_by_protection(id, Some(source)))
                // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                .filter(|&id| !self.noncombat_damage_prevented_to_creature(id))
                // Phantom Centaur's self-shield prevents its own share and removes one of its
                // own +1/+1 counters instead (CR 615) — a shielded creature swaps its
                // `DamageMarked` for that counter removal rather than being filtered out outright.
                .flat_map(|object| {
                    if self.phantom_shield_active(object) {
                        return self.phantom_shield_counter_removal(object).into_iter().collect();
                    }
                    vec![Event::DamageMarked {
                        object,
                        amount: self.resolve_amount(amount, controller, object, target, x),
                        source: Some(source),
                    }]
                })
                .collect(),
            // Mass weaken: every creature gets -power/-toughness until end of turn (a negative
            // TempBoost, cleared at cleanup). A 0-or-less-toughness creature dies to the next SBA. (CR 704, CR 514)
            Effect::WeakenEachCreature {
                power,
                toughness,
                opponents_only,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| self.is_creature_on_battlefield(id))
                    .filter(|&id| !opponents_only || self.controller_of(id) != controller)
                    .map(|object| Event::TempBoost {
                        object,
                        power: -power,
                        toughness: -toughness,
                        keywords: &[],
                        source_name,
                    })
                    .collect()
            }
            Effect::ExileTarget { .. } => {
                let object = expect_object_target(target, "exile");
                vec![self.exile_or_command(object, self.next_object_id())]
            }
            // The O-Ring pattern (CR 603.6e): exile the target, linking it to this ability's own
            // `source` (the Aura) so `Game::check_linked_exile_returns` can send it back once the
            // Aura leaves.
            Effect::ExileUntilSourceLeaves { .. } => {
                let object = expect_object_target(target, "exile-until-source-leaves");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — it's never actually placed in exile, so there's nothing to
                // link back to this source.
                let permanent = self
                    .as_permanent(object)
                    .expect("exile-until-source-leaves resolves against a battlefield permanent");
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledUntilSourceLeaves {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Skyclave Apparition's linked exile (a sibling of `ExileUntilSourceLeaves`, not a
            // fork of its list): exile the target, linking it to this ability's own `source` so
            // `Game::check_leaves_battlefield_illusions` can mint its owner an Illusion once
            // `source` leaves. Unlike the O-Ring pattern, the card is never returned.
            Effect::ExileTargetMintingIllusionOnLeave { .. } => {
                let object = expect_object_target(target, "exile-minting-illusion-on-leave");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — nothing to link back to this source.
                let permanent = self.as_permanent(object).expect(
                    "exile-minting-illusion-on-leave resolves against a battlefield permanent",
                );
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledUntilSourceLeavesMintingIllusion {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Flicker (CR 400.7 — a new object, Momentary Blink/Mistmeadow Witch): exile the
            // target creature, then either return it immediately under its owner's control
            // (`return_at` absent) or schedule that return as a real CR 603.7 delayed triggered
            // ability at `return_at`'s step (`ReturnFlickeredCard`, carrying the specific card now
            // sitting in exile).
            Effect::FlickerTarget { return_at, .. } => {
                let object = expect_object_target(target, "flicker");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — nothing to flicker back.
                let permanent = self
                    .as_permanent(object)
                    .expect("flicker resolves against a battlefield permanent");
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let owner = permanent.owner;
                let mut next = self.next_object_id();
                let exiled = next;
                next += 1;
                let exile_event = self.exile_or_command(object, exiled);
                // CR 903.9b: a commander diverted to the command zone instead of exile was never
                // exiled — nothing returns.
                if matches!(exile_event, Event::MovedToCommandZone { .. }) {
                    return vec![exile_event];
                }
                match return_at {
                    None => vec![
                        exile_event,
                        Event::FlickeredToBattlefield {
                            permanent: next,
                            from: exiled,
                            controller: owner,
                        },
                    ],
                    Some(fire_at) => vec![
                        exile_event,
                        Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at,
                            effect: Effect::ReturnFlickeredCard {
                                exiled: Some(exiled),
                            },
                        },
                    ],
                }
            }
            // The delayed payload `FlickerTarget` schedules when it carries a `return_at`
            // (Mistmeadow Witch): return the specific card `exiled` names to the battlefield under
            // its owner's control. Guard-return with no return if it's since left exile some
            // other way (CR 603.10a last-known information).
            Effect::ReturnFlickeredCard { exiled } => {
                let Some(exiled) = exiled else {
                    return Vec::new();
                };
                let exiled = self.current_id(exiled);
                if self.zone_of(exiled) != Zone::Exile {
                    return Vec::new();
                }
                vec![Event::FlickeredToBattlefield {
                    permanent: self.next_object_id(),
                    from: exiled,
                    controller: self.owner_of(exiled),
                }]
            }
            // Bojuka Bog / Remorseful Cleric: exile every card in the target player's graveyard.
            // Ids are minted sequentially, matching the order `apply` will push them (same
            // pattern as ReturnAllToHand's mass bounce).
            Effect::ExileGraveyard => {
                let Some(Target::Player(player)) = target else {
                    panic!("exile-graveyard resolves with a chosen player target");
                };
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.live_object_ids() {
                    if self.zone_of(id) != Zone::Graveyard || self.owner_of(id) != player {
                        continue;
                    }
                    events.push(Event::MovedToExile {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            // Final Act's "Exile all graveyards" mode: every player's graveyard, no target — the
            // mass twin of `ExileGraveyard` above, minus the `owner_of(id) != player` filter.
            Effect::ExileAllGraveyards => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.live_object_ids() {
                    if self.zone_of(id) != Zone::Graveyard {
                        continue;
                    }
                    events.push(Event::MovedToExile {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            Effect::ReturnToHand { .. } => {
                let object = expect_object_target(target, "bounce");
                let permanent = self
                    .as_permanent(object)
                    .expect("bounce resolves against a battlefield permanent");
                // A token leaving the battlefield ceases to exist rather than changing zones
                // (CR 111.7) — it never reaches the hand.
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                }]
            }
            // The ability's own source, wherever it now lives (Angelic Destiny: by the time an
            // `EnchantedCreatureDies` trigger resolves, the Aura is already a graveyard card, not
            // a permanent) — a no-target self-return. Guard-return if the source has left the game
            // entirely (CR 603.6c last-known-info edge; no pool card leaves it any other way).
            Effect::ReturnThisToHand => {
                let current = self.current_id(source);
                if matches!(self.objects[current as usize], Object::Removed) {
                    return Vec::new();
                }
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: current,
                }]
            }
            // Nether Traitor: the ability's own source (a graveyard card by now) returns to the
            // battlefield under its owner's control (CR 603.6e). The self-return twin of
            // `ReanimateToBattlefield` — enters via the same ETB path. No-op if it has already left.
            // Teacher's Pest activates this from the graveyard directly (CR 112.6) with
            // `tapped = true`.
            Effect::ReturnThisFromGraveyardToBattlefield { tapped } => {
                let current = self.current_id(source);
                if matches!(self.objects[current as usize], Object::Removed) {
                    return Vec::new();
                }
                vec![Event::ReanimatedToBattlefield {
                    permanent: self.next_object_id(),
                    from: current,
                    controller,
                    finality: false,
                    tapped,
                }]
            }
            // Mass bounce: every matching permanent returns to its owner's hand (a token ceases to
            // exist). Ids are minted sequentially, matching the order `apply` will push them.
            Effect::ReturnAllToHand { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    if p.token {
                        events.push(Event::TokenCeasedToExist {
                            token: id,
                            controller: p.owner,
                            def: p.def,
                        });
                        continue;
                    }
                    events.push(Event::ReturnedToHand {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            Effect::Mill { count, .. } => {
                let Some(Target::Player(player)) = target else {
                    panic!("mill resolves with a chosen player target");
                };
                self.mill_events(
                    player,
                    self.resolve_count(count, controller, source, target, x),
                )
            }
            Effect::ExileTopMayPlay {
                count,
                until_next_turn,
            } => {
                let n = self.resolve_count(count, controller, source, target, x);
                self.exile_top_may_play_events(controller, n, until_next_turn)
            }
            // Containment Construct's payoff: exile the just-discarded card from the graveyard
            // and grant permission to play it until end of turn.
            Effect::ExileFromGraveyardMayPlay { card } => {
                let from = card.expect("the discarded card is filled in at placement");
                vec![Event::ExiledFromGraveyardMayPlay {
                    player: controller,
                    card: self.next_object_id(),
                    from,
                }]
            }
            // Currency Converter's payoff: exile the just-discarded card into this ability's own
            // source-linked pile (no impulse-play permission — unlike `ExileFromGraveyardMayPlay`).
            // ponytail: guard-returns rather than panics if `card` is missing or has already moved
            // out of the graveyard (e.g. a second effect exiled it first) — the "may" just does
            // nothing, same shape as a fizzled optional trigger.
            Effect::ExileDiscardedWithThis { card } => {
                let Some(from) = card else {
                    return Vec::new();
                };
                if self.zone_of(from) != Zone::Graveyard {
                    return Vec::new();
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(from, exiled),
                    Event::ExiledWithSource {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Quintorius's end step: exile the chosen graveyard card into this source's own
            // exiled-with pile — same shape as `ExileDiscardedWithThis` above, but the card is a
            // chosen target rather than a just-discarded one, and there's no impulse-play
            // permission (the free-cast permission comes later, from the activated ability). (CR 602, CR 601, CR 113)
            Effect::ExileTargetFromGraveyardWithThis => {
                let object = expect_object_target(target, "exile target from graveyard with this");
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledWithSource {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Restore Relic: exile the targeted graveyard card, then mint a token copy of its
            // copiable characteristics (CR 707.2) — `CreateTokenCopy`'s target-a-battlefield-
            // permanent shape, but reading `def` off the graveyard card before it moves.
            Effect::ExileTargetFromGraveyardCreateTokenCopy { .. } => {
                let object =
                    expect_object_target(target, "exile target from graveyard, create a copy");
                let def = self.def_of(object);
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                let token = exiled + 1;
                vec![
                    move_event,
                    Event::TokenCreated {
                        token,
                        controller,
                        def,
                    creator: source,
                },
                ]
            }
            // `kind = Some(k)` (Staff of the Storyteller's story counter) bypasses the +1/+1
            // replacement pipeline entirely, same as `EntersWithCounters`'s own kind split above.
            Effect::PutCounters {
                count,
                kind: Some(kind),
                ..
            } => {
                let object = expect_object_target(target, "a kind-counter effect");
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                if count <= 0 {
                    return Vec::new();
                }
                vec![Event::KindCountersPlaced {
                    object,
                    kind,
                    count,
                }]
            }
            Effect::PutCounters {
                count,
                kind: None,
                divided,
                ..
            } => {
                let object = expect_object_target(target, "a counter effect");
                // A divided spell's per-target count was already settled (CR 601.2d) right after
                // targets were chosen — see `Game::maybe_begin_counter_division` — and recorded
                // on the resolving spell (`source` is that spell's own object id; `divided` only
                // appears on `Timing::Spell` effects, so this always resolves through the spell
                // path, mirroring `Effect::DealDamage`'s own divided read).
                let count = if divided {
                    self.spell(source)
                        .counter_division
                        .pairs()
                        .into_iter()
                        .find_map(|(t, amt)| (t == object).then_some(amt))
                        .unwrap_or(0)
                } else {
                    self.resolve_count(count, controller, source, target, x) as i32
                };
                let n = self.counters_after_replacements(object, count);
                if n <= 0 {
                    return Vec::new();
                }
                vec![Event::CountersPlaced {
                    object,
                    count: n,
                    source_name,
                }]
            }
            // Double the target's +1/+1 counters: place as many more as it already has (CR 614).
            Effect::DoubleCounters { .. } => {
                let object = expect_object_target(target, "a counter-doubling effect");
                self.doubled_counters_event(object, source_name)
                    .into_iter()
                    .collect()
            }
            // Put `count` +1/+1 counters on each battlefield permanent matching `filter`
            // (Mazirek: "each creature you control"; Shadrix Silverquill's begin-combat "Target
            // player puts a +1/+1 counter on each creature they control" reads `filter`'s
            // `you`/`opponent` axis from the chosen Player target's perspective instead).
            // Ids are snapshotted via `battlefield()` up front, same as `DestroyAll`.
            Effect::PutCountersEach {
                filter,
                count,
                target_player,
            } => {
                let you = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!(
                            "a target-player counters-each effect resolves with a chosen player target"
                        );
                    };
                    player
                } else {
                    controller
                };
                let count = self.resolve_count(count, controller, source, target, x) as i32;
                self.battlefield()
                    .into_iter()
                    .filter(|&id| self.permanent_matches(&filter, id, you, Some(source)))
                    .filter_map(|object| {
                        let n = self.counters_after_replacements(object, count);
                        (n > 0).then_some(Event::CountersPlaced {
                            object,
                            count: n,
                            source_name,
                        })
                    })
                    .collect()
            }
            // Promise of Loyalty's rider: place a vow counter on each surviving creature, marking
            // the controller (the caster — "can't attack *you*") as the protected player. Scans
            // every player's creatures matching `filter` (the survivors an all-players keep-one
            // edict left — see the `PlaceVowCounters` doc), not just the controller's own.
            Effect::PlaceVowCounters { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(&filter, id, controller, Some(source)))
                .map(|object| Event::VowCountersPlaced {
                    object,
                    protected: controller,
                })
                .collect(),
            // Nexus Mentality's other mode: "Remove all counters from target nonland permanent
            // you control. Draw a card for each counter removed this way."
            Effect::RemoveAllCountersThenDraw { .. } => {
                let object = expect_object_target(target, "a remove-all-counters-then-draw effect");
                let (mut events, removed) = self.remove_all_counters_events(object);
                events.extend(self.draw_events(controller, removed as u32));
                events
            }
            // Breena: the attacking player (context) draws one; the controller's chosen creature
            // gets `counters` +1/+1 counters.
            Effect::AttackerDrawsControllerCounters { attacker, counters } => {
                let drawer = attacker.expect("the attacking player is filled in at placement");
                let object = expect_object_target(target, "Breena's counter half");
                let mut events = self.draw_events(drawer, 1);
                let n = self.counters_after_replacements(object, counters as i32);
                if n > 0 {
                    events.push(Event::CountersPlaced {
                        object,
                        count: n,
                        source_name,
                    });
                }
                events
            }
            // Parasitic Impetus: the enchanted creature's controller (context) loses `amount`
            // life; this ability's controller (the Aura's controller) gains the same.
            Effect::AttackerLosesLifeYouGain { attacker, amount } => {
                let loser = attacker.expect("the attacking player is filled in at placement");
                let amount = amount as i32;
                vec![
                    Event::LifeChanged {
                        player: loser,
                        amount: -amount,
                        source: Some(source),
                    },
                    Event::LifeChanged {
                        player: controller,
                        amount: self.life_gain_after_replacements(controller, amount),
                        source: Some(source),
                    },
                ]
            }
            // Tomik: the attacking opponent (context) loses `life_loss` life; this ability's
            // controller draws a card.
            Effect::AttackerLosesLifeYouDraw {
                attacker,
                life_loss,
            } => {
                let loser = attacker.expect("the attacking player is filled in at placement");
                let mut events = vec![Event::LifeChanged {
                    player: loser,
                    amount: -(life_loss as i32),
                    source: Some(source),
                }];
                events.extend(self.draw_events(controller, 1));
                events
            }
            // Firemane Commando: the attacking player (context) draws `count`, not this
            // ability's controller.
            Effect::AttackingPlayerDraws { drawer, count } => {
                let drawer = drawer.expect("the attacking player is filled in at placement");
                self.draw_events(drawer, count)
            }
            // Marauding Raptor: 2 damage to the permanent that just entered (context), not a
            // chosen target. `then_if_subtype`/`then` (the Dinosaur pump rider) are handled by
            // the caller in `run` — this leaf only deals the damage.
            Effect::DealDamageToEnteringPermanent {
                entering, amount, ..
            } => {
                let object = entering.expect("the entering permanent is filled in at placement");
                if self.damage_prevented_by_protection(object, Some(source)) {
                    return Vec::new();
                }
                // Phantom Centaur's self-shield prevents this damage outright and removes one
                // of its own +1/+1 counters instead (CR 615).
                if self.phantom_shield_active(object) {
                    return self.phantom_shield_counter_removal(object).into_iter().collect();
                }
                // Tajic prevents noncombat damage to its controller's other creatures (CR 615).
                if self.noncombat_damage_prevented_to_creature(object) {
                    return Vec::new();
                }
                vec![Event::DamageMarked {
                    object,
                    amount,
                    source: Some(source),
                }]
            }
            // Blood Artist: the target player loses life, the controller gains the same.
            Effect::DrainTarget { amount, .. } => {
                let Some(Target::Player(loser)) = target else {
                    panic!("a targeted drain resolves with a chosen player target");
                };
                vec![
                    Event::LifeChanged {
                        player: loser,
                        amount: -amount,
                        source: Some(source),
                    },
                    Event::LifeChanged {
                        player: controller,
                        amount: self.life_gain_after_replacements(controller, amount),
                        source: Some(source),
                    },
                ]
            }
            // Questing Phelddagrif: the target player gains life, with no matching loss.
            Effect::TargetPlayerGainsLife { amount, .. } => {
                let Some(Target::Player(gainer)) = target else {
                    panic!("target-player-gains-life resolves with a chosen player target");
                };
                vec![Event::LifeChanged {
                    player: gainer,
                    amount: self.life_gain_after_replacements(gainer, amount),
                    source: Some(source),
                }]
            }
            // Zulaport Cutthroat: each opponent loses life; the controller gains a flat
            // `amount` — or, for Exsanguinate's "life lost this way", the summed total.
            Effect::EachOpponentDrain { amount, sum_gain } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                let opponents: Vec<PlayerId> =
                    self.living_players().filter(|&p| p != controller).collect();
                let mut events: Vec<Event> = opponents
                    .iter()
                    .map(|&opponent| Event::LifeChanged {
                        player: opponent,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect();
                let gain = if sum_gain {
                    amount * opponents.len() as i32
                } else {
                    amount
                };
                events.push(Event::LifeChanged {
                    player: controller,
                    amount: self.life_gain_after_replacements(controller, gain),
                    source: Some(source),
                });
                events
            }
            // Dina, Soul Steeper: each opponent loses life, with no lifegain half (a gain would
            // re-trigger her "whenever you gain life" ability into a loop).
            Effect::EachOpponentLosesLife { amount } => {
                let amount = self.resolve_amount(amount, controller, source, target, x);
                self.living_players()
                    .filter(|&p| p != controller)
                    .map(|opponent| Event::LifeChanged {
                        player: opponent,
                        amount: -amount,
                        source: Some(source),
                    })
                    .collect()
            }
            // Raise Dead: send the chosen graveyard creature card to its owner's hand. Reuses
            // the bounce event (both move an object to its owner's hand); the graveyard card
            // isn't on the stack, so that event's stack cleanup is a harmless no-op.
            Effect::ReturnFromGraveyardToHand { .. } => {
                let object = expect_object_target(target, "graveyard recursion");
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                }]
            }
            // Reanimate: put the chosen graveyard creature card onto the battlefield under the
            // ability's controller's control (enters via the ETB path — see the event's apply arm).
            // Excava, the Risen Past's `becomes` rider follows with a `ReanimatedCreatureBecame` on
            // the just-entered permanent — the "It's a 1/1 Spirit creature with flying" indefinite
            // set (CR 611.2c). A plain reanimation (`becomes == None`) is just the one event.
            Effect::ReanimateToBattlefield {
                finality, becomes, ..
            } => {
                let object = expect_object_target(target, "reanimation");
                let entered = self.reanimate_event(object, controller, finality);
                let Some(becomes) = becomes else {
                    return vec![entered];
                };
                let Event::ReanimatedToBattlefield { permanent, .. } = entered else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                vec![
                    entered,
                    Event::ReanimatedCreatureBecame {
                        object: permanent,
                        add_types: becomes.add_types,
                        add_subtypes: becomes.add_subtypes,
                        base_power: becomes.base_power,
                        base_toughness: becomes.base_toughness,
                        keywords: becomes.keywords,
                    },
                ]
            }
            // Changing Loyalty / Gift of Immortality: reanimate the creature this Aura was
            // enchanting when it died, under either this ability's own controller ("your
            // control") or that card's owner ("its owner's control"). `dying` is the pre-death
            // battlefield id — `current_id` follows its `Moved` lineage into whatever object it
            // is now.
            Effect::ReanimateDyingEnchantedCreature { dying, under_owner } => {
                let Some(dying) = dying else {
                    return Vec::new();
                };
                let card = self.current_id(dying);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let new_controller = if under_owner {
                    self.owner_of(card)
                } else {
                    controller
                };
                vec![self.reanimate_event(card, new_controller, false)]
            }
            // Hofri Ghostforge: "exile it. If you do, create a token that's a copy of that
            // creature, except it's a Spirit in addition to its other types ...". `dead` is the
            // pre-death battlefield id; `current_id` follows its `Moved` lineage into the graveyard
            // card. Guard-return with no mint if it's no longer in a graveyard (exiled/moved in
            // response — the "if you do" fails). Reads the copiable `def` off the card before it
            // exiles, mints the token copy (CR 707.2) under `controller`, then adds `add_subtypes`
            // on the minted token (CR 613.4 subtype layer, indefinite).
            Effect::ExileDeadCreatureCreateCopyWithSubtype {
                dead,
                add_subtypes,
                leaves_returns_exiled,
            } => {
                let Some(dead) = dead else {
                    return Vec::new();
                };
                let card = self.current_id(dead);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let def = self.def_of(card);
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(card, exiled);
                let token = exiled + 1;
                let mut events = vec![
                    move_event,
                    Event::TokenCreated {
                        token,
                        controller,
                        def,
                    creator: source,
                },
                ];
                if !add_subtypes.is_empty() {
                    events.push(Event::AddedSubtypes {
                        object: token,
                        subtypes: add_subtypes,
                    });
                }
                // "... and it has 'When this token leaves the battlefield, return the exiled
                // card to its owner's graveyard.'" — link the minted token to the exiled card;
                // `Game::queue_token_return_exiled_trigger` reads this once `token` leaves.
                if leaves_returns_exiled {
                    events.push(Event::TokenGrantedReturnExiledOnLeave { token, exiled });
                }
                events
            }
            // Hofri Ghostforge's minted Spirit token's granted rider: "return the exiled card to
            // its owner's graveyard." `exiled` was baked in at mint time
            // (`Game::queue_token_return_exiled_trigger`). Guard-return with no move if that card
            // is no longer in exile (already reclaimed some other way) — the printed rider only
            // returns a card that's still exiled. `Event::ReturnedExiledCardToGraveyard`, not
            // `MovedToGraveyard` — see that event's doc for why (this isn't a death).
            Effect::ReturnExiledCardToOwnersGraveyard { exiled } => {
                if self.zone_of(exiled) != Zone::Exile {
                    return Vec::new();
                }
                vec![Event::ReturnedExiledCardToGraveyard {
                    card: self.next_object_id(),
                    from: exiled,
                }]
            }
            // Gift of Immortality: the delayed CR 603.7 payoff scheduled by
            // `ScheduleReturnThisAuraAttachedToReanimated`, fired at the next end step. Guard-
            // return with no return if this Aura has since left the graveyard (moved/exiled some
            // other way — CR 603.10a last-known information) or `creature` no longer resolves to
            // a battlefield permanent (destroyed before the delayed trigger fired). Otherwise
            // move the Aura graveyard→battlefield through the same shared reanimate choke
            // `ReanimateDyingEnchantedCreature` above uses, then attach it in the same batch
            // (`Event::AttachedTo`) rather than pausing to choose a host.
            Effect::ReturnThisAuraAttachedTo { creature } => {
                let card = self.current_id(source);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let Some(creature) = creature else {
                    return Vec::new();
                };
                let creature = self.current_id(creature);
                if self.zone_of(creature) != Zone::Battlefield {
                    return Vec::new();
                }
                let event = self.reanimate_event(card, self.owner_of(card), false);
                let Event::ReanimatedToBattlefield { permanent, .. } = event else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                vec![
                    event,
                    Event::AttachedTo {
                        object: permanent,
                        host: Some(creature),
                    },
                ]
            }
            // Mistveil Plains: put the chosen graveyard card on the bottom of its owner's
            // library. Mystic Sanctuary sets `to_top` for its "on top of your library" instead.
            Effect::TuckFromGraveyard { to_top, .. } => {
                let object = expect_object_target(target, "graveyard tuck");
                vec![Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top,
                }]
            }
            // Temporal Spring ("Put target permanent on top of its owner's library") and
            // Condemn's tuck half ("Put target attacking creature on the bottom of its owner's
            // library"): put a targeted battlefield permanent into its owner's library at a fixed
            // position. No shuffle — unlike its fused sibling `ShuffleTargetPermanentIntoLibraryThenReveal`
            // above, this needs no `&mut self` and stays in the pure event-building path.
            Effect::TuckPermanentIntoLibrary { to_top, .. } => {
                let object = expect_object_target(target, "a permanent to tuck");
                let owner = self.owner_of(object);
                // CR 111.7: a token can't exist in a library — it ceases to exist instead.
                if self.permanent(object).token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: owner,
                        def: self.def_of(object),
                    }];
                }
                vec![Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top,
                }]
            }
            // Replenish (Eiganjo Dynastorian's back face): every matching card in the
            // controller's own graveyard returns to the battlefield under their control, with no
            // finality counter. Ids are minted sequentially, matching the order `apply` will push
            // them (same pattern as `ReturnAllToHand`'s mass bounce).
            Effect::MassReturnFromGraveyard { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.live_object_ids() {
                    if self.zone_of(id) != Zone::Graveyard || self.owner_of(id) != controller {
                        continue;
                    }
                    if !filter.matches(self.def_of(id)) {
                        continue;
                    }
                    events.push(Event::ReanimatedToBattlefield {
                        permanent: next,
                        from: id,
                        controller,
                        finality: false,
                        tapped: false,
                    });
                    next += 1;
                }
                events
            }
            // Pausing effects never resolve to plain events — `run` intercepts
            // them and pauses on their PendingChoice before reaching this point.
            Effect::Scry { .. }
            // Needs `&mut self` to arm the prevention shield on `Game::combat_extras` — only
            // resolves via `Game::run`.
            | Effect::PreventCombatDamageToYouCreatingTokens { .. }
            | Effect::PreventAllCombatDamageThisTurn
            | Effect::Surveil { .. }
            | Effect::LookAtTop { .. }
            | Effect::DistributeTop { .. }
            | Effect::ExileTopCastMatchingFree { .. }
            | Effect::RevealUntilMayDeploy { .. }
            | Effect::RevealUntilExileCastFree { .. }
            | Effect::Cascade { .. }
            | Effect::SearchLibrary { .. }
            | Effect::EachPlayerSacrifices { .. }
            | Effect::EachPlayerExilesFromGraveyard
            | Effect::TargetPlayerExilesFromGraveyard { .. }
            | Effect::CasterKeepsOneOfEachTypePerPlayer
            | Effect::EachPlayerControllerChoosesCounterTarget
            | Effect::CouncilsDilemmaVote { .. }
            | Effect::OpponentSplitsExilePiles
            | Effect::RevealTopSplitPiles
            | Effect::EachPlayerExilesUntilNonlandOpponentPicks
            | Effect::EachPlayerCreatesFractalFromExiledPower { .. }
            | Effect::EachOtherTokenBecomesCopyOfChosen
            | Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. }
            | Effect::EachPlayerDiscardsHandThenDraws { .. }
            | Effect::MaySacrifice { .. }
            | Effect::SacrificeOwn { .. }
            | Effect::DefendingPlayerSacrifices { .. }
            | Effect::MayReturnFromGraveyard { .. }
            | Effect::MayDiscard { .. }
            // Needs `&mut self` to pause on the MayYesNo/PayOrControllerDraws chain — only
            // resolves via `Game::run`, never this pure path.
            | Effect::MayDrawUnlessPays { .. }
            // Needs `&mut self` to pause the targeted player on a MayYesNo — only resolves via
            // `Game::run`, never this pure path.
            | Effect::TargetPlayerMayDraw { .. }
            | Effect::ShuffleTargetCardsFromGraveyardIntoLibrary { .. }
            | Effect::Discard { .. }
            | Effect::PutLandFromHand { .. }
            | Effect::CastCreatureFaceDown
            | Effect::CashOutExiledWithThis
            | Effect::CastExiledWithThisFree
            | Effect::Fight { .. }
            | Effect::ChooseOne { .. }
            | Effect::ChooseCreatureType
            | Effect::ChooseColor
            | Effect::CopyTargetSpell
            | Effect::CopyThisSpell { .. }
            | Effect::RetargetSpellCopy { .. }
            // Pauses on `ChooseSpellTargets` to bend the chosen spell — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ChangeTargetOfTargetSpellOrAbility { .. }
            | Effect::CopyTriggeringSpell { .. }
            | Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. }
            // Needs `&mut self` to mint the ability copy (`push_ability_group_with_x`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::CopyTriggeringAbility { .. }
            | Effect::Demonstrate { .. }
            // Records onto `Game::pending_enter_bonus_counters` — needs `&mut self`, only resolves
            // via `Game::run`, never this pure path.
            | Effect::CommanderEntersWithBonusCounters { .. }
            | Effect::Sequence { .. }
            | Effect::Conditional { .. }
            | Effect::Proliferate { .. }
            | Effect::PhaseOut
            | Effect::DoubleCountersOnTargetCreatures { .. }
            | Effect::MoveCounters { .. }
            | Effect::UntapSearchedLand
            | Effect::AttachTriggeringAuraToMintedToken { .. }
            | Effect::ReflexiveTrigger { .. }
            | Effect::ReturnFromGraveyardAttachedToToken { .. }
            | Effect::AttachSelfToReanimated
            | Effect::AttachSelfToMintedToken
            | Effect::AttachMintedAuraToTarget { .. }
            | Effect::DoubleCountersOnAttachedCreature
            | Effect::ScheduleReturnThisAuraAttachedToReanimated
            // Needs `&mut self` to pause on `ChooseAttachHost` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost
            | Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost
            // Needs `&mut self` to draw from the injected RNG — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileRandomFromGraveyardMayPlay
            | Effect::ShuffleLibrary
            // Player-driven exile loop + a running tally across pauses — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTopUntilStopCastFreeUnderBudget { .. }
            // Needs `&mut self` to read the actual post-shuffle library order — only resolves
            // via `Game::run`, never this pure path.
            | Effect::ShuffleTargetPermanentIntoLibraryThenReveal { .. }
            // Needs `&mut self` to mint the exiled object id (`Game::next_object_id`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardSpellCastFree { .. }
            // Needs `&mut self` to write `Game::surge_exiled_card` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardCardRecordManaValue { .. }
            // Needs `&mut self` to mark `Game::self_exile_time_counters` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileSelfWithTimeCounters { .. }
            // Needs `&mut self` to mint the free copy (`Game::mint_spell_copies`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::MintFreeCopyOfExiledCard { .. }
            // Needs `&mut self` to conditionally `run_sequence` its `then` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardCardThenIfCreature { .. }
            // Needs `&mut self` to pause on `SacrificeUnlessPay` — only resolves via `Game::run`,
            // never this pure path.
            | Effect::SacrificeSelfUnlessPay { .. }
            // Needs `&mut self` to scan the battlefield and pause on `SacrificeUnlessReturnLand`
            // (or sacrifice outright with no candidates) — only resolves via `Game::run`, never
            // this pure path.
            | Effect::SacrificeSelfUnlessReturnLand { .. } => {
                unreachable!("a pausing/composite effect resolves via Game::run")
            }
            Effect::GoadTarget { .. } => {
                let object = expect_object_target(target, "goad");
                vec![Event::Goaded {
                    object,
                    by: controller,
                    source_name,
                }]
            }
            Effect::TapTarget { .. } => {
                let object = expect_object_target(target, "tap");
                vec![Event::Tapped { object }]
            }
            Effect::RegenerateShield { .. } => {
                let object = expect_object_target(target, "a regeneration shield");
                vec![Event::RegenerationShieldCreated { object }]
            }
            Effect::UntapTarget { .. } => {
                let object = expect_object_target(target, "untap");
                vec![Event::Untapped { object }]
            }
            Effect::GainControlUntilEndOfTurn { .. } => {
                let object = expect_object_target(target, "a steal");
                vec![Event::ControlGainedUntilEndOfTurn {
                    object,
                    controller,
                    source_name,
                }]
            }
            Effect::GainControl { .. } => {
                let object = expect_object_target(target, "a permanent control change");
                vec![Event::ControlGained { object, controller }]
            }
            Effect::GainControlWhile {
                while_source_tapped,
                ..
            } => {
                let object = expect_object_target(target, "a conditioned steal");
                vec![Event::ConditionedControlGained {
                    object,
                    controller,
                    condition: crate::ControlCondition {
                        source,
                        needs_tapped: while_source_tapped,
                    },
                }]
            }
            // Backup's rider (CR 702.166): the shared target creature gains the source's other
            // abilities until end of turn — but only "if that's another creature", so the source
            // targeting itself grants nothing (the counter still landed in the preceding step).
            Effect::GrantSourceAbilitiesUntilEndOfTurn => {
                let object = expect_object_target(target, "Backup's ability grant");
                if object == source {
                    return Vec::new();
                }
                vec![Event::AbilitiesGranted {
                    target: object,
                    source,
                }]
            }
            // Beledros: untap every matching permanent the controller controls — the mass
            // mirror of UntapTarget, same "you control" scoping as PumpCreaturesYouControlUntilEndOfTurn.
            Effect::UntapAll { filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .map(|object| Event::Untapped { object })
                .collect(),
            // Faerie Mastermind: every player draws, not just the controller. Ids are minted
            // sequentially across every player's batch in one pass — draw_events can't be
            // called once per player here since each call restarts from the same
            // not-yet-applied next_object_id (see DestroyAll's `next` for the same reason).
            Effect::EachPlayerDraws { count } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for p in self.living_players() {
                    let library = &self.players[p.0 as usize].library;
                    for i in 0..count as usize {
                        match library.get(i) {
                            Some(&from) => {
                                events.push(Event::CardDrawn {
                                    player: p,
                                    object: next,
                                    from,
                                    card: self.def_of(from),
                                });
                                next += 1;
                            }
                            None => events.push(Event::DrewFromEmptyLibrary { player: p }),
                        }
                    }
                }
                events
            }
            // Ominous Harvest: the target player loses life, with no matching gain.
            Effect::TargetPlayerLosesLife { amount } => {
                let Some(Target::Player(player)) = target else {
                    panic!("target-player-loses-life resolves with a chosen player target");
                };
                vec![Event::LifeChanged {
                    player,
                    amount: -amount,
                    source: Some(source),
                }]
            }
            // Perpetual Timepiece: untargeted self-mill (unlike Mill's target-player shape).
            Effect::MillSelf { count } => {
                let count = self.resolve_count(count, controller, source, target, x);
                self.mill_events(controller, count)
            }
            // Kirol, History Buff: the source becomes prepared (idempotent if already prepared),
            // enabling its back-face copy cast (see `Game::cast_prepared`).
            Effect::BecomePrepared => vec![Event::PreparedChanged {
                object: source,
                prepared: true,
            }],
            // A Class's "Level N" ability (CR 717.2): the activation gate only offered this while
            // the source sat at level N-1, so resolution just records the new level.
            Effect::LevelUp { level } => vec![Event::LeveledUp { source, level }],
            // Stensian Sanguinist's attack trigger: arm a delayed watch on the just-deathtouched
            // shared target — its own source becomes prepared the first time that creature deals
            // combat damage to a player this combat (see `Game::fire_combat_damage_watch_triggers`). (CR 510, CR 120.3, CR 506)
            Effect::ArmCombatDamageWatch => {
                let watched = expect_object_target(target, "a combat-damage watch's armed target");
                vec![Event::CombatDamageWatchArmed {
                    controller,
                    source,
                    watched,
                }]
            }
            // Surge to Victory: arm the this-turn, controller-scoped, repeatable combat-damage-
            // copy watch over the card the preceding `Sequence` step just exiled. `None` (the
            // exile step never ran) is unreachable in practice — CR 608.2b already fizzles the
            // whole ability before either step resolves without a legal target — but a silent
            // no-op rather than a panic, matching this resolution's other snapshot-read arms.
            Effect::ScheduleThisTurnCombatDamageCopy => match self.surge_exiled_card {
                Some((card, _)) => vec![Event::CombatDamageCopyArmed {
                    controller,
                    source,
                    card,
                }],
                None => vec![],
            },
            // Ingenious Prodigy: "you may remove a +1/+1 counter from it." A negative
            // `CountersPlaced`, mirroring `RemoveAllCountersThenDraw`'s removal above; guarded so
            // a source with none doesn't go negative (unreachable in practice — the enclosing
            // ability's `SourceHasCounters` intervening-if already requires at least one).
            Effect::RemoveCounterFromSelf => {
                if self.plus_counters(source) <= 0 {
                    return vec![];
                }
                vec![Event::CountersPlaced {
                    object: source,
                    count: -1,
                    source_name,
                }]
            }
            // Alchemist's Refuge: "You may cast spells this turn as though they had flash." (CR 702.8, CR 601, CR 500)
            // ponytail: resolved as a one-shot turn-flag set (`Player::flash_permission_this_turn`) (CR 500)
            // rather than a continuous "as though they had flash" static — behaviorally identical (CR 702.8)
            // for this pool (gone at cleanup either way; nothing reads it mid-resolution before
            // the flag is set here).
            Effect::GrantFlashThisTurn => {
                vec![Event::FlashPermissionGranted { player: controller }]
            }
            // Yavimaya Bloomsage's Channel back face: "Until end of turn, any time you could (CR 605, CR 118.4)
            // activate a mana ability, you may pay 1 life. If you do, add {C}." Resolved as a
            // one-shot turn-flag set, mirroring `GrantFlashThisTurn` above.
            Effect::GrantChannelColorlessManaThisTurn => {
                vec![Event::ChannelColorlessManaGranted { player: controller }]
            }
            // Counter target spell (the unconditional hard-counter path — `unless_pays: Some(_)`
            // is intercepted earlier, in `run`, so this arm only ever sees `None`).
            Effect::CounterTargetSpell { .. } => {
                let original = expect_object_target(target, "a spell to counter");
                self.counter_spell(original)
            }
            // Counter target activated ability (CR 701.5c/112.7a — Azorius Guildmage). The target
            // is the ability's source id (see `TargetSpec::ActivatedAbilityOnStack`); the
            // `AbilityCountered` apply removes the topmost matching stack ability. A guard-return
            // (CR 608.2b) if it already left the stack is handled upstream by `target_still_legal`,
            // which fizzles this ability before it runs; this stays a no-op if nothing matches.
            Effect::CounterTargetActivatedAbility => {
                let source_id = expect_object_target(target, "an activated ability to counter");
                let on_stack = self.stack.iter().any(|item| {
                    matches!(item, StackItem::Ability { source, activated: true, .. } if *source == source_id)
                });
                if !on_stack {
                    return Vec::new();
                }
                vec![Event::AbilityCountered { source: source_id }]
            }
            // Schedule a CR 603.7 delayed trigger: resolve `who` to a concrete player now (the
            // effect itself doesn't fire until the matching step begins — see
            // `Game::fire_delayed_triggers`).
            Effect::ScheduleAtNextUpkeep { who, then, fire_at } => {
                let player = match who {
                    DelayController::You => controller,
                    DelayController::TargetSpellController => self.controller_of(
                        expect_object_target(target, "a delayed trigger's target-spell controller"),
                    ),
                };
                vec![Event::DelayedTriggerScheduled {
                    controller: player,
                    source,
                    fire_at,
                    effect: *then,
                }]
            }
            // Arm a CR 603.7 delayed one-shot: always the ability's own controller/source (Brass
            // Infiniscope has no "someone else's spell" wrinkle) — the watch itself doesn't fire
            // until a matching cast happens, see `Game::fire_next_cast_triggers`.
            Effect::ScheduleNextCastTrigger { filter, then } => {
                vec![Event::NextCastTriggerArmed {
                    controller,
                    source,
                    filter,
                    then,
                }]
            }
            // Sacrifice one already-resolved object (never authored directly — see the variant's
            // doc). Guard-return if it's already left the battlefield (destroyed/exiled some
            // other way before the delayed trigger fired): nothing left to sacrifice.
            Effect::SacrificeObject { object } => {
                let id = object.expect("filled in when the delayed sacrifice was scheduled");
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: controller,
                        def,
                    },
                ]
            }
            // Sacrifice the ability's own source (CR 701.16) — Court Hussar's "sacrifice it",
            // authorable directly (unlike `SacrificeObject` above). No zone guard needed: this
            // only ever runs synchronously off the source's own ETB, which can't have already
            // left the battlefield.
            Effect::SacrificeSource => {
                let def = self.def_of(source);
                vec![
                    self.sacrifice_event(source),
                    Event::Sacrificed {
                        object: source,
                        by: controller,
                        def,
                    },
                ]
            }
            // A `ThisPermanentLeavesBattlefield` look-back payoff (Animate Dead): "that creature's
            // controller sacrifices it" (CR 603.10a last-known information). Guard-return if the
            // triggering context never filled a host, or if that creature no longer sits on the
            // battlefield (it died first and the Aura fell off its own CR 704.5m SBA, or it was
            // bounced/exiled in response — the "that creature" reference fizzles). `by` reads the
            // creature's own current controller, not this ability's — CR "that creature's
            // controller", not "you".
            Effect::SacrificeEnchantedCreature { creature } => {
                let Some(id) = creature else {
                    return Vec::new();
                };
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: self.controller_of(id),
                        def,
                    },
                ]
            }
            // Exile one already-resolved object (never authored directly — see the variant's
            // doc). Guard-return if it's already left the battlefield (destroyed/exiled/bounced
            // some other way before the delayed trigger fired): nothing left to exile. A token
            // ceases to exist instead of actually changing zones (CR 111.7) — the same
            // exile-or-command-zone choke point `ExileAll`/`ExileTarget` already use (CR 903.9b).
            Effect::ExileObject { object } => {
                let id = object.expect("filled in when the delayed exile was scheduled");
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let permanent = self.permanent(id);
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: id,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                vec![self.exile_or_command(id, self.next_object_id())]
            }
            Effect::CreateToken {
                token,
                count,
                controller: token_controller,
                // `enters_with` needs the just-minted token already in game state to route
                // through `counters_after_replacements` (it reads the token's controller), so it
                // can't be placed here — `execute_effect` is pure (`&self`). `Game::run`
                // special-cases `CreateToken` to place counters right after applying this batch;
                // this arm only reaches direct `execute_effect` callers (a mana ability, a (CR 605, CR 113)
                // sacrifice edict's `then`), none of which mint a token with counters today.
                enters_with: _,
                set_base_pt,
                exile_at_next_end_step,
                enters_tapped_and_attacking: _,
                attacking_context,
                must_attack_defender,
            } => {
                // Mint sequential ids matching the order `apply` will push them (CR 111.1).
                let count = self.resolve_count(count, controller, source, target, x);
                // "…tokens … that attack that opponent this turn if able" (Furygale Flocking):
                // the flattened single-opponent defender every `controller` value but
                // `one_per_opponent` binds its tokens to (the one legal defending player in a
                // 1v1 game; with more opponents, still just the first one found — CR 508.1a).
                let flattened_defender = must_attack_defender
                    .then(|| self.living_players().find(|&p| p != controller))
                    .flatten();
                // Who receives the token(s), paired with the must-attack defender (if any) that
                // recipient's batch is bound to: the ability's own controller by default, the
                // shared target's controller (Beast Within's "its controller creates..."), one
                // copy per opponent under that opponent (a hostile edict), or one copy per
                // opponent under the ability's own controller (Eccentric Pestfinder's "for each
                // opponent, you create..." — Furygale Flocking's "for each opponent, create
                // two ... tokens ... that attack that opponent" additionally binds each
                // opponent's own batch to *that* opponent, not the flattened one). Combat
                // Calligrapher's tapped-and-attacking rider overrides all of that (CR 111.4): the
                // token is minted under the *attacking* player from `attacking_context`, not the
                // ability's controller.
                let batches: Vec<(PlayerId, Option<PlayerId>)> = match attacking_context {
                    Some((attacker, _defender)) => vec![(attacker, None)],
                    None => match token_controller {
                        TokenController::You => vec![(controller, flattened_defender)],
                        TokenController::TargetController => {
                            let object =
                                expect_object_target(target, "a token's target-controller");
                            vec![(self.controller_of(object), flattened_defender)]
                        }
                        TokenController::EachOpponent => self
                            .living_players()
                            .filter(|&p| p != controller)
                            .map(|p| (p, flattened_defender))
                            .collect(),
                        TokenController::OnePerOpponent => self
                            .living_players()
                            .filter(|&p| p != controller)
                            .map(|opponent| (controller, must_attack_defender.then_some(opponent)))
                            .collect(),
                        // Questing Phelddagrif's green rider: "Target opponent creates a 1/1 ...
                        // Hippo ... token" — same `Target::Player` resolution as `TargetPlayer`
                        // above, just narrowed to an opponent by `Effect::target`'s `TargetSpec`.
                        TokenController::TargetPlayer | TokenController::TargetOpponent => {
                            let Some(Target::Player(player)) = target else {
                                panic!("a token's target-player recipient resolves with a chosen player target");
                            };
                            vec![(player, flattened_defender)]
                        }
                    },
                };
                // "…create an X/X … token …, where X is …" (Manaform Hellkite): bake the
                // resolved base power/toughness straight into the minted def before any copies
                // are minted — a genuine base-P/T set, not `enters_with`'s counters. Resolving
                // needs no just-minted game state (unlike `enters_with`), so it's safe here.
                let mut def = token;
                if let Some(amount) = set_base_pt {
                    let n = self.resolve_amount(amount, controller, source, target, x);
                    if let CardKind::Creature {
                        power, toughness, ..
                    } = &mut def.kind
                    {
                        *power = n;
                        *toughness = n;
                    }
                }
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for (recipient, batch_defender) in batches {
                    // Doubling Season (CR 614): each batch may enter under a different player
                    // (Combat Calligrapher), so apply the recipient's token-creation replacements
                    // per batch.
                    let count = self.token_count_after_replacements(recipient, count);
                    for _ in 0..count {
                        events.push(Event::TokenCreated {
                            token: next,
                            controller: recipient,
                            def,
                    creator: source,
                });
                        // Attach the "attacks this turn if able" requirement to each minted token
                        // — bound to this batch's own defender (see `batches` above).
                        if let Some(defender) = batch_defender {
                            events.push(Event::MustAttackDeclared {
                                object: next,
                                defender,
                            });
                        }
                        // "…creates a tapped … token … that's attacking that opponent" (Combat
                        // Calligrapher): the token enters already tapped and joins combat as an
                        // attacker against the baked defender — CR 508.4, not a declared attack,
                        // so `TokenEnteredAttacking` (not `AttackerDeclared`) carries it.
                        if let Some((_attacker, defender)) = attacking_context {
                            events.push(Event::Tapped { object: next });
                            events.push(Event::TokenEnteredAttacking {
                                token: next,
                                defender,
                            });
                        }
                        // "Exile that token at the beginning of the next end step." (Manaform
                        // Hellkite, CR 603.7b): schedule a delayed exile against this specific
                        // minted token, not a re-scan (mirrors `CreateTokenCopy`'s
                        // `sacrifice_at_next_end_step`).
                        if exile_at_next_end_step {
                            events.push(Event::DelayedTriggerScheduled {
                                controller,
                                source,
                                fire_at: Step::End,
                                effect: Effect::ExileObject { object: Some(next) },
                            });
                        }
                        next += 1;
                    }
                }
                events
            }
            // Treasures reuse the token machinery with the shared `treasure_token` def, entering
            // under the ability's controller or a chosen target player (Prismari Command).
            Effect::CreateTreasure {
                count,
                target_player,
                tapped,
            } => {
                let recipient = if target_player {
                    let Some(Target::Player(player)) = target else {
                        panic!(
                            "target-player create-treasure resolves with a chosen player target"
                        );
                    };
                    player
                } else {
                    controller
                };
                let count = self.resolve_count(count, controller, source, target, x);
                // Doubling Season doubles Treasures too — they are tokens (CR 614).
                let count = self.token_count_after_replacements(recipient, count);
                let mut events = Vec::new();
                for next in (self.next_object_id()..).take(count as usize) {
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: recipient,
                        def: treasure_token(),
                    creator: source,
                });
                    // "create a number of tapped Treasure tokens" (Goldvein Hydra): each minted
                    // Treasure enters already tapped.
                    if tapped {
                        events.push(Event::Tapped { object: next });
                    }
                }
                events
            }
            // A token copy of the target creature: reuse the token machinery with the target's
            // current copiable characteristics (its `CardDef`). If the target is itself a token,
            // `def_of` returns its token def — which is exactly what we want to copy.
            Effect::CreateTokenCopy {
                count,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                haste,
                ..
            } => {
                const HASTE: &[Keyword] = &[Keyword::Haste];
                let object = expect_object_target(target, "a token copy");
                let def = self.def_of(object);
                let count = self.resolve_count(count, controller, source, target, x);
                // Doubling Season (CR 614): the copies enter under `controller`.
                let count = self.token_count_after_replacements(controller, count);
                let mut events = Vec::new();
                for token in (self.next_object_id()..).take(count as usize) {
                    events.push(Event::TokenCreated {
                        token,
                        controller,
                        def,
                    creator: source,
                });
                    // Determined Iteration: "The token created this way gains haste."
                    if haste {
                        events.push(Event::TempBoost {
                            object: token,
                            power: 0,
                            toughness: 0,
                            keywords: HASTE,
                            source_name,
                        });
                    }
                    // Determined Iteration: "Sacrifice it at the beginning of the next end step"
                    // — schedule the delayed sacrifice against this specific minted token, not a
                    // re-scan (see `Effect::SacrificeObject`).
                    if sacrifice_at_next_end_step {
                        events.push(Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at: Step::End,
                            effect: Effect::SacrificeObject {
                                object: Some(token),
                            },
                        });
                    }
                    // Twinflame: "Exile those tokens at the beginning of the next end step" —
                    // schedule the delayed exile against this specific minted token, not a
                    // re-scan (mirrors `CreateToken`'s own `exile_at_next_end_step`).
                    if exile_at_next_end_step {
                        events.push(Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at: Step::End,
                            effect: Effect::ExileObject {
                                object: Some(token),
                            },
                        });
                    }
                }
                events
            }
            // Muddle, the Ever-Changing's magecraft ability: become a copy of the chosen target
            // until end of turn, except it has myriad — the copy overwrite mirrors
            // `Game::answer_enter_as_copy`'s `BecameCopy`, and the myriad grant reuses the same
            // "gains a keyword" `TempBoost` shape that answer's `gains_haste` rider uses.
            Effect::BecomeCopyOfTargetCreatureGainingMyriad { .. } => {
                let chosen = expect_object_target(
                    target,
                    "become-copy-of-target-creature-gaining-myriad",
                );
                let def = self.def_of(chosen);
                const MYRIAD: &[Keyword] = &[Keyword::Myriad];
                vec![
                    Event::BecameCopy {
                        object: source,
                        def,
                        until_eot: true,
                    },
                    Event::TempBoost {
                        object: source,
                        power: 0,
                        toughness: 0,
                        keywords: MYRIAD,
                        source_name,
                    },
                ]
            }
            // Myriad's payload (CR 702.114a): for each opponent other than the defending player,
            // mint a token copy of the attacker's current (possibly copied) characteristics that
            // enters tapped and attacking that opponent (`Event::Tapped`/`Event::TokenEnteredAttacking`,
            // never `AttackerDeclared` — CR 508.4, so a minted copy can't re-trigger myriad), then
            // schedule it to be exiled at the true end of combat.
            Effect::MyriadTokenCopies { attacking_context } => {
                let (attacker, defender) = attacking_context
                    .expect("filled in by Game::queue_myriad_triggers when the ability is synthesized");
                let def = self.def_of(source);
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for opponent in self.living_players() {
                    if opponent == attacker || opponent == defender {
                        continue;
                    }
                    // Doubling Season (CR 614): each copy is its own token creation.
                    let count = self.token_count_after_replacements(attacker, 1);
                    for _ in 0..count {
                        let token = next;
                        events.push(Event::TokenCreated {
                            token,
                            controller: attacker,
                            def,
                    creator: source,
                });
                        events.push(Event::Tapped { object: token });
                        events.push(Event::TokenEnteredAttacking {
                            token,
                            defender: opponent,
                        });
                        events.push(Event::DelayedTriggerScheduled {
                            controller: attacker,
                            source,
                            fire_at: Step::EndCombat,
                            effect: Effect::ExileObject { object: Some(token) },
                        });
                        next += 1;
                    }
                }
                events
            }
            // Redoubled Stormsinger: "for each creature token you control that entered this
            // turn, create a tapped and attacking token that's a copy of that token. At the
            // beginning of the next end step, sacrifice those tokens." No chosen target — scan
            // the attacker's own battlefield for the matching tokens (CR 508.4: each mint enters
            // tapped and attacking, never declared, so it can't re-trigger this ability).
            Effect::CopyEachEnteredThisTurnTokenTappedAttacking { attacking_context } => {
                let (attacker, defender) = attacking_context
                    .expect("filled in by contextualize_effect from the Attacks trigger context");
                let filter = PermanentFilter {
                    types: TypeSet::CREATURE,
                    token: TokenFilter::Token,
                    controller: FilterController::You,
                    entered_this_turn: true,
                    ..Default::default()
                };
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    if !self.permanent_matches(&filter, id, attacker, Some(source)) {
                        continue;
                    }
                    let def = self.def_of(id);
                    events.push(Event::TokenCreated {
                        token: next,
                        controller: attacker,
                        def,
                    creator: source,
                });
                    events.push(Event::Tapped { object: next });
                    events.push(Event::TokenEnteredAttacking {
                        token: next,
                        defender,
                    });
                    events.push(Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::SacrificeObject {
                            object: Some(next),
                        },
                    });
                    next += 1;
                }
                events
            }
        }
    }
}
