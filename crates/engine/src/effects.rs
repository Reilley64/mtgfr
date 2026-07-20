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
                // it leaves the stack with no effect. This is where protection (CR 702.16b)
                // actually filters a targeted ability (activation itself isn't re-validated —
                // see `Game::activate_ability`'s own doc), sourced from the ability's own
                // permanent's colors (Nin, the Pain Artist, a UR source).
                // The target-legality `{X}` is the ability's source's own entered X (see
                // `Game::ability_source_x`) — needed for a `mv_max_x` re-check (Kinetic Ooze),
                // 0 for every other ability; distinct from the *activation* `{X}` below (Unbound
                // Flourishing's copied {X} ability, CR 107.3), which no pool `mv_max_x` reads.
                let legality_x = self.ability_source_x(source);
                // The source may already be `Object::Removed` (a Dies-trigger source token that
                // vanished — `def_of` would panic); no colors is the same "no protection filters"
                // posture this ability had before source colors were wired at all.
                let source_colors = match self.objects[source as usize] {
                    Object::Removed => [false; Color::COUNT],
                    _ => color_identity(self.def_of(source)),
                };
                if !self.target_still_legal(
                    effect.target(),
                    source,
                    target,
                    controller,
                    source_colors,
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
            // Animate Dead (CR 303.4a's "enchant creature card in a graveyard"): a real Aura, but
            // its cast-time target is a graveyard card, not an on-battlefield permanent — the
            // ordinary `CardKind::Aura` arm below assumes a live host to attach to immediately, so
            // this one instead runs the same generic permanent-enter path a Creature/Enchantment
            // does (it enters unattached; its own ETB ability's `reanimate_to_battlefield` +
            // `attach_self_to_reanimated` effects do the reanimate-then-attach, see
            // `CardDef::enchant_graveyard`'s doc). Every ordinary Aura keeps the immediate-attach
            // arm below.
            CardKind::Aura if spell.def.enchant_graveyard => {
                self.resolve_permanent_enter(spell, object, events);
            }
            CardKind::Creature { .. }
            | CardKind::Enchantment
            | CardKind::Artifact
            | CardKind::Planeswalker { .. } => {
                self.resolve_permanent_enter(spell, object, events);
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
                // (`Game::resume.spell_finish`), rather than moving it to its post-resolution
                // zone out from under its own still-open decision.
                if self.resolution_is_paused() {
                    self.resume.spell_finish = Some(object);
                    return;
                }
                self.finish_instant_sorcery_resolution(object, events);
            }
            CardKind::Land { .. } => {
                unreachable!("lands are played directly to the battlefield, never resolved")
            }
        }
    }

    /// Resolve a Creature/Enchantment/Artifact/Planeswalker (or Animate Dead's graveyard-target
    /// Aura, see `resolve_spell`'s `enchant_graveyard` arm) entering the battlefield: the
    /// devour/enter-as-copy pauses, `enters_with_counters`/Opal-Palace/escape counter placements,
    /// and Gorma's "creatures enter with an additional counter" static. Split out of
    /// [`Self::resolve_spell`] so a card that's `CardKind::Aura` but enters unattached (its own
    /// ETB ability does the attaching) can share this generic entry with the non-Aura kinds.
    fn resolve_permanent_enter(&mut self, spell: Spell, object: ObjectId, events: &mut Vec<Event>) {
        // Animate Dead (CR 303.4a/608.2b): its own cast-time "enchant creature card in a
        // graveyard" target can fizzle the same way an Aura's battlefield host can — an
        // opponent exiling the chosen graveyard card in response leaves it with no legal
        // object, so it goes to the graveyard (or ceases to exist, if it's a copy)
        // instead of entering unattached. The pool's only card with a cast-time graveyard
        // target, so this re-check is scoped to `enchant_graveyard` rather than folded
        // into the `CardKind::Aura` fizzle branch in `resolve_spell`.
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
            pending::raise(
                self,
                pending::ChoiceRequest::Devour {
                    player: spell.controller,
                    source: entered,
                    multiplier,
                },
            );
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
            pending::raise(
                self,
                pending::ChoiceRequest::EnterAsCopy {
                    player: spell.controller,
                    source: entered,
                    marker,
                },
            );
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
            let n =
                self.counters_after_replacements(entered, escape.plus_one_plus_one_counters as i32);
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

    /// Move a resolved instant/sorcery `object` to its post-resolution zone: ceases to exist if
    /// it's a copy (CR 707.10a), exile if it was cast via flashback/escape (CR 702.34e/702.19d),
    /// its owner's hand if it was bought back (CR 702.27d), the bottom of its owner's library if
    /// an [`Effect::TuckSelfToLibraryBottom`] step marked it (Spell Crumple), else the graveyard.
    /// Split out of
    /// [`Self::resolve_spell`] so [`Game::resume_deferred_sequence`] can also call it once a
    /// [`ResumeState::spell_finish`] pause clears.
    pub(crate) fn finish_instant_sorcery_resolution(
        &mut self,
        object: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let spell = *self.spell(object);
        // A copy ceases to exist (CR 707.10a); a cast instant/sorcery goes to the graveyard.
        if spell.copy {
            // A copy that ran a self-move rider (Spell Crumple's `TuckSelfToLibraryBottom`,
            // Rousing Refrain's `ExileSelfWithTimeCounters`) never reaches a library/exile — it
            // just ceases to exist. Discard those scratch marks here so they can't leak past this
            // resolution and redirect the *next* spell that finishes.
            self.self_tuck_to_library_bottom = false;
            self.self_exile_time_counters = None;
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
        // Spell Crumple's own "Then put Spell Crumple on the bottom of its owner's library"
        // rider: an `Effect::TuckSelfToLibraryBottom` step this resolution ran marked the spell
        // to tuck itself rather than reach the graveyard below — the self-referential sibling of
        // the buyback fork above.
        if std::mem::take(&mut self.self_tuck_to_library_bottom) {
            self.push_apply(
                events,
                Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top: false,
                    second_from_top: false,
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
                    second_from_top: false,
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

    /// Whether `object` is a still-on-stack spell that's a copy (CR 707.10a) — such a spell
    /// ceases to exist rather than going to any graveyard or library wherever it would
    /// otherwise land (countered here; resolving, via `finish_instant_sorcery_resolution`'s own
    /// check). `false` for a non-spell or an ordinary, non-copy spell.
    pub(crate) fn is_copy_object(&self, object: ObjectId) -> bool {
        matches!(self.objects[object as usize], Object::Spell(s) if s.copy)
    }

    /// Counter `spell` (CR 701.5a): move it from the stack to its owner's graveyard, so it never
    /// resolves. A no-op if `spell` already left the stack (CR 608.2b) — a response emptied that
    /// stack slot (countered/resolved) before this counter could act. Shared by the unconditional
    /// [`Effect::CounterTargetSpell`] arm and the [`PendingChoice::PayOrCounter`] decline handler.
    pub(crate) fn counter_spell(&self, spell: ObjectId) -> Vec<Event> {
        if !matches!(self.objects[spell as usize], Object::Spell(_)) {
            return Vec::new();
        }
        // CR 701.5g: "this spell can't be countered" — the counter fizzles and the spell
        // stays on the stack, unaffected.
        if self.def_of(spell).uncounterable {
            return Vec::new();
        }
        // CR 707.10a: a countered spell that's a copy ceases to exist rather than going to any
        // graveyard (mirrors `finish_instant_sorcery_resolution`'s own copy guard for the
        // resolving case) — checked first since it preempts every other "where does it go"
        // branch below (flashback/escape exile, Quintorius's tuck, the plain graveyard).
        if self.is_copy_object(spell) {
            return vec![Event::SpellCeasedToExist { spell }];
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
                second_from_top: false,
            }];
        }
        vec![Event::MovedToGraveyard {
            card: self.next_object_id(),
            from: spell,
        }]
    }

    /// The live current id of a "return this" ability's own source — or `None` if it has left
    /// every zone that effect's pool consumers actually fire from (CR 603.6e / 400.7: a
    /// return-this effect only acts on the object in the zone the ability expects; if it left
    /// that zone — exiled by Nezumi Graverobber mid-trigger, say — it's a new object the effect
    /// does not track). Shared by every `ReturnThis*` arm, each passing its own `allowed` list:
    /// [`Effect::ReturnThisFromGraveyardToBattlefield`]'s pool (Nether Traitor, Teacher's Pest)
    /// only ever fires with its source already a graveyard card, so `&[Zone::Graveyard]`;
    /// [`Effect::ReturnThisToHand`]'s pool fires from either a graveyard death trigger (Angelic
    /// Destiny, Squee-shaped graveyard triggers) or a battlefield activated ability (Flickering
    /// Ward), so `&[Zone::Graveyard, Zone::Battlefield]`. Folds in the pre-existing
    /// left-the-game guard (`Object::Removed`), since `zone_of` panics on it.
    pub(crate) fn return_this_source(
        &self,
        source: ObjectId,
        allowed: &[Zone],
    ) -> Option<ObjectId> {
        let current = self.current_id(source);
        if matches!(self.objects[current as usize], Object::Removed) {
            return None;
        }
        allowed.contains(&self.zone_of(current)).then_some(current)
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

    /// Resolve one effect — the sole call-site verb for Effect → board mutation (ADR 0004).
    /// A pausing effect sets `pending_choice` (via [`pending::raise`] / dig-loop helpers); every other effect
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
            targets_second: _,
            x,
            spent_mana: _,
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
            // Scry/surveil — ArrangeTop pause peel (`resolution/pause_arrange`).
            Effect::Scry { .. } | Effect::Surveil { .. } => {
                self.run_arrange_top(effect, controller, source, target, x)
            }
            // Clash (CR 701.22): pick an opponent, both reveal + scry-1 their top, score the clash.
            // Pauses on the shared opponent chooser and/or the two keep/bottom scries.
            Effect::Clash => self.begin_clash(controller, source, events),
            // LookAtTop / DistributeTop / SearchLibrary — look pause peel (`resolution/pause_look`).
            Effect::LookAtTop { .. }
            | Effect::DistributeTop { .. }
            | Effect::SearchLibrary { .. } => self.run_look_pause(effect, ctx),
            // Exile the top N face-up, pause on a choose-up-to-one over the matching cards to
            // grant free-cast permission, then bottom the rest (Herald of Amity's ETB dig).
            // Pauses on a ChooseExiledDigToCastFree choice.
            Effect::ExileTopCastMatchingFree { count, filter } => {
                self.exile_top_cast_matching_free(controller, source, count, filter, events)
            }
            // Songbirds' Blessing: reveal-until-Aura, pausing on a battlefield-or-hand choice
            // over the match.
            Effect::RevealUntilMayDeploy { filter } => {
                self.reveal_until_may_deploy(controller, filter, events)
            }
            // Creative Technique: reveal-until-nonland, pausing on the shared exiled-dig
            // may-cast-free choice over the match.
            Effect::RevealUntilExileCastFree { filter } => {
                self.reveal_until_exile_cast_free(controller, source, filter, events)
            }
            // ShuffleLibrary — see `resolution/resolve_misc.rs`.
            Effect::ShuffleLibrary => self.run_misc_choreo(effect, ctx, events),
            // Dance with Calamity: the player-driven exile-until-stop loop, then a free cast of any
            // number of the exiled cards if the tally stayed under budget. Pauses on a
            // DanceExileMore choice.
            Effect::ExileTopUntilStopCastFreeUnderBudget { budget } => {
                self.dance_with_calamity(controller, source, budget, events)
            }
            // Cascade (CR 702.85): reveal-until a cheaper nonland, may cast it free, bottom the
            // rest in random order. Pauses on a ChooseExiledDigToCastFree choice (reused from the
            // dig) when a hit is found.
            Effect::Cascade { mana_value } => self.cascade(controller, source, mana_value, events),
            // Edict / fan-out pauses — edict pause peel (`resolution/pause_edict`).
            Effect::EachPlayerSacrifices { .. }
            | Effect::EachPlayerExilesFromGraveyard
            | Effect::TargetPlayerExilesFromGraveyard { .. }
            | Effect::CasterKeepsOneOfEachTypePerPlayer
            | Effect::EachPlayerControllerChoosesCounterTarget
            | Effect::CouncilsDilemmaVote { .. }
            | Effect::EachOtherTokenBecomesCopyOfChosen
            | Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. }
            | Effect::SacrificeOwn { .. }
            | Effect::DefendingPlayerSacrifices { .. }
            | Effect::SacrificeSelfUnlessReturnLand { .. } => {
                self.run_edict_pause(effect, ctx, events)
            }
            // Abstract Performance: split the top eight into two piles, an opponent picks one,
            // pausing on an OpponentChoosesPile choice.
            Effect::OpponentSplitsExilePiles => {
                self.opponent_splits_exile_piles(controller, source, events)
            }
            // Fact or Fiction: reveal the top five, an opponent splits them into two piles,
            // pausing on a PartitionRevealed choice.
            Effect::RevealTopSplitPiles => self.reveal_top_split_piles(controller, source, events),
            // Murmurs from Beyond: reveal the top `count`, an opponent picks one to graveyard,
            // the rest to hand — pausing on an OpponentChoosesRevealedToGraveyard choice.
            Effect::RevealTopOpponentPicksOneToGraveyard { count } => {
                self.reveal_top_opponent_picks_one_to_graveyard(controller, source, count, events)
            }
            // Plargg and Nassari: each player exiles from the top until a nonland, an opponent
            // picks one, pausing on an OpponentChoosesExiledNonland choice.
            Effect::EachPlayerExilesUntilNonlandOpponentPicks => {
                self.each_player_exiles_until_nonland(controller, source, events)
            }
            // MaySacrifice / MayReturnFromGraveyard / MayDiscard / MayDraw* /
            // SacrificeSelfUnlessPay — may pause peel (`resolution/pause_may`).
            Effect::MaySacrifice { .. }
            | Effect::MayReturnFromGraveyard { .. }
            | Effect::MayDiscard { .. }
            | Effect::MayDrawUnlessPays { .. }
            | Effect::TargetPlayerMayDraw { .. }
            | Effect::MayDrawUpTo { .. }
            | Effect::MayDrawUpToThenOpponentMayRepeat { .. }
            | Effect::SacrificeSelfUnlessPay { .. } => self.run_may_pause(effect, ctx),
            // ChooseCreatureType / ChooseColor / SetOwnColorUntilEndOfTurn / ChooseOne /
            // Demonstrate / Proliferate / PhaseOut — choose pause peel (`resolution/pause_choose`).
            Effect::ChooseCreatureType
            | Effect::ChooseColor
            | Effect::SetOwnColorUntilEndOfTurn
            | Effect::ChooseOne { .. }
            | Effect::Demonstrate { .. }
            | Effect::Proliferate { .. }
            | Effect::PhaseOut => self.run_choose_pause(effect, ctx),
            // Kinetic Ooze — see `resolution/counters.rs::resolve_double_counters_on_target_creatures`.
            Effect::DoubleCountersOnTargetCreatures { .. } => {
                self.resolve_double_counters_on_target_creatures(ctx, events)
            }
            // Donation — see `resolution/control.rs::resolve_target_opponent_gains_control`.
            Effect::TargetOpponentGainsControl { .. } => {
                self.resolve_target_opponent_gains_control(ctx, events)
            }
            // Exchange control — see `resolution/control.rs::resolve_exchange_control`.
            Effect::ExchangeControl { .. } => self.resolve_exchange_control(ctx, events),
            // Perpetual Timepiece / Quandrix Command mode 3 — exile-cast pause peel
            // (`resolution/pause_exile_cast`).
            Effect::ShuffleTargetCardsFromGraveyardIntoLibrary { .. } => {
                self.run_exile_cast_pause(effect, ctx)
            }
            // Chaos Warp — see `resolution/zones.rs::resolve_shuffle_then_reveal`.
            Effect::ShuffleTargetPermanentIntoLibraryThenReveal { .. } => {
                self.resolve_shuffle_then_reveal(target, events)
            }
            // CounterTargetSpell unless-pays / destination — counter-spell peel
            // (`resolution/pause_counter_spell`).
            Effect::CounterTargetSpell {
                unless_pays: Some(_),
                ..
            }
            | Effect::CounterTargetSpell {
                unless_pays: None,
                countered_dest: Some(_),
                ..
            } => self.run_counter_spell(effect, ctx, events),
            // Fight / MoveCounters — fight pause peel (`resolution/pause_fight`).
            Effect::Fight { .. } | Effect::MoveCounters { .. } => self.run_fight_pause(effect, ctx),
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
            Effect::MayPayToCopyThis { cost, count } => {
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
                        effect: Effect::CopyThisSpell {
                            count,
                            cast_from_graveyard_only: false,
                            optional: false,
                        },
                    },
                );
            }
            // Willbender (CR 114.6 / 702.37f) / Wild Ricochet (CR 114.6a). The bent spell is this
            // ability's own chosen target (CR 603.3d for Willbender's trigger; the cast target for
            // Wild Ricochet), already re-checked legal by CR 608.2b before this ran (so a spell that
            // left the stack fizzles). Guard the shape defensively.
            Effect::ChangeTargetOfTargetSpellOrAbility { optional, .. } => {
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
            Effect::CopyTriggeringSpell {
                triggering_spell,
                count,
                may_choose_new_targets,
                last_known_information,
            } => {
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
                may_choose_new_targets,
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
                self.resolution_frame.surge_exiled_card = Some((exiled, mana_value));
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
            // Discard / PutFromHand* / CastCreatureFaceDown — hand pause peel (`resolution/pause_hand`).
            Effect::Discard { .. }
            | Effect::PutFromHandOnTop { .. }
            | Effect::PutLandFromHand { .. }
            | Effect::PutCreatureFromHand
            | Effect::CastCreatureFaceDown => self.run_hand_pause(effect, ctx),
            // CashOutExiledWithThis / CastExiledWithThisFree — exile-cast pause peel
            // (`resolution/pause_exile_cast`).
            Effect::CashOutExiledWithThis | Effect::CastExiledWithThisFree => {
                self.run_exile_cast_pause(effect, ctx)
            }
            // A sequence runs its steps in order, sharing this target/{X}; a pausing step defers
            // the rest until answered.
            Effect::Sequence { steps } => self.run_sequence(steps, ctx, events),
            // A per-step gate: run `then` only if `condition` holds (negated by `negate`) right
            // now (mid-resolution), sharing this target/{X}. Reuses the same intervening-if
            // evaluator triggers use, except `TargetPowerAtLeast` (Yavimaya Bloomsage's power-7
            // check), `SourceEnteredWithXAtLeast` (Kinetic Ooze's X-threshold riders),
            // `ColorWasSpentToCastThis` (Court Hussar's "unless {W} was spent to cast it"), and
            // `SourceUntapped` (Howling Mine's CR 603.4 *second* check): `TriggerContext` carries
            // neither a target nor a source id, so those are special-cased directly against the
            // shared `target`/this resolution's own `source` here — the same "condition_holds
            // can't reach it" shape as `ability_condition_holds`'s source-based special cases.
            Effect::Conditional {
                condition,
                then,
                negate,
            } => {
                let holds = match condition {
                    Condition::TargetPowerAtLeast { at_least } => target
                        .and_then(Target::object_id)
                        .is_some_and(|object| self.power(object) >= at_least as i32),
                    // Nezumi Graverobber: "Then if there are no cards in that player's graveyard"
                    // — the just-exiled target's owner (the moved card object still records it) has
                    // an empty graveyard now. No legal target exiled (the flip clause is a no-op):
                    // `is_some_and` is false, so it doesn't flip.
                    Condition::TargetCardOwnerGraveyardEmpty => {
                        target.and_then(Target::object_id).is_some_and(|object| {
                            self.graveyard_cards(self.owner_of(object)).is_empty()
                        })
                    }
                    Condition::SourceEnteredWithXAtLeast { at_least } => {
                        self.ability_source_x(source) >= at_least
                    }
                    Condition::ColorWasSpentToCastThis { color } => self
                        .as_permanent(source)
                        .is_some_and(|p| p.spent_colors[color.index()]),
                    // Howling Mine's CR 603.4 *second* check: re-read the source's own tapped
                    // state fresh at resolution, not the placement-time snapshot — the ability may
                    // have triggered while untapped but had this intervening-if falsified by a
                    // response that taps it before it resolves (Magma Opus's "tap two target
                    // permanents"). Source-object-based like `SourceEnteredWithXAtLeast` above.
                    Condition::SourceUntapped => {
                        self.as_permanent(source).is_some_and(|p| !p.tapped)
                    }
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
            // Marauding Raptor — see `resolution/damage.rs::resolve_deal_damage_to_entering`.
            Effect::DealDamageToEnteringPermanent { .. } => {
                self.resolve_deal_damage_to_entering(effect, ctx, events)
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
            // Fractal Harness — see
            // `resolution/counters.rs::resolve_double_counters_on_attached_creature`.
            Effect::DoubleCountersOnAttachedCreature => {
                self.resolve_double_counters_on_attached_creature(ctx, events)
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
            // Cauldron Dance: grant haste to the creature this same resolution's preceding
            // `ReanimateToBattlefield` step just put onto the battlefield — read back from
            // `events`, mirroring `ScheduleReturnThisAuraAttachedToReanimated`'s idiom above —
            // then schedule its return to hand at the next end step (CR 603.7). No such event
            // yet (the reanimation target was illegal): nothing to grant or schedule
            // (guard-return).
            Effect::ScheduleReturnReanimatedToHand => {
                let Some(permanent) = events.iter().rev().find_map(|e| match e {
                    Event::ReanimatedToBattlefield { permanent, .. } => Some(*permanent),
                    _ => None,
                }) else {
                    return;
                };
                const HASTE: &[Keyword] = &[Keyword::Haste];
                let source_name = self.source_name_of(source);
                self.push_apply(
                    events,
                    Event::TempBoost {
                        object: permanent,
                        power: 0,
                        toughness: 0,
                        keywords: HASTE,
                        source_name,
                    },
                );
                self.push_apply(
                    events,
                    Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at: Step::End,
                        effect: Effect::ReturnObjectToHand {
                            object: Some(permanent),
                        },
                    },
                );
            }
            // Screams from Within: the immediate dies-return, choosing a new host (unlike Gift
            // of Immortality's same-creature return above). Pauses via the shared helper — see
            // its doc comment.
            Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.return_aura_from_graveyard_attached_to_chosen_host(source, events)
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
            // Destroy / exile / mill "this way" snapshot choreography lives in the family modules
            // (`resolve_destroy_all` / `resolve_exile_all` / `resolve_mill_self`).
            Effect::DestroyAll { .. } => {
                self.resolve_destroy_all(effect, controller, source, target, x, events)
            }
            Effect::ExileAll { .. } => {
                self.resolve_exile_all(effect, controller, source, target, x, events)
            }
            Effect::MillSelf { count } => {
                self.resolve_mill_self(count, controller, source, target, x, events)
            }
            // Rousing Refrain / Spell Crumple self-move riders — see
            // `resolution/resolve_misc.rs`.
            Effect::ExileSelfWithTimeCounters { .. } | Effect::TuckSelfToLibraryBottom => {
                self.run_misc_choreo(effect, ctx, events)
            }
            // Oversimplify — see `resolution/resolve_misc.rs`.
            Effect::EachPlayerCreatesFractalFromExiledPower { .. } => {
                self.run_misc_choreo(effect, ctx, events)
            }
            // Wheel of Fortune — see `resolution/resolve_misc.rs`.
            Effect::EachPlayerDiscardsHandThenDraws { .. } => {
                self.run_misc_choreo(effect, ctx, events)
            }
            // `CreateToken`'s `enters_with` choreography — see
            // `resolution/tokens.rs::resolve_create_token`.
            Effect::CreateToken { .. } => self.resolve_create_token(effect, ctx, events),
            // Advanced Reconstruction — see `resolution/resolve_misc.rs`.
            Effect::ExileRandomFromGraveyardMayPlay => self.run_misc_choreo(effect, ctx, events),
            // Ruhan of the Fomori — see `resolution/resolve_misc.rs`.
            Effect::MustAttackRandomOpponent => self.run_misc_choreo(effect, ctx, events),
            // Inkshield / Moment's Peace — see `resolution/resolve_misc.rs`.
            Effect::PreventCombatDamageToYouCreatingTokens { .. }
            | Effect::PreventAllCombatDamageThisTurn => self.run_misc_choreo(effect, ctx, events),
            // Each of these draws may be replaced by dredge (CR 702.52): `draw_with_dredge` draws one
            // card at a time, pausing on `ChooseDredge` before any draw the controller has an eligible
            // dredger for (accepting mills + returns, declining draws). `answer_choose_dredge` re-enters
            // it for the remaining draws and resumes the deferred sequence once the batch is done.
            Effect::DrawCards { count } => {
                let n = self.resolve_count(count, controller, source, target, x);
                self.draw_with_dredge(controller, n, false, events);
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
}
