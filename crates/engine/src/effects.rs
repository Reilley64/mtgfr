//! Stack resolution payoffs — applying effects when spells and abilities resolve.
//!
//! Primary: CR 608 (resolving spells and abilities). Owns stack entry
//! (`resolve_top` / `resolve_spell` / enter / finish) and a **thin** [`Game::run`]
//! dispatcher; Effect bodies live in [`crate::resolution`] (mint families, pause
//! peels, resolve choreography). Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

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
            // Copy-family choreography — see `resolution/copy.rs::run_copy`.
            Effect::CopyTargetSpell
            | Effect::CopyThisSpell { .. }
            | Effect::RetargetSpellCopy { .. }
            | Effect::MayPayToCopyThis { .. }
            | Effect::ChangeTargetOfTargetSpellOrAbility { .. }
            | Effect::CopyTriggeringSpell { .. }
            | Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. }
            | Effect::CopyTriggeringAbility { .. } => self.run_copy(effect, ctx, events),
            // Opal Palace / Renegade Bull / Surge to Victory record-mana-value — see
            // `resolution/resolve_misc.rs`.
            Effect::CommanderEntersWithBonusCounters { .. }
            | Effect::ExileTargetGraveyardSpellCastFree { .. }
            | Effect::ExileTargetGraveyardCardRecordManaValue { .. } => {
                self.run_misc_choreo(effect, ctx, events)
            }
            // Surge to Victory's mint-free-copy step — see `resolution/copy.rs::run_copy`.
            Effect::MintFreeCopyOfExiledCard { .. } => self.run_copy(effect, ctx, events),
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
            // Feral Appetite — see `resolution/sequence_steps.rs::run_sequence_step`.
            Effect::ExileTargetGraveyardCardThenIfCreature { .. } => {
                self.run_sequence_step(effect, ctx, events)
            }
            // Marauding Raptor — see `resolution/damage.rs::resolve_deal_damage_to_entering`.
            Effect::DealDamageToEnteringPermanent { .. } => {
                self.resolve_deal_damage_to_entering(effect, ctx, events)
            }
            // Fabled Passage / Ajani's Chosen / Forum Filibuster / Fractal Harness / Scriv /
            // Animate Dead — see `resolution/sequence_steps.rs::run_sequence_step`.
            Effect::UntapSearchedLand
            | Effect::AttachTriggeringAuraToMintedToken { .. }
            | Effect::ReflexiveTrigger { .. }
            | Effect::ReturnFromGraveyardAttachedToToken { .. }
            | Effect::AttachSelfToReanimated
            | Effect::AttachSelfToMintedToken
            | Effect::AttachMintedAuraToTarget { .. } => self.run_sequence_step(effect, ctx, events),
            // Fractal Harness — see
            // `resolution/counters.rs::resolve_double_counters_on_attached_creature`.
            Effect::DoubleCountersOnAttachedCreature => {
                self.resolve_double_counters_on_attached_creature(ctx, events)
            }
            // Gift of Immortality / Cauldron Dance / Screams from Within / Ghoulish Impetus —
            // see `resolution/sequence_steps.rs::run_sequence_step`.
            Effect::ScheduleReturnThisAuraAttachedToReanimated
            | Effect::ScheduleReturnReanimatedToHand
            | Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost
            | Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost => {
                self.run_sequence_step(effect, ctx, events)
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
