//! Triggered abilities: enqueue, intervening-if, APNAP placement, look-back-in-time.
//!
//! Primary: CR 603 (triggered abilities). Also: CR 603.6c/603.10 look-back, CR 603.3c no
//! legal target, CR 603.7 delayed triggers. Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// The outcome of placing a resolved ability via [`Game::place_targeted_ability`].
pub(crate) enum Placement {
    /// Paused on a [`PendingChoice::ChooseTarget`] for the ability's target.
    Paused,
    /// Pushed straight onto the stack (the ability is targetless).
    Placed,
    /// Dropped: the ability targets but has no legal target (CR 603.3c).
    NoLegalTarget,
}

impl Game {
    /// Scan just-produced events and queue the triggered abilities they fire. Two flavors:
    /// *self-referential* triggers fire on the event's own subject (this permanent entered /
    /// attacked / died); *controller-scoped* triggers fire on every permanent the relevant
    /// player controls (their upkeep/end step began, they gained life, they cast a spell).
    pub(crate) fn enqueue_triggers(&mut self, events: &[Event]) {
        // CR 603.6c/603.10.1 "look back in time": a watch-others death trigger reads the game
        // state just *before* the deaths, so a watcher that dies in the same batch (a board wipe,
        // a combat sweep) still sees the *other* deaths in that batch. The dying watchers are
        // already off the battlefield by now, so snapshot them from the events up front.
        let batch_deaths = self.batch_creature_deaths(events);
        self.queue_enchantment_death_watchers(events);
        for (idx, event) in events.iter().enumerate() {
            match *event {
                // Self-referential: the source is the object the event is about. A land enters via
                // its own `LandPlayed` event (not `PermanentEntered`), so an ETB on a land — a
                // scry land / Temple — fires from here too.
                Event::PermanentEntered { permanent, .. }
                | Event::ReanimatedToBattlefield { permanent, .. }
                | Event::ReturnedFromLinkedExile { permanent, .. }
                | Event::FlickeredToBattlefield { permanent, .. }
                | Event::SearchedToBattlefield { permanent, .. }
                | Event::PutOntoBattlefieldFromHand { permanent, .. }
                // A manifest enters the battlefield as a creature (CR 701.34); its own `Etb` scans
                // its (empty while face-down) `functional_abilities`, but other permanents'
                // "whenever a creature enters" watches see it.
                | Event::Manifested { permanent, .. }
                | Event::LandPlayed { permanent, .. } => {
                    // Evoke (CR 702.74a): queued *before* the permanent's own `Etb` trigger below
                    // so it lands underneath it on the stack and so resolves *after* — an ETB
                    // payoff (Mulldrifter's draw two) still happens before the sacrifice.
                    // ponytail: no ordering choice is raised for the controller's own simultaneous
                    // triggers (CR 603.3b) — the two are queued as separate single-ability groups
                    // rather than one multi-ability group, so `place_pending_triggers` places both
                    // without pausing. Grow into a real `OrderTriggers` choice if an evoke card
                    // ever needs the controller to choose the other order.
                    if self.as_permanent(permanent).is_some_and(|p| p.evoked) {
                        self.queue_evoke_sacrifice(permanent);
                    }
                    self.queue_self_trigger(permanent, Trigger::Etb);
                    // ponytail: watch-others companion to the self `Etb` above — constellation/
                    //   landfall watch *any other* permanent's entry, not their own.
                    self.queue_permanent_enters_triggers(permanent);
                }
                // Turned face up (CR 702.37f): scan the now-revealed permanent's own abilities for
                // a turned-face-up trigger. The flag is already cleared (the apply ran first), so
                // its real abilities are visible — the same self-scan idiom as the `Etb` above,
                // but not entering the battlefield, so no watch-others enters triggers.
                Event::TurnedFaceUp { permanent } => {
                    self.queue_self_trigger(permanent, Trigger::TurnedFaceUp);
                }
                Event::TokenCreated {
                    token,
                    controller,
                    def,
                } => {
                    self.queue_self_trigger(token, Trigger::Etb);
                    // A created token is a permanent entering the battlefield (CR 603.6a) too.
                    self.queue_permanent_enters_triggers(token);
                    // Staff of the Storyteller's "whenever you create one or more creature
                    // tokens" (CR 603.3b): record the controller now, deduped and fired once per
                    // batch below — mirrors `graveyard_exits_this_batch`.
                    // ponytail: "you" scope only (`Trigger::YouCreateToken` is fieldless) — no
                    //   pool card needs an opponent/any-player watch yet.
                    if matches!(def.kind, CardKind::Creature { .. }) {
                        self.batch_trigger_scratch
                            .creature_tokens_created_this_batch
                            .push(controller);
                    }
                }
                Event::AttackerDeclared { object, defender } => {
                    // Self-referential "whenever this creature attacks" carries the attack
                    // context too (Goblin Guide's `RevealTopToHand.defender`) — every other
                    // consumer's effect ignores `ctx.attack`, so this is a no-op for them.
                    let ctx = TriggerContext {
                        attack: Some((self.controller_of(object), defender)),
                        // CR 510.2/603.10a: "where X is this creature's power" reads the attacker's
                        // power the instant the trigger goes on the stack (Guardian Scalelord).
                        source_power: Some(self.power(object)),
                        ..TriggerContext::of(self.owner_of(object))
                    };
                    self.queue_trigger_group(ctx, object, self.def_of(object), Trigger::Attacks);
                    self.queue_watch_attack_triggers(object, defender);
                    self.queue_enchanted_creature_attacks_triggers(object, defender);
                    self.queue_myriad_triggers(object, defender);
                }
                // A creature dying (battlefield → graveyard) fires its own Dies trigger. (CR 603.6, CR 403.5, CR 603)
                // ponytail: keyed off `MovedToGraveyard`, which carries no source zone, so a (CR 400)
                //   creature *discarded* from hand would also match — guarded to creatures, and
                // no pool card has both a Dies trigger and is a discard target. Commanders (CR 603.6, CR 601.2c, CR 603)
                //   divert to the command zone (a different event) and don't fire Dies yet.
                Event::MovedToGraveyard { from, .. } => {
                    // If the dying creature's owner left the game in this same SBA sweep, the (CR 704, CR 108.4)
                    // creature left with them (CR 800.4a) and its arena slot is already Removed —
                    // guard before reading it, since its Dies trigger doesn't fire. (CR 603.6, CR 603)
                    // ponytail: this also suppresses other players' death-watch (Blood Artist) for
                    //   that one simultaneous death; no pool card is documented to care — revisit if
                    // a "whenever a creature dies" card must see a death coincident with a loss. (CR 704, CR 603.6, CR 108.4)
                    if matches!(
                        self.objects[self.current_id(from) as usize],
                        Object::Removed
                    ) {
                        continue;
                    }
                    let def = self.def_of(from);
                    // Self-referential "put into a graveyard from the battlefield" (CR): any
                    // permanent kind, not `Dies`'s creature-only guard below — Fallen Ideal.
                    if self
                        .batch_trigger_scratch
                        .permanents_put_into_graveyard_from_battlefield
                        .contains(&from)
                    {
                        self.queue_trigger_group(
                            TriggerContext::of(self.owner_of(from)),
                            from,
                            def,
                            Trigger::ThisAuraLeaves,
                        );
                    }
                    // Leaves-to-ANY-zone self-trigger (Animate Dead): unlike `ThisAuraLeaves`
                    // above, this also fires off `MovedToExile`/`ReturnedToHand`/
                    // `TuckedToLibrary` below, and its payoff acts on the captured host, not self.
                    self.queue_leaves_battlefield_triggers(from, self.owner_of(from), def);
                    if matches!(def.kind, CardKind::Creature { .. }) {
                        let ctx = TriggerContext {
                            dying_source_stats: self.dying_source_stats(from),
                            ..TriggerContext::of(self.owner_of(from))
                        };
                        self.queue_trigger_group(ctx, from, def, Trigger::Dies);
                        self.queue_watch_death_triggers(
                            self.owner_of(from),
                            from,
                            def,
                            &batch_deaths,
                            false, // a nontoken permanent (CR: "put into a graveyard from the battlefield")
                        );
                        self.queue_enchanted_creature_dies_triggers(from);
                        self.queue_an_enchanted_creature_dies_triggers(from, &batch_deaths);
                    }
                }
                // A dying token fires its Dies trigger before vanishing; its arena slot is (CR 603.6, CR 111, CR 603)
                // already gone, so its controller/def come off the event.
                Event::TokenCeasedToExist {
                    token,
                    controller,
                    def,
                } => {
                    // A token only ever exists as a battlefield permanent, so — unlike the
                    // `MovedToGraveyard` arm above — no scratch guard is needed: every
                    // `TokenCeasedToExist` is CR's "put into a graveyard from the battlefield".
                    self.queue_trigger_group(
                        TriggerContext::of(controller),
                        token,
                        def,
                        Trigger::ThisAuraLeaves,
                    );
                    // Leaves-to-ANY-zone self-trigger (Animate Dead) — see the `MovedToGraveyard`
                    // arm above. `def`/`controller` are passed explicitly since the token's arena
                    // slot is already `Object::Removed`.
                    self.queue_leaves_battlefield_triggers(token, controller, def);
                    // Hofri Ghostforge's minted Spirit token's granted return rider — only a
                    // token can carry this link (the printed def never does).
                    self.queue_token_return_exiled_trigger(token, controller);
                    if matches!(def.kind, CardKind::Creature { .. }) {
                        let ctx = TriggerContext {
                            dying_source_stats: self.dying_source_stats(token),
                            ..TriggerContext::of(controller)
                        };
                        self.queue_trigger_group(ctx, token, def, Trigger::Dies);
                        self.queue_watch_death_triggers(
                            controller,
                            token,
                            def,
                            &batch_deaths,
                            true, // a token ceasing to exist never satisfies a "nontoken" watch
                        );
                        self.queue_enchanted_creature_dies_triggers(token);
                        self.queue_an_enchanted_creature_dies_triggers(token, &batch_deaths);
                    }
                }
                // Leaves-to-ANY-zone self-trigger (Animate Dead) — the exile/bounce/tuck
                // destination twin of the `MovedToGraveyard`/`TokenCeasedToExist` arms above.
                Event::MovedToExile { from, .. }
                | Event::ReturnedToHand { from, .. }
                | Event::TuckedToLibrary { from, .. } => {
                    // Same guard as the `MovedToGraveyard` arm above: if `from`'s owner left the
                    // game in this same SBA sweep (CR 800.4a), the moved card left with them and
                    // its post-move object is already `Object::Removed` — `def_of`/`owner_of`
                    // would panic reading it.
                    if matches!(
                        self.objects[self.current_id(from) as usize],
                        Object::Removed
                    ) {
                        continue;
                    }
                    self.queue_leaves_battlefield_triggers(from, self.owner_of(from), self.def_of(from));
                }
                // Controller-scoped: fire on every permanent the player controls.
                Event::StepBegan {
                    step: Step::Upkeep,
                    active_player,
                } => {
                    self.queue_controller_triggers(active_player, Trigger::Upkeep, None);
                    // CR "at the beginning of your upkeep" fires the *controller's* graveyard-
                    // functional upkeep triggers too (Squee), scoped to the active player's upkeep.
                    self.queue_graveyard_controller_triggers(active_player, Trigger::Upkeep);
                    self.queue_each_upkeep_triggers();
                    self.queue_echo_triggers(active_player);
                }
                // Other-player-only: fire on every battlefield permanent whose controller is
                // NOT the player whose untap step this is (Drumbellower). The untap step itself
                // grants no priority (CR 502), so — like the every-player flavors above — this
                // queues here and the resulting ability rides the normal APNAP placement path,
                // going on the stack at the next priority window (the following upkeep) rather (CR 117, CR 405, CR 503)
                // than resolving mid-untap-sweep.
                // ponytail: the ability resolves at the next priority window, not synchronously (CR 117, CR 113)
                //   inside the untap sweep — behaviorally identical for Drumbellower (untapping
                //   an already-untapped creature is a no-op; the untap step's own turn-based
                //   action only untaps the *active* player's permanents, never the ones
                //   Drumbellower's controller cares about).
                Event::StepBegan {
                    step: Step::Untap,
                    active_player,
                } => self.queue_each_other_player_untap_step_triggers(active_player),
                Event::StepBegan {
                    step: Step::BeginCombat,
                    active_player,
                } => self.queue_controller_triggers(active_player, Trigger::BeginCombat, None),
                Event::StepBegan {
                    step: Step::End,
                    active_player,
                } => {
                    self.queue_controller_triggers(active_player, Trigger::EndStep, None);
                    self.queue_each_end_step_triggers();
                }
                // Only a *gain* of life (positive change) triggers. Excluding the life-gain's
                // own source stops a "whenever you gain life, gain life" ability looping on its
                // own resolution (CR would loop forever; no real card does this).
                // ponytail: single fire per life-gain event, self-source excluded — revisit if a
                //   card legitimately wants to re-trigger off life it gained itself.
                Event::LifeChanged {
                    player,
                    amount,
                    source,
                } if amount > 0 => {
                    self.queue_controller_triggers(player, Trigger::YouGainLife, source)
                }
                // "Whenever you lose life for the first time each turn" (Intermediate
                // Chirography's level 2). A decrease bumped `life_losses_this_turn` at apply; this
                // loss's own ordinal is that post-batch tally minus the same-player losses trailing
                // it in this batch (mirrors the `CardDrawn` ordinal recovery below), so the trigger
                // fires only when this is the turn's *first* loss. CR 118.9/119.3.
                Event::LifeChanged { player, amount, .. } if amount < 0 => {
                    let trailing = events[idx + 1..]
                        .iter()
                        .filter(|e| matches!(e, Event::LifeChanged { player: p, amount: a, .. } if *p == player && *a < 0))
                        .count() as u32;
                    let nth = self.players[player.0 as usize].life_losses_this_turn - trailing;
                    if nth == 1 {
                        self.queue_controller_triggers(
                            player,
                            Trigger::YouLoseLifeFirstTimeEachTurn,
                            None,
                        );
                    }
                }
                // Magecraft: casting an instant/sorcery. (The copy half is `SpellCopied`, below.)
                // Also the general-purpose `CastSpell` watch (Monologue Tax's "an opponent casts
                // their second spell", Killian's "targets a creature", …) — both fire off the
                // same event, independently of each other.
                Event::SpellCast {
                    spell,
                    controller,
                    target,
                    x,
                    ..
                } => {
                    let def = self.def_of(spell);
                    if matches!(def.kind, CardKind::Spell { .. }) {
                        self.queue_magecraft_triggers(controller, def.mana_value());
                    }
                    let cast_from_hand = self.spell(spell).cast_from_hand;
                    // The cast's payment rode a `ManaSpent` earlier in this same batch (CR
                    // 601.2h) — `None` for a free/alt cast that spent no mana (Manaform
                    // Hellkite's `Amount::TriggeringSpellManaSpent` reads `Some(0)` in that case,
                    // via the `unwrap_or(0)` below).
                    let mana_spent = events.iter().find_map(|e| match *e {
                        Event::ManaSpent { player, mana } if player == controller => Some(mana),
                        _ => None,
                    });
                    self.queue_cast_spell_triggers(
                        controller,
                        spell,
                        def,
                        target,
                        x,
                        cast_from_hand,
                        mana_spent.map_or(0, |mana| mana.total()),
                    );
                    // "When you spend this mana to cast …" (Study Hall / Path of Ancestry / Opal
                    // Palace): fire the producing land's `SpendManaToCast` if its tagged mana
                    // funded it.
                    if let Some(spend) = mana_spent {
                        self.queue_spend_to_cast_triggers(controller, spell, spend);
                    }
                    self.queue_becomes_targeted_triggers(target);
                    self.queue_spell_targets_this_only_triggers(target, controller, spell, def);
                    self.queue_prowess_triggers(controller, def, target);
                    // "When you cast this spell" (CR 601.2i/603.3): scanned off the cast card's
                    // own def, not a battlefield watcher — the source is the spell object itself,
                    // so the resulting ability is a separate stack object (Hydroid Krasis's cast
                    // trigger resolves even if the spell is later countered, CR 702.137a).
                    self.queue_trigger_group(
                        TriggerContext {
                            cast_x: Some(x),
                            ..TriggerContext::of(controller)
                        },
                        spell,
                        def,
                        Trigger::YouCastThis,
                    );
                    // Cascade (CR 702.85e): a rules-keyword "when you cast this spell" trigger, not
                    // a printed `[[abilities]]`, so it's fabricated here (like a delayed trigger)
                    // and rides the normal APNAP placement path onto the stack above the cascading
                    // spell. `mana_value` is baked in now as last-known information (CR 702.85b).
                    if def.cascade {
                        self.pending_trigger_groups.push(TriggerGroup {
                            expanded: false,
                            controller,
                            source: spell,
                            // ponytail: `timing`/`optional`/`cost`/`condition` are inert
                            // placeholders — `place_pending_triggers` only reads `effect`, exactly
                            // as `fire_delayed_triggers` fabricates its groups.
                            abilities: vec![Ability {
                                timing: Timing::Triggered(Trigger::YouCastThis),
                                effect: Effect::Cascade {
                                    mana_value: def.mana_value(),
                                },
                                optional: false,
                                min_level: 0,
                                cost: Cost::FREE,
                                condition: None,
                                once_each_turn: false,
                            }],
                        });
                    }
                    // Demonstrate (CR 702.147a): another rules-keyword "when you cast this spell"
                    // trigger, fabricated the same way as cascade above — `spell` is baked in now
                    // as this cast's own object id.
                    if def.demonstrate {
                        self.pending_trigger_groups.push(TriggerGroup {
                            expanded: false,
                            controller,
                            source: spell,
                            abilities: vec![Ability {
                                timing: Timing::Triggered(Trigger::YouCastThis),
                                effect: Effect::Demonstrate { spell },
                                optional: false,
                                min_level: 0,
                                cost: Cost::FREE,
                                condition: None,
                                once_each_turn: false,
                            }],
                        });
                    }
                }
                // Faerie Mastermind's "an opponent draws their second card each turn". Every
                // `CardDrawn` in this batch is already applied (so `draws_this_turn` holds the
                // *final* post-batch tally), but a single "draw two" produces two `CardDrawn`
                // events in one batch — reading the final tally for both would misfire (both see
                // 2). So recover THIS draw's own ordinal: subtract the same-player draws that come
                // after it in the batch (each of those bumped the tally by one).
                Event::CardDrawn { player, .. } => {
                    let trailing = events[idx + 1..]
                        .iter()
                        .filter(|e| matches!(e, Event::CardDrawn { player: p, .. } if *p == player))
                        .count() as u32;
                    let nth = self.players[player.0 as usize].draws_this_turn - trailing;
                    self.queue_player_draws_triggers(player, nth);
                }
                // Magecraft's "or copy" half: copying an instant/sorcery fires the copier's
                // magecraft triggers (a copy is always an instant/sorcery spell).
                Event::SpellCopied {
                    controller, copy, ..
                } => {
                    self.queue_magecraft_triggers(controller, self.def_of(copy).mana_value());
                }
                // A sacrifice (CR 701.20) — distinct from `MovedToGraveyard`/`TokenCeasedToExist`,
                // which also fire for a plain destroy: `YouSacrifice`/`AnyPlayerSacrifices` watch
                // specifically for this marker.
                Event::Sacrificed { object, by, def } => {
                    self.queue_sacrifice_triggers(object, by, def)
                }
                // A discard (CR 701.8) — distinct from `MovedToGraveyard`, which also fires for a
                // sacrifice/destroy: `YouDiscard` watches specifically for this marker.
                Event::Discarded { card, player, .. } => self.queue_discard_triggers(player, card),
                // Combat damage to a player (CR 510.2) — never a non-combat life loss, which
                // only emits `LifeChanged`, and never combat damage to a *creature*, which (CR 510, CR 120.3, CR 506)
                // emits `DamageMarked` instead.
                Event::CombatDamageDealtToPlayer { source, amount, .. } => {
                    self.queue_combat_damage_triggers(source, amount)
                }
                _ => {}
            }
        }
        // CR "one or more cards leave your graveyard" (Quintorius Field Historian / Lorehold): the
        // whole batch, not each card, is the trigger event — a board-wipe-then-reanimate or a
        // multi-card exile fires each affected player's watcher once. `create_object` recorded every
        // owner who lost a graveyard card this batch; drain (dedup, then clear) and fire once each.
        if !self
            .batch_trigger_scratch
            .graveyard_exits_this_batch
            .is_empty()
        {
            let exits = std::mem::take(&mut self.batch_trigger_scratch.graveyard_exits_this_batch);
            let mut owners: Vec<PlayerId> = exits.iter().map(|(p, _)| *p).collect();
            owners.sort_unstable_by_key(|p| p.0);
            owners.dedup();
            for owner in owners {
                // Each owner's own graveyard-object ids that left (CR 603.10a last-known info),
                // threaded into the trigger's context for Spirit of Resilience's copy payoff.
                // ponytail: leaked per fire so `TriggerContext` stays `Copy`; move to a runtime
                // carrier if a long game's repeated fires make the leak matter.
                let cards: Vec<ObjectId> = exits
                    .iter()
                    .filter(|(p, _)| *p == owner)
                    .map(|(_, id)| *id)
                    .collect();
                let cards: &'static [ObjectId] = Box::leak(cards.into_boxed_slice());
                self.queue_cards_leave_graveyard_triggers(owner, cards);
            }
        }
        // CR 603.3b "one or more creature tokens": the whole batch, not each token, is the
        // trigger event — Make Inklings' two tokens at once fires Staff of the Storyteller once,
        // not twice. Same drain-dedup-clear shape as `graveyard_exits_this_batch` above.
        if !self
            .batch_trigger_scratch
            .creature_tokens_created_this_batch
            .is_empty()
        {
            let mut owners = std::mem::take(
                &mut self
                    .batch_trigger_scratch
                    .creature_tokens_created_this_batch,
            );
            owners.sort_unstable_by_key(|p| p.0);
            owners.dedup();
            for owner in owners {
                self.queue_controller_triggers(owner, Trigger::YouCreateToken, None);
            }
        }
        // Laelia, the Blade Reforged's growth trigger: CR "one or more cards put into exile
        // from your library and/or your graveyard" batches to one trigger, not one per card.
        // Same drain-dedup-clear shape as the two accumulators above.
        if !self
            .batch_trigger_scratch
            .library_or_graveyard_exits_this_batch
            .is_empty()
        {
            let mut owners = std::mem::take(
                &mut self
                    .batch_trigger_scratch
                    .library_or_graveyard_exits_this_batch,
            );
            owners.sort_unstable_by_key(|p| p.0);
            owners.dedup();
            for owner in owners {
                self.queue_controller_triggers(
                    owner,
                    Trigger::CardsExiledFromYourLibraryOrGraveyard,
                    None,
                );
            }
        }
        // Primo, the Unbounded's batch combat-damage Fractal trigger (CR 510.2/603.3b): reads
        // `events` directly (no scratch accumulator needed — combat damage is already fully (CR 510, CR 120.3, CR 506)
        // described on the event itself), so it runs once here rather than accumulating inside
        // the main event loop above like the three batches just above.
        self.queue_zero_base_power_combat_damage_triggers(events);
        // Scratch for this batch only — see `dying_creature_attachments`'s doc comment on `Game`.
        self.batch_trigger_scratch
            .dying_creature_attachments
            .clear();
        self.batch_trigger_scratch.dying_creature_stats.clear();
        self.batch_trigger_scratch
            .permanents_put_into_graveyard_from_battlefield
            .clear();
        self.batch_trigger_scratch
            .permanents_left_battlefield
            .clear();
    }

    /// Fire `Trigger::ThisPermanentLeavesBattlefield` for `from` if it left the battlefield to
    /// any zone this batch (recorded in `permanents_left_battlefield` by the `Game::apply` exit
    /// choke points) and its `def` has this trigger — Animate Dead's "when this Aura leaves the
    /// battlefield, sacrifice enchanted creature." Threads the captured host (CR 603.10a
    /// last-known information) into the trigger context so the payoff effect can act on "that
    /// creature." Takes `owner`/`def` rather than re-deriving them from `from` because a
    /// `TokenCeasedToExist` source's arena slot is already `Object::Removed` by the time this
    /// runs.
    fn queue_leaves_battlefield_triggers(&mut self, from: ObjectId, owner: PlayerId, def: CardDef) {
        let Some(&(_, host)) = self
            .batch_trigger_scratch
            .permanents_left_battlefield
            .iter()
            .find(|&&(id, _)| id == from)
        else {
            return;
        };
        let ctx = TriggerContext {
            left_battlefield_host: host,
            ..TriggerContext::of(owner)
        };
        self.queue_trigger_group(ctx, from, def, Trigger::ThisPermanentLeavesBattlefield);
    }

    /// Hofri Ghostforge's minted Spirit token: if `token` carries a baked
    /// [`Game::exile_links`]'s `token_leaves_returns_exiled` link (recorded by an
    /// [`Event::TokenGrantedReturnExiledOnLeave`] at mint time), queue the granted "When this
    /// token leaves the battlefield, return the exiled card to its owner's graveyard" trigger.
    /// The printed def the token copied never carries this ability, so it can't run through
    /// [`Self::queue_trigger_group`]'s ordinary `def.abilities` scan — synthesized here directly,
    /// the same way [`Self::queue_myriad_triggers`]/`queue_prowess_triggers` synthesize an
    /// unauthored ability. A no-op for every permanent leaving the battlefield without a link.
    fn queue_token_return_exiled_trigger(&mut self, token: ObjectId, controller: PlayerId) {
        let Some(&(_, exiled)) = self
            .exile_links
            .token_leaves_returns_exiled
            .iter()
            .find(|&&(t, _)| t == token)
        else {
            return;
        };
        self.pending_trigger_groups.push(TriggerGroup {
            expanded: false,
            controller,
            source: token,
            abilities: vec![Ability {
                timing: Timing::Triggered(Trigger::ThisPermanentLeavesBattlefield),
                effect: Effect::ReturnExiledCardToOwnersGraveyard { exiled },
                optional: false,
                min_level: 0,
                cost: Cost::FREE,
                condition: None,
                once_each_turn: false,
            }],
        });
    }

    /// Queue a self-referential trigger: `source` is both the event's subject and the
    /// ability's controller (via ownership). Only used for `Trigger::Etb` (see its two call
    /// sites): the entering permanent's own `Amount::X`/`Amount::HalfX` reads (The Goose
    /// Mother's "create half X Food tokens", Fractal Harness's "put X +1/+1 counters on
    /// [the token it creates]") resolve against [`Permanent::entered_with_x`], its locked-in
    /// cast `{X}` (CR 601.2b/107.3i) — the same value [`Game::ability_source_x`] returns for a
    /// `mv_max_x` re-check (Kinetic Ooze), locked in at placement like `cast_mana_value`/`cast_x`
    /// above.
    pub(crate) fn queue_self_trigger(&mut self, source: ObjectId, trigger: Trigger) {
        let ctx = TriggerContext {
            cast_x: Some(self.ability_source_x(source)),
            ..TriggerContext::of(self.owner_of(source))
        };
        self.queue_trigger_group(ctx, source, self.def_of(source), trigger);
    }

    /// Queue evoke's "sacrificed when it enters" (CR 702.74a) as its own single-ability
    /// [`TriggerGroup`], reusing [`Effect::SacrificeObject`] against `evoked_permanent` itself —
    /// the same synthetic-`then` shape a delayed sacrifice trigger uses (see that variant's doc),
    /// fabricated here since it isn't one of the permanent's own printed abilities. Its
    /// `timing`/`condition` are inert placeholders, like [`Game::fire_delayed_triggers`]'s.
    pub(crate) fn queue_evoke_sacrifice(&mut self, evoked_permanent: ObjectId) {
        self.pending_trigger_groups.push(TriggerGroup {
            expanded: false,
            controller: self.owner_of(evoked_permanent),
            source: evoked_permanent,
            abilities: vec![Ability {
                timing: Timing::Triggered(Trigger::Etb),
                effect: Effect::SacrificeObject {
                    object: Some(evoked_permanent),
                },
                optional: false,
                min_level: 0,
                cost: Cost::FREE,
                condition: None,
                once_each_turn: false,
            }],
        });
    }

    /// Queue watch-others death triggers for the death of one creature (whose id was `dying`,
    /// whose def/controller were `dying_def`/`dead_controller`, and `dying_is_token` reports
    /// whether it was a token): every watcher fires its "whenever a creature dies" ability, plus
    /// its "whenever a creature you control dies" ability when it shares that controller.
    /// Watchers are the surviving battlefield permanents *plus* the creatures that died alongside
    /// this one in the same batch (`batch_deaths`, the CR 603.6c look-back) — minus the dying
    /// creature itself for the plain arms, which never watch their own death ("another creature
    /// dies"). The dying creature *does* separately self-fire its `*IncludingThis` arms ("this
    /// creature or another creature dies" — Blood Artist / Zulaport Cutthroat), reading its own
    /// last-known information since it's already off the battlefield.
    pub(crate) fn queue_watch_death_triggers(
        &mut self,
        dead_controller: PlayerId,
        dying: ObjectId,
        dying_def: CardDef,
        batch_deaths: &[(ObjectId, CardDef, PlayerId)],
        dying_is_token: bool,
    ) {
        // Feeds `Amount::CreaturesDiedThisTurn` (Gorma, the Gullet) — called exactly once per
        // creature death (see the two `enqueue_triggers` call sites), so this is a clean count.
        self.players[dead_controller.0 as usize].creatures_died_this_turn += 1;
        for id in self.battlefield() {
            // A graveyard-functional card (Nether Traitor cast as a body) watches deaths only from
            // the graveyard (CR 603.6e) — skip it on the battlefield; the graveyard scan below fires
            // it when it's actually in the graveyard.
            if self.def_of(id).functions_in_graveyard {
                continue;
            }
            self.queue_death_watcher(
                id,
                self.def_of(id),
                self.owner_of(id),
                dead_controller,
                dying,
                dying_is_token,
            );
        }
        // Graveyard-functional death-watchers owned by the dead creature's controller (Nether
        // Traitor's "whenever another creature is put into your graveyard from the battlefield"):
        // scan the controller's graveyard so a card sitting there watches its own creatures dying.
        for id in self.graveyard_cards(dead_controller) {
            if !self.def_of(id).functions_in_graveyard {
                continue;
            }
            self.queue_death_watcher(
                id,
                self.def_of(id),
                dead_controller,
                dead_controller,
                dying,
                dying_is_token,
            );
        }
        for &(id, def, controller) in batch_deaths {
            if id == dying {
                continue;
            }
            self.queue_death_watcher(id, def, controller, dead_controller, dying, dying_is_token);
        }
        self.queue_self_death_watcher(dying, dying_def, dead_controller);
    }

    /// Queue one watcher's death triggers against a creature that just died: its `CreatureDies`
    /// ability always, its `CreatureYouControlDies` ability only when the watcher shares the dead
    /// creature's controller, and its `CreatureAnOpponentControlsDies` ability only when it
    /// doesn't (CR 102.2: in a multiplayer pod, every other player is an opponent, so
    /// `controller != dead_controller` is exactly "an opponent controls it"). The `*IncludingThis`
    /// arms fire under the same conditions — a "this or another" watcher still sees *other*
    /// creatures' deaths exactly like the plain arm. The `*Nontoken` arms (Blight Mound, Pawn of
    /// Ulamog) additionally require `!dying_is_token` — a dying token never satisfies "a nontoken
    /// creature ... dies".
    fn queue_death_watcher(
        &mut self,
        watcher: ObjectId,
        def: CardDef,
        controller: PlayerId,
        dead_controller: PlayerId,
        dying: ObjectId,
        dying_is_token: bool,
    ) {
        // The dead creature's id rides along (CR 603.10a last-known information) for a watch whose
        // payoff acts on "it"/"that creature" (Hofri Ghostforge's exile-and-copy).
        let ctx = TriggerContext {
            dead_creature: Some(dying),
            ..TriggerContext::of(controller)
        };
        self.queue_trigger_group(ctx, watcher, def, Trigger::CreatureDies);
        self.queue_trigger_group(ctx, watcher, def, Trigger::CreatureDiesIncludingThis);
        if controller == dead_controller {
            self.queue_trigger_group(ctx, watcher, def, Trigger::CreatureYouControlDies);
            self.queue_trigger_group(
                ctx,
                watcher,
                def,
                Trigger::CreatureYouControlDiesIncludingThis,
            );
            if !dying_is_token {
                self.queue_trigger_group(
                    ctx,
                    watcher,
                    def,
                    Trigger::CreatureYouControlDiesNontoken,
                );
                self.queue_trigger_group(
                    ctx,
                    watcher,
                    def,
                    Trigger::CreatureYouControlDiesIncludingThisNontoken,
                );
            }
        } else {
            self.queue_trigger_group(ctx, watcher, def, Trigger::CreatureAnOpponentControlsDies);
        }
    }

    /// Queue the dying creature's own `*IncludingThis` death triggers against its own death (CR
    /// 603.6c/603.10 last-known information: the permanent is already off the battlefield, so
    /// `def`/`dead_controller` come from the caller's snapshot, not a battlefield lookup). The
    /// `*IncludingThisNontoken` arm self-fires unconditionally too — the "nontoken" qualifier
    /// (Pawn of Ulamog: "this creature or another *nontoken* creature") only scopes the
    /// *other*-creature half, not "this creature".
    /// ponytail: only the three `*IncludingThis*` arms self-fire here — the plain `CreatureDies`/
    ///   `CreatureYouControlDies`/`CreatureYouControlDiesNontoken` arms stay self-excluded via
    ///   `queue_death_watcher`'s battlefield scan and the `batch_deaths` skip in
    ///   `queue_watch_death_triggers`.
    fn queue_self_death_watcher(
        &mut self,
        dying: ObjectId,
        def: CardDef,
        dead_controller: PlayerId,
    ) {
        let ctx = TriggerContext::of(dead_controller);
        self.queue_trigger_group(ctx, dying, def, Trigger::CreatureDiesIncludingThis);
        self.queue_trigger_group(
            ctx,
            dying,
            def,
            Trigger::CreatureYouControlDiesIncludingThis,
        );
        self.queue_trigger_group(
            ctx,
            dying,
            def,
            Trigger::CreatureYouControlDiesIncludingThisNontoken,
        );
    }

    /// Every creature that died in this event batch, as `(former-permanent id, def, controller)`.
    /// Feeds the CR 603.6c look-back in [`queue_watch_death_triggers`]. Mirrors the death arms of
    /// [`enqueue_triggers`] exactly — same creature guard, same "owner eliminated this sweep" skip
    /// — so the two never disagree on what counts as a death.
    /// ponytail: "batch" = one submit's whole event vec, so deaths that were *sequential* within a
    ///   single submit (a sequenced effect's two kills, a cascading SBA sweep) count as
    ///   simultaneous — a watcher dying in the earlier kill would over-fire for the later one.
    /// No pool effect kills in sequence within one resolution; revisit if one ever does. (CR 704, CR 108.3, CR 108.4)
    fn batch_creature_deaths(&self, events: &[Event]) -> Vec<(ObjectId, CardDef, PlayerId)> {
        let mut deaths = Vec::new();
        for event in events {
            match *event {
                Event::MovedToGraveyard { from, .. } => {
                    // Owner left the game in this same sweep: the creature went with them, not to
                    // the graveyard — no death to watch (matches the guard in `enqueue_triggers`).
                    if matches!(
                        self.objects[self.current_id(from) as usize],
                        Object::Removed
                    ) {
                        continue;
                    }
                    let def = self.def_of(from);
                    if matches!(def.kind, CardKind::Creature { .. }) {
                        deaths.push((from, def, self.owner_of(from)));
                    }
                }
                Event::TokenCeasedToExist {
                    token,
                    controller,
                    def,
                } => {
                    if matches!(def.kind, CardKind::Creature { .. }) {
                        deaths.push((token, def, controller));
                    }
                }
                _ => {}
            }
        }
        deaths
    }

    /// Queue `EnchantmentYouControlDies` watch triggers for this event batch (Starfield Mystic):
    /// for each enchantment that was put into a graveyard from the battlefield (or a token
    /// enchantment that ceased to exist), fire the ability on every other battlefield permanent
    /// its owner controls. A narrow sibling of the creature death watch (CR 603.6e "leaves the
    /// battlefield" trigger, scoped to "you control") — deliberately *not* folded into
    /// `batch_creature_deaths`/`queue_watch_death_triggers`, which must stay creature-only for the
    /// CR 603.6c look-back those feed.
    /// ponytail: "you control" only, no opponent-controller or `*IncludingThis` sibling, and no
    ///   CR 603.6c simultaneous-death look-back (an enchantment dying alongside its own watcher in
    ///   one sweep won't see it) — grow those from a real card, per flag-don't-force.
    fn queue_enchantment_death_watchers(&mut self, events: &[Event]) {
        for event in events {
            let (dying, owner, def) = match *event {
                Event::MovedToGraveyard { from, .. } => {
                    // Owner left the game in this same sweep: no death to watch (matches the
                    // guard in `enqueue_triggers`/`batch_creature_deaths`).
                    if matches!(
                        self.objects[self.current_id(from) as usize],
                        Object::Removed
                    ) {
                        continue;
                    }
                    (from, self.owner_of(from), self.def_of(from))
                }
                Event::TokenCeasedToExist {
                    token,
                    controller,
                    def,
                } => (token, controller, def),
                _ => continue,
            };
            if !def.kind.types().intersects(TypeSet::ENCHANTMENT) {
                continue;
            }
            for id in self.battlefield() {
                if id == dying || self.owner_of(id) != owner {
                    continue;
                }
                self.queue_trigger_group(
                    TriggerContext::of(owner),
                    id,
                    self.def_of(id),
                    Trigger::EnchantmentYouControlDies,
                );
            }
        }
    }

    /// Queue a controller-scoped trigger: every permanent `player` controls whose ability
    /// matches `trigger` fires. `exclude` skips one source (used to break a life-gain self-loop).
    pub(crate) fn queue_controller_triggers(
        &mut self,
        player: PlayerId,
        trigger: Trigger,
        exclude: Option<ObjectId>,
    ) {
        for id in self.battlefield() {
            if Some(id) == exclude || self.owner_of(id) != player {
                continue;
            }
            // A graveyard-functional card's triggers fire only from the graveyard (CR 603.6e), so
            // it must not also fire from the battlefield — see `queue_graveyard_controller_triggers`.
            if self.def_of(id).functions_in_graveyard {
                continue;
            }
            self.queue_trigger_group(TriggerContext::of(player), id, self.def_of(id), trigger);
        }
    }

    /// Queue [`Trigger::CardsLeaveYourGraveyard`] for `player`, threading the batch's leaving-card
    /// ids through [`TriggerContext::cards_left_graveyard`] so Spirit of Resilience's "become a
    /// copy of an artifact or creature card from among those cards" payoff bakes them in at
    /// placement (CR 603.10a last-known information). Bespoke rather than
    /// [`queue_controller_triggers`](Self::queue_controller_triggers) because that helper has no
    /// context override — same shape as [`queue_magecraft_triggers`](Self::queue_magecraft_triggers).
    pub(crate) fn queue_cards_leave_graveyard_triggers(
        &mut self,
        player: PlayerId,
        cards: &'static [ObjectId],
    ) {
        let ctx = TriggerContext {
            cards_left_graveyard: cards,
            ..TriggerContext::of(player)
        };
        for id in self.battlefield() {
            if self.owner_of(id) != player || self.def_of(id).functions_in_graveyard {
                continue;
            }
            self.queue_trigger_group(ctx, id, self.def_of(id), Trigger::CardsLeaveYourGraveyard);
        }
    }

    /// Queue Echo's pay-or-sacrifice choice (CR 702.31c) for every permanent `player` controls
    /// whose echo is still unpaid — placed one at a time, after the ordinary trigger queue, by
    /// [`Game::place_pending_triggers`]. Bespoke rather than [`queue_trigger_group`] because
    /// Echo isn't a [`Trigger`]/`[[abilities]]` on the card: it's a top-level [`CardDef::echo`]
    /// cost, checked directly against [`Permanent::echo_unpaid`].
    pub(crate) fn queue_echo_triggers(&mut self, player: PlayerId) {
        for id in self.battlefield() {
            if self.owner_of(id) != player || self.def_of(id).echo.is_none() {
                continue;
            }
            if self.permanent(id).echo_unpaid {
                self.pending_echo.push(id);
            }
        }
    }

    /// Queue [`Trigger::Magecraft`] for `player`, threading the triggering spell's mana value `mv`
    /// through [`TriggerContext::cast_mana_value`] — same context
    /// [`queue_cast_spell_triggers`](Self::queue_cast_spell_triggers) (Magecraft's general
    /// `CastSpell` form) already threads for its watchers, so `Amount::TriggeringSpellManaValue`
    /// (Deekah's Magecraft Fractal's "put X +1/+1 counters on it, where X is that spell's mana
    /// value") bakes correctly at placement (CR 603.4) whether the trigger fired off a cast or a
    /// copy. Bespoke rather than [`queue_controller_triggers`](Self::queue_controller_triggers)
    /// because that helper has no context override.
    pub(crate) fn queue_magecraft_triggers(&mut self, player: PlayerId, mv: u32) {
        for id in self.battlefield() {
            if self.owner_of(id) != player || self.def_of(id).functions_in_graveyard {
                continue;
            }
            let ctx = TriggerContext {
                cast_mana_value: Some(mv),
                ..TriggerContext::of(player)
            };
            self.queue_trigger_group(ctx, id, self.def_of(id), Trigger::Magecraft);
        }
    }

    /// Queue a controller-scoped trigger for `player`'s **graveyard-functional** cards (CR 603.6e):
    /// the graveyard twin of [`queue_controller_triggers`](Self::queue_controller_triggers). Every
    /// card in `player`'s graveyard whose [`CardDef::functions_in_graveyard`] is set and whose
    /// ability matches `trigger` fires (Squee's "at the beginning of your upkeep, you may return
    /// this card from your graveyard to your hand").
    pub(crate) fn queue_graveyard_controller_triggers(
        &mut self,
        player: PlayerId,
        trigger: Trigger,
    ) {
        for id in self.graveyard_cards(player) {
            if !self.def_of(id).functions_in_graveyard {
                continue;
            }
            self.queue_trigger_group(TriggerContext::of(player), id, self.def_of(id), trigger);
        }
    }

    /// Queue every battlefield permanent's [`Trigger::EachUpkeep`] ability at the beginning of
    /// *any* player's upkeep (CR "at the beginning of each upkeep") — unlike
    /// [`queue_controller_triggers`](Self::queue_controller_triggers), this doesn't gate on
    /// whose upkeep it is, only on the ability's own controller for the resulting effect.
    /// ponytail: carries no active-player context — the pool's each-upkeep abilities (a Pest, a
    ///   Saproling, a Snake) don't need to know whose upkeep triggered them. Add a
    ///   [`TriggerContext`] active-player field if a future card's effect reads it.
    pub(crate) fn queue_each_upkeep_triggers(&mut self) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            self.queue_trigger_group(
                TriggerContext::of(controller),
                id,
                self.def_of(id),
                Trigger::EachUpkeep,
            );
        }
    }

    /// Queue every battlefield permanent's [`Trigger::EachEndStep`] ability at the beginning of
    /// *any* player's end step (CR "at the beginning of each end step") — the end-step twin of
    /// [`queue_each_upkeep_triggers`](Self::queue_each_upkeep_triggers); see its doc for the
    /// same "doesn't gate on whose step it is" shape.
    /// ponytail: carries no active-player context, matching `queue_each_upkeep_triggers` — add a
    ///   [`TriggerContext`] active-player field if a future card's effect reads it.
    pub(crate) fn queue_each_end_step_triggers(&mut self) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            self.queue_trigger_group(
                TriggerContext::of(controller),
                id,
                self.def_of(id),
                Trigger::EachEndStep,
            );
        }
    }

    /// Queue every battlefield permanent's [`Trigger::EachOtherPlayerUntapStep`] ability at the
    /// beginning of `untapping_player`'s untap step (CR "during each other player's untap
    /// step") — the mirror image of
    /// [`queue_each_upkeep_triggers`](Self::queue_each_upkeep_triggers): this one skips
    /// permanents whose controller *is* `untapping_player` instead of firing for everyone.
    pub(crate) fn queue_each_other_player_untap_step_triggers(
        &mut self,
        untapping_player: PlayerId,
    ) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            if controller == untapping_player {
                continue;
            }
            self.queue_trigger_group(
                TriggerContext::of(controller),
                id,
                self.def_of(id),
                Trigger::EachOtherPlayerUntapStep,
            );
        }
    }

    /// Drain every CR 603.7 delayed trigger whose `fire_at` step just began — the first time any
    /// step of that kind begins after scheduling ("the beginning of the next upkeep"/"next end
    /// step" fires regardless of whose turn it is; a player-specific "your next turn" duration,
    /// like Atsushi's exile-play permission, uses a different mechanism — see
    /// [`Event::PlayFromExilePermissionArmed`]). A no-op unless `events` (the just-applied batch)
    /// contains a matching `StepBegan` and something is actually scheduled for that step. Each
    /// drained entry becomes its own single-ability [`TriggerGroup`], so it rides the normal
    /// APNAP placement path ([`Game::place_pending_triggers`]) exactly like any other triggered
    /// ability.
    pub(crate) fn fire_delayed_triggers(&mut self, events: &mut Vec<Event>) {
        let Some(fire_at) = events.iter().find_map(|e| match e {
            Event::StepBegan {
                step: step @ (Step::Upkeep | Step::End | Step::EndCombat),
                ..
            } => Some(*step),
            _ => None,
        }) else {
            return;
        };
        let due: Vec<(PlayerId, ObjectId, Effect)> = self
            .delayed_triggers
            .scheduled
            .iter()
            .filter(|&&(_, _, f, _)| f == fire_at)
            .map(|&(controller, source, _, effect)| (controller, source, effect))
            .collect();
        if due.is_empty() {
            return;
        }
        self.push_apply(events, Event::DelayedTriggersFired { fire_at });
        let trigger = match fire_at {
            Step::Upkeep => Trigger::Upkeep,
            _ => Trigger::EachEndStep,
        };
        for (controller, source, effect) in due {
            // ponytail: `timing`/`optional`/`cost`/`condition` are fabricated — a delayed
            // trigger isn't one of `source`'s printed abilities, so there's no real `Ability` to
            // borrow. `place_pending_triggers` only ever reads `effect`/`optional`/`cost` off the
            // group it pulls from `pending_trigger_groups`, so the unread `timing`/`condition`
            // values are inert placeholders, not load-bearing.
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller,
                source,
                abilities: vec![Ability {
                    timing: Timing::Triggered(trigger),
                    effect,
                    optional: false,
                    min_level: 0,
                    cost: Cost::FREE,
                    condition: None,
                    once_each_turn: false,
                }],
            });
        }
    }

    /// Enqueue a reflexive "when you do" triggered ability (CR 603.3b — Forum Filibuster). Called
    /// mid-resolution by [`Effect::ReflexiveTrigger`] once its "you do" condition is met (the
    /// parent minted `token`): each effect in `then` becomes its own single-ability
    /// [`TriggerGroup`], with `token` threaded in ([`fill_reflexive_token`]), so it rides the
    /// normal APNAP placement path ([`Game::place_pending_triggers`], run later in the pipeline)
    /// onto the stack as a real, respondable object with its own priority window and its own
    /// target chosen at placement (CR 601.2c) — exactly like any triggered ability fired by a
    /// resolving spell/ability.
    pub(crate) fn queue_reflexive_trigger(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        then: &'static [Effect],
        token: ObjectId,
    ) {
        for &effect in then {
            // ponytail: `timing`/`condition` are fabricated inert placeholders, the same way
            // `fire_delayed_triggers` fabricates them — a reflexive ability isn't one of `source`'s
            // printed abilities, and `place_pending_triggers` only reads `effect`/`optional`/`cost`.
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller,
                source,
                abilities: vec![Ability {
                    timing: Timing::Triggered(Trigger::Upkeep),
                    effect: effect.with_reflexive_token(token),
                    optional: false,
                    min_level: 0,
                    cost: Cost::FREE,
                    condition: None,
                    once_each_turn: false,
                }],
            });
        }
    }

    /// Queue watch-others *attack* triggers: a player attacked `attacked`, so scan every
    /// battlefield permanent and fire its `PlayerAttacksYourOpponent` ability when `attacked` is
    /// one of that permanent's controller's opponents (i.e. isn't the controller themself). The
    /// attacking player and attacked opponent ride along in the [`TriggerContext`].
    pub(crate) fn queue_watch_attack_triggers(
        &mut self,
        attacker_object: ObjectId,
        attacked: PlayerId,
    ) {
        let attacking_player = self.controller_of(attacker_object);
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            // "one of your opponents": skip watchers whose own controller was the one attacked.
            if controller == attacked {
                continue;
            }
            let ctx = TriggerContext {
                controller,
                attack: Some((attacking_player, attacked)),
                discarded: None,
                entering: None,
                dying_source_stats: None,
                cast_mana_value: None,
                cast_mana_spent: None,
                cast_x: None,
                auras_you_controlled_attached_to_dying_creature: None,
                combat_damage: None,
                dying_enchanted_creature: None,
                triggering_spell: None,
                source_power: None,
                dead_creature: None,
                cards_left_graveyard: &[],
                left_battlefield_host: None,
                triggering_ability: None,
            };
            self.queue_trigger_group(ctx, id, self.def_of(id), Trigger::PlayerAttacksYourOpponent);
        }
    }

    /// How many of `attackers` are each enchanted by an Aura `controller` controls (CR 303.4) —
    /// Killian, Decisive Mentor's "one or more creatures that are enchanted by an Aura you
    /// control attack" batch gate.
    fn attackers_enchanted_by_aura_controlled_by(
        &self,
        attackers: &[(ObjectId, PlayerId)],
        controller: PlayerId,
    ) -> u8 {
        attackers
            .iter()
            .filter(|&&(atk, _)| {
                !self
                    .auras_controlled_by_attached_to(atk, controller)
                    .is_empty()
            })
            .count() as u8
    }

    /// Queue the batch attack-count triggers (CR 508.1, "attack with two or more creatures"):
    /// `attacking_player` just declared the whole `attackers` set for this combat. Scan it once,
    /// not per single [`Event::AttackerDeclared`] (a per-event fire can't see "two or more").
    /// Three shapes: [`Trigger::YouAttackWithCreatures`] fires on `attacking_player`'s own
    /// permanents, gated on their total attacker count this combat (any defenders);
    /// [`Trigger::OpponentAttacksYouWithCreatures`] fires on a defending player's permanents,
    /// gated on how many of `attacking_player`'s creatures are attacking that defender —
    /// per-opponent, so two different attacking players sending one creature each don't combine;
    /// [`Trigger::CreatureEnchantedByYourAuraAttacks`] fires on every battlefield permanent,
    /// gated on how many of the *whole* attacker set (any defender) are enchanted by an Aura the
    /// watcher's own controller controls (CR 508.1, Killian, Decisive Mentor's second ability).
    /// Called directly from [`Game::declare_attackers`] once the attacker set is committed,
    /// rather than from [`Self::enqueue_triggers`]'s per-event scan.
    pub(crate) fn queue_batch_attack_triggers(
        &mut self,
        attacking_player: PlayerId,
        attackers: &[(ObjectId, PlayerId)],
    ) {
        let total_attackers = attackers.len() as u8;
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let against_controller = attackers
                .iter()
                .filter(|&&(_, defender)| defender == controller)
                .count() as u8;
            let ctx = if controller == attacking_player {
                TriggerContext::of(controller)
            } else {
                TriggerContext {
                    attack: Some((attacking_player, controller)),
                    ..TriggerContext::of(controller)
                }
            };
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::YouAttackWithCreatures { at_least }) => {
                        controller == attacking_player && total_attackers >= at_least
                    }
                    Timing::Triggered(Trigger::OpponentAttacksYouWithCreatures { at_least }) => {
                        controller != attacking_player && against_controller >= at_least
                    }
                    Timing::Triggered(Trigger::AnotherPlayerAttacksWithCreatures { at_least }) => {
                        controller != attacking_player
                            && total_attackers >= at_least
                            && against_controller == 0
                    }
                    Timing::Triggered(Trigger::CreatureEnchantedByYourAuraAttacks { at_least }) => {
                        self.attackers_enchanted_by_aura_controlled_by(attackers, controller)
                            >= at_least
                    }
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// Queue attached-permanent attack triggers (CR 508.1, the Impetus cycle — and CR 301.5g's
    /// Equipment twin, Fractal Harness's "whenever equipped creature attacks"): the attacking
    /// `attacker_object`'s attachments (`self.attachments`, Auras *and* Equipment alike) each
    /// fire their `EnchantedCreatureAttacks` ability, controlled by *that attachment's own
    /// controller* (not the attacker's) — an Aura enchanting an opponent's creature is the
    /// Impetus cycle's usual home. The enchanted/equipped creature's controller and the defended
    /// player ride along in the [`TriggerContext`]'s `attack` tuple, the same shape
    /// [`queue_watch_attack_triggers`](Self::queue_watch_attack_triggers) uses, so a drain effect
    /// (Parasitic Impetus) can read the host's controller off it.
    pub(crate) fn queue_enchanted_creature_attacks_triggers(
        &mut self,
        attacker_object: ObjectId,
        defender: PlayerId,
    ) {
        let host_controller = self.controller_of(attacker_object);
        for aura in self.attachments(attacker_object) {
            let ctx = TriggerContext {
                controller: self.controller_of(aura),
                attack: Some((host_controller, defender)),
                discarded: None,
                entering: None,
                dying_source_stats: None,
                cast_mana_value: None,
                cast_mana_spent: None,
                cast_x: None,
                auras_you_controlled_attached_to_dying_creature: None,
                combat_damage: None,
                dying_enchanted_creature: None,
                triggering_spell: None,
                source_power: None,
                dead_creature: None,
                cards_left_graveyard: &[],
                left_battlefield_host: None,
                triggering_ability: None,
            };
            self.queue_trigger_group(
                ctx,
                aura,
                self.def_of(aura),
                Trigger::EnchantedCreatureAttacks,
            );
        }
    }

    /// Queue Aura-attached death triggers (CR "When enchanted creature dies…"): the death twin of
    /// [`queue_enchanted_creature_attacks_triggers`](Self::queue_enchanted_creature_attacks_triggers).
    /// Each Aura that was attached to `dying` (read from the pre-move
    /// [`Self::dying_creature_attachments`] snapshot, since the Aura has itself already left the
    /// battlefield in this same state-based-action sweep — an ordinary Aura into its own graveyard
    /// card, a *token* Aura, CR 707.10a, straight to [`Object::Removed`]) fires its
    /// `EnchantedCreatureDies` ability, controlled by *that Aura's own controller* — not the dying
    /// creature's. The controller/def ride on the snapshot itself rather than a live `controller_of`/
    /// `def_of(aura)` re-read, which would panic for a vanished token Aura.
    ///
    /// Skip an Aura whose owner left the game in this same sweep (CR 800.4a) — its trigger must
    /// not fire, checked directly off the captured controller's [`Player::lost`] rather than
    /// probing the aura object's own removal (which no longer distinguishes "owner eliminated"
    /// from "this was always going to vanish here," now that a token Aura vanishes the same way).
    pub(crate) fn queue_enchanted_creature_dies_triggers(&mut self, dying: ObjectId) {
        let auras: Vec<(ObjectId, PlayerId, CardDef)> = self
            .batch_trigger_scratch
            .dying_creature_attachments
            .iter()
            .filter(|&&(host, ..)| host == dying)
            .map(|&(_, aura, controller, def)| (aura, controller, def))
            .collect();
        for (aura, controller, def) in auras {
            if self.players[controller.0 as usize].lost {
                continue;
            }
            let ctx = TriggerContext {
                dying_enchanted_creature: Some(dying),
                ..TriggerContext::of(controller)
            };
            self.queue_trigger_group(ctx, aura, def, Trigger::EnchantedCreatureDies);
        }
    }

    /// Queue watch-any-enchanted-creature-dies triggers (CR 603.6c, Hateful Eidolon: "Whenever
    /// an enchanted creature dies, draw a card for each Aura you controlled that was attached to
    /// it.") — the watch-others twin of
    /// [`queue_enchanted_creature_dies_triggers`](Self::queue_enchanted_creature_dies_triggers):
    /// placed on every battlefield permanent, not just the dying creature's own Auras, and gated
    /// per watcher on how many of the Auras attached to `dying` (the same pre-move
    /// [`Self::dying_creature_attachments`] snapshot) that watcher's own controller controlled.
    /// `batch_deaths` extends the watcher set to creatures that died alongside `dying` in the
    /// same event batch (they're already off the battlefield) — the CR 603.6c look-back, same
    /// shape as [`queue_watch_death_triggers`](Self::queue_watch_death_triggers).
    pub(crate) fn queue_an_enchanted_creature_dies_triggers(
        &mut self,
        dying: ObjectId,
        batch_deaths: &[(ObjectId, CardDef, PlayerId)],
    ) {
        let auras: Vec<(ObjectId, PlayerId)> = self
            .batch_trigger_scratch
            .dying_creature_attachments
            .iter()
            .filter(|&&(host, _, controller, def)| {
                // Skip an Aura whose owner left the game in this same sweep (CR 800.4a) — its
                // watchers must not count it, checked directly off the captured controller's
                // `Player::lost` (see `queue_enchanted_creature_dies_triggers`'s own doc for why
                // this no longer probes the aura object's own removal).
                host == dying
                    && !self.players[controller.0 as usize].lost
                    && def.kind == CardKind::Aura
            })
            .map(|&(_, aura, controller, _)| (aura, controller))
            .collect();
        if auras.is_empty() {
            return;
        }
        for id in self.battlefield() {
            self.queue_an_enchanted_creature_dies_watcher(
                id,
                self.def_of(id),
                self.owner_of(id),
                &auras,
            );
        }
        for &(id, def, controller) in batch_deaths {
            if id == dying {
                continue;
            }
            self.queue_an_enchanted_creature_dies_watcher(id, def, controller, &auras);
        }
    }

    /// Queue one watcher's [`Trigger::AnEnchantedCreatureDies`] ability against `auras` (the
    /// Auras that were attached to the creature that just died), gated on `controller` (the
    /// watcher's own) having controlled at least one of them — CR 603.10a last-known
    /// information, the same "fire only if the count is nonzero" shortcut documented on
    /// [`Trigger::AnEnchantedCreatureDies`] itself. The count feeds
    /// `Amount::AurasYouControlledAttachedToDyingCreature` via [`contextualize_effect`].
    fn queue_an_enchanted_creature_dies_watcher(
        &mut self,
        watcher: ObjectId,
        def: CardDef,
        controller: PlayerId,
        auras: &[(ObjectId, PlayerId)],
    ) {
        let count = auras
            .iter()
            .filter(|&&(_, aura_controller)| aura_controller == controller)
            .count() as u32;
        if count == 0 {
            return;
        }
        let ctx = TriggerContext {
            auras_you_controlled_attached_to_dying_creature: Some(count),
            ..TriggerContext::of(controller)
        };
        self.queue_trigger_group(ctx, watcher, def, Trigger::AnEnchantedCreatureDies);
    }

    /// Queue sacrifice-watch triggers for a permanent (`sacrificed`, whose card definition was
    /// `def`) that player `by` just sacrificed (CR 701.20): `Trigger::YouSacrifice` on `by`'s own
    /// battlefield permanents whose `filter` matches (Smothering Abomination), and
    /// `Trigger::AnyPlayerSacrifices` on every battlefield permanent whose `filter` matches,
    /// regardless of controller (Mazirek). Bespoke rather than routed through
    /// [`queue_trigger_group`](Self::queue_trigger_group), like the death-watch triggers, because
    /// each watcher needs its *own* filter checked against the sacrificed permanent, not just a
    /// trigger-tag match.
    pub(crate) fn queue_sacrifice_triggers(
        &mut self,
        sacrificed: ObjectId,
        by: PlayerId,
        def: CardDef,
    ) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let ctx = TriggerContext::of(controller);
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::YouSacrifice { filter }) => {
                        controller == by && sacrifice_matches(&filter, def, id, sacrificed)
                    }
                    Timing::Triggered(Trigger::AnyPlayerSacrifices { filter }) => {
                        sacrifice_matches(&filter, def, id, sacrificed)
                    }
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// Queue `Trigger::YouDiscard` watchers on every permanent `player` controls (CR 701.8):
    /// fires whenever they discard a card, regardless of source — an `Effect::Discard`
    /// resolution, a discard-cost payment, or the cleanup hand-size trim all route through here.
    /// Like [`queue_controller_triggers`](Self::queue_controller_triggers), but threads
    /// `discarded` (the discarded card's new graveyard-object id) into the [`TriggerContext`] so
    /// the effect can act on "that card" (Containment Construct).
    pub(crate) fn queue_discard_triggers(&mut self, player: PlayerId, discarded: ObjectId) {
        let ctx = TriggerContext {
            controller: player,
            attack: None,
            discarded: Some(discarded),
            entering: None,
            dying_source_stats: None,
            cast_mana_value: None,
            cast_mana_spent: None,
            cast_x: None,
            auras_you_controlled_attached_to_dying_creature: None,
            combat_damage: None,
            dying_enchanted_creature: None,
            triggering_spell: None,
            source_power: None,
            dead_creature: None,
            cards_left_graveyard: &[],
            left_battlefield_host: None,
            triggering_ability: None,
        };
        for id in self.battlefield() {
            if self.owner_of(id) != player {
                continue;
            }
            self.queue_trigger_group(ctx, id, self.def_of(id), Trigger::YouDiscard);
        }
    }

    /// Every graveyard card, across every living player, whose def is flagged
    /// [`CardDef::functions_in_graveyard`] (CR 603.6e) — the `PermanentEnters` watch's graveyard
    /// twin of [`Game::graveyard_cards`], scanned for *every* player rather than one because a
    /// watcher's `controller` scope (you/opponent/any_player) is checked against the entering
    /// permanent's controller independently of whose graveyard it sits in (Vanguard of the
    /// Restless's "whenever a Spirit you control enters, you may pay {2}{W}...").
    fn graveyard_functional_watchers(&self) -> Vec<ObjectId> {
        self.living_players()
            .flat_map(|p| self.graveyard_cards(p))
            .filter(|&id| self.def_of(id).functions_in_graveyard)
            .collect()
    }

    /// Queue enters-the-battlefield triggers (CR 702.76a constellation / CR 704.5n-kin
    /// landfall): permanent `entering` just entered the battlefield. Scans every *other*
    /// battlefield permanent's `PermanentEnters`/`PermanentEntersIncludingThis`
    /// `{ filter, controller }` ability and fires it when `filter` matches `entering` (relative
    /// to the watcher's own controller, [`Game::permanent_matches`]) and `controller`'s scope
    /// holds between `entering`'s controller and the watcher's; then queues `entering`'s own
    /// `PermanentEntersIncludingThis` ability against itself
    /// ([`queue_self_permanent_enters_trigger`](Self::queue_self_permanent_enters_trigger)).
    /// Mirrors [`Game::queue_sacrifice_triggers`] exactly: bespoke rather than routed through
    /// [`queue_trigger_group`](Self::queue_trigger_group), because each watcher needs its own
    /// filter/scope checked against `entering`, not just a trigger-tag match. The watcher's own
    /// entry never fires its own *plain* `PermanentEnters` ability — guarded by the
    /// `id == entering` skip, not by `filter.other` (that axis stays available for a filter that
    /// separately wants "another permanent" semantics); only the `IncludingThis` arm self-fires.
    pub(crate) fn queue_permanent_enters_triggers(&mut self, entering: ObjectId) {
        // Feeds `Amount::NontokenCreaturesEnteredThisTurn` (Gyome, Master Chef) — called exactly
        // once per entering permanent (this function's two call sites, the cast/reanimate/search
        // family and `Event::TokenCreated`), so this is a clean count; a token doesn't bump it.
        // `entering` may already have left the battlefield by the time this runs — a state-based
        // action (e.g. an unattached Aura, CR 704.5m) can sweep it away in the same batch before
        // triggers are queued (CR 704 SBAs precede trigger placement) — so read it as a possibly-
        // dead permanent rather than the live-only `Game::permanent`; a permanent that's already
        // gone simply doesn't bump the counter.
        if let Some(perm) = self.as_permanent(entering)
            && !perm.token
            && matches!(self.def_of(entering).kind, CardKind::Creature { .. })
        {
            let controller = self.controller_of(entering);
            self.players[controller.0 as usize].nontoken_creatures_entered_this_turn += 1;
        }
        // Feeds `Condition::LandEnteredUnderYourControlThisTurn` (Zimone, All-Questioning) — CR
        // landfall's own "enters," not "played," so a cast, fetched, *or token* land all set it;
        // unlike the nontoken-creature tally above, this doesn't exclude tokens.
        if self.as_permanent(entering).is_some()
            && matches!(self.def_of(entering).kind, CardKind::Land { .. })
        {
            let controller = self.controller_of(entering);
            self.players[controller.0 as usize].land_entered_under_your_control_this_turn = true;
        }
        // A graveyard-functional watcher's `PermanentEnters` ability fires only from the
        // graveyard (CR 603.6e) — the same battlefield exclusion `queue_controller_triggers`
        // already enforces, so it doesn't also self-fire (and duplicate itself) while it's a
        // live permanent.
        let watchers: Vec<ObjectId> = self
            .battlefield()
            .into_iter()
            .filter(|&id| !self.def_of(id).functions_in_graveyard)
            .chain(self.graveyard_functional_watchers())
            .collect();
        for id in watchers {
            if id == entering {
                continue;
            }
            let controller = self.owner_of(id);
            let ctx = TriggerContext {
                entering: Some(entering),
                ..TriggerContext::of(controller)
            };
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    // The plain and `IncludingThis` arms both watch *other* permanents
                    // identically — only the entering permanent's own `IncludingThis` self-fire
                    // (below) tells them apart.
                    Timing::Triggered(
                        Trigger::PermanentEnters {
                            filter,
                            controller: scope,
                        }
                        | Trigger::PermanentEntersIncludingThis {
                            filter,
                            controller: scope,
                        },
                    ) => {
                        let entering_controller = self.controller_of(entering);
                        let scope_matches = match scope {
                            EnterController::You => entering_controller == controller,
                            EnterController::Opponent => entering_controller != controller,
                            EnterController::AnyPlayer => true,
                        };
                        scope_matches
                            && self.permanent_matches(&filter, entering, controller, Some(id))
                    }
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
        self.queue_self_permanent_enters_trigger(entering);
    }

    /// Queue `entering`'s own `PermanentEntersIncludingThis` ability against its own entry (CR
    /// 603.6a "this permanent or another … enters" — Doomwake Giant's constellation firing off
    /// its own ETB). Unlike death's last-known-information self-fire
    /// ([`queue_self_death_watcher`](Self::queue_self_death_watcher)), `entering` is already on
    /// the battlefield here, so this reads it directly rather than off a snapshot.
    fn queue_self_permanent_enters_trigger(&mut self, entering: ObjectId) {
        let controller = self.owner_of(entering);
        let ctx = TriggerContext::of(controller);
        let abilities: Vec<Ability> = self
            .functional_abilities(entering)
            .iter()
            .filter(|a| match a.timing {
                Timing::Triggered(Trigger::PermanentEntersIncludingThis {
                    filter,
                    controller: scope,
                }) => {
                    let scope_matches = match scope {
                        EnterController::You | EnterController::AnyPlayer => true,
                        EnterController::Opponent => false,
                    };
                    scope_matches
                        && self.permanent_matches(&filter, entering, controller, Some(entering))
                }
                _ => false,
            })
            .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
            .map(|a| Ability {
                effect: contextualize_effect(a.effect, ctx),
                ..*a
            })
            .collect();
        if !abilities.is_empty() {
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller,
                source: entering,
                abilities,
            });
        }
    }

    /// Queue combat-damage-to-a-player watch triggers (CR 510.2): `source` (a creature) just
    /// dealt `amount` combat damage to a player. Scans every battlefield permanent's
    /// `DealsCombatDamageToPlayer{who}` ability and fires it when `who` matches: `This` only for
    /// `source`'s own ability, `YourCreatures` for any battlefield permanent that shares
    /// `source`'s controller, `YourTokens` the same but only when `source` is a token. Bespoke
    /// rather than routed through [`queue_trigger_group`](Self::queue_trigger_group), like the
    /// sacrifice-watch and attack-watch triggers, because each watcher's `who` must be checked
    /// against `source` individually rather than matched against one fixed `Trigger` value.
    /// `amount` is baked onto `This`-scoped watchers' context as CR 603.10a last-known
    /// information (`ctx.combat_damage`, Venerable Warsinger's "mana value X or less … where X is
    /// the amount of damage this creature dealt to that player") — see [`TriggerContext`].
    /// ponytail: `amount` is only threaded for `who = This` (the pool's one consumer); a future
    /// `YourCreatures`/`YourTokens` amount-reading card needs a per-watcher sum, not this event's
    /// single `amount`. (CR 510, CR 111, CR 108.3)
    pub(crate) fn queue_combat_damage_triggers(&mut self, source: ObjectId, amount: i32) {
        let source_controller = self.controller_of(source);
        let source_is_token = self.permanent(source).token;
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let ctx = TriggerContext {
                combat_damage: (id == source).then_some(amount),
                ..TriggerContext::of(controller)
            };
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::DealsCombatDamageToPlayer { who }) => match who {
                        CombatDamageScope::This => id == source,
                        CombatDamageScope::YourCreatures => controller == source_controller,
                        CombatDamageScope::YourTokens => {
                            controller == source_controller && source_is_token
                        }
                    },
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// Queue [`Trigger::ZeroBasePowerCreaturesYouControlDealCombatDamage`] watch triggers (CR
    /// 510.2/603.3b, Primo, the Unbounded): scans `events` for every
    /// [`Event::CombatDamageDealtToPlayer`] dealt by a creature with **base** power 0 (read off
    /// `def_of(source).kind`, not [`Game::power`](Self::power) — a counter-pumped Fractal has
    /// current power above 0 but still qualifies on base power), sums each `(controller,
    /// defending player)` pair's damage across every qualifying source, and fires that
    /// controller's watcher ability once per defending player — CR 603.3b's "one or more", a
    /// whole player's worth of qualifying attackers is one trigger, not one per source — with the
    /// summed damage baked into [`TriggerContext::combat_damage`] (CR 603.10a last-known
    /// information, locked into the effect by [`contextualize_effect`]/`fill_combat_damage` at
    /// placement, same shape as [`queue_combat_damage_triggers`](Self::queue_combat_damage_triggers)'s
    /// per-source damage). A small `Vec` accumulator, not a `HashMap` — deterministic iteration
    /// order, sorted before firing so two controllers/defenders fire in a fixed order.
    /// ponytail: the base-power-0 filter is hard-coded (no `PermanentFilter` field) — see the
    /// `Trigger` variant's own doc.
    pub(crate) fn queue_zero_base_power_combat_damage_triggers(&mut self, events: &[Event]) {
        let mut totals: Vec<(PlayerId, PlayerId, i32)> = Vec::new();
        for event in events {
            let &Event::CombatDamageDealtToPlayer {
                source,
                player,
                amount,
            } = event
            else {
                continue;
            };
            if !matches!(
                self.def_of(source).kind,
                CardKind::Creature { power: 0, .. }
            ) {
                continue;
            }
            let controller = self.controller_of(source);
            match totals
                .iter_mut()
                .find(|&&mut (c, d, _)| c == controller && d == player)
            {
                Some((_, _, total)) => *total += amount,
                None => totals.push((controller, player, amount)),
            }
        }
        if totals.is_empty() {
            return;
        }
        totals.sort_unstable_by_key(|&(controller, defender, _)| (controller.0, defender.0));
        for (controller, _defender, total) in totals {
            for id in self.battlefield() {
                if self.controller_of(id) != controller {
                    continue;
                }
                let ctx = TriggerContext {
                    combat_damage: Some(total),
                    ..TriggerContext::of(controller)
                };
                self.queue_trigger_group(
                    ctx,
                    id,
                    self.def_of(id),
                    Trigger::ZeroBasePowerCreaturesYouControlDealCombatDamage,
                );
            }
        }
    }

    /// The this-turn cast tally `nth_each_turn` reads for a given watcher `filter` — filter-
    /// scoped for `SpellFilter::HasXInCost` (Nev/Zimone's "first spell WITH {X} each turn" reads
    /// [`Player::x_spells_cast_this_turn`], not every spell the caster cast), the whole-turn
    /// [`Player::spells_cast_this_turn`] for every other filter (Monologue Tax/Mangara's
    /// unfiltered "their second spell").
    fn cast_tally_for(&self, filter: SpellFilter, caster: PlayerId) -> u32 {
        let player = &self.players[caster.0 as usize];
        match filter {
            SpellFilter::HasXInCost => player.x_spells_cast_this_turn,
            _ => player.spells_cast_this_turn,
        }
    }

    /// Queue [`Trigger::CastSpell`] watch triggers (the general form behind Magecraft): a spell
    /// (`def`, aimed at `target`, cast with chosen `{X}` = `x`) was just cast by
    /// `spell_controller`. Scans every battlefield
    /// permanent's `CastSpell{filter, caster, nth_each_turn}` ability and fires it when the
    /// caster scope matches relative to the watcher's own controller, the spell matches `filter`
    /// ([`Game::spell_matches_filter`]), and — if `nth_each_turn` is set — the caster's
    /// filter-scoped cast tally ([`Game::cast_tally_for`]) equals it. Bespoke rather than routed
    /// through [`queue_trigger_group`](Self::queue_trigger_group), like the sacrifice/combat-
    /// damage watches, because each watcher's own filter/caster/nth must be checked individually
    /// rather than matched against one fixed `Trigger` value.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn queue_cast_spell_triggers(
        &mut self,
        spell_controller: PlayerId,
        spell: ObjectId,
        def: CardDef,
        target: Option<Target>,
        x: u32,
        cast_from_hand: bool,
        mana_spent: u32,
    ) {
        // ponytail: `Event::SpellCast` already incremented `spells_cast_this_turn` (and, for
        //   `has_x` spells, `x_spells_cast_this_turn`) by the time this runs (apply.rs), so
        //   `nth_each_turn == n` reads the post-increment tally — this cast is already counted.
        //   That's the intended reading ("their second spell" means the cast currently resolving
        //   is the second), not a bug to fix.
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let ctx = TriggerContext {
                cast_mana_value: Some(def.mana_value()),
                // CR 601.2h: the mana actually spent on this cast, locked in when the trigger
                // goes on the stack, same last-known-information shape as `cast_mana_value`
                // above — fills a watcher's `Amount::TriggeringSpellManaSpent` (Manaform
                // Hellkite's "X is the amount of mana spent to cast that spell").
                cast_mana_spent: Some(mana_spent),
                // CR 603.4: the triggering spell's chosen {X} is locked in when the trigger goes
                // on the stack, same last-known-information shape as `cast_mana_value` above —
                // fills a watcher's `Amount::X`/`put_counters { count = "x" }` (Nev's payoff).
                cast_x: Some(x),
                // The live cast-watch's own copy payoff (Unbound Flourishing / Owlin
                // Spiralmancer's "copy it"): the spell that fired this watch, baked into a
                // matched ability's `Effect::CopyTriggeringSpell` by `contextualize_effect`
                // below, same last-known-information shape as `cast_x` above.
                triggering_spell: Some(spell),
                ..TriggerContext::of(controller)
            };
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::CastSpell {
                        filter,
                        caster,
                        nth_each_turn,
                        from_hand,
                    }) => {
                        let caster_matches = match caster {
                            CasterScope::You => spell_controller == controller,
                            CasterScope::Opponent => spell_controller != controller,
                            CasterScope::AnyPlayer => true,
                        };
                        // ponytail: only `SpellFilter::HasXInCost` gets its own tally
                        //   (`x_spells_cast_this_turn`) — every other filter still falls back to
                        //   the whole-turn `spells_cast_this_turn`, which is correct for the
                        //   unfiltered users (Monologue Tax/Mangara's "their second spell") but
                        //   would misfire for a *filtered* `nth_each_turn` on any other filter
                        //   axis. No card in the pool pairs `nth_each_turn` with a non-`has_x`
                        //   filter today; add that filter's own tally when one does.
                        let cast_tally = self.cast_tally_for(filter, spell_controller);
                        caster_matches
                            // A `cast_spell` trigger's own `from_hand` gate (below) is the pool's
                            // cast-zone predicate; no card pairs it with a `cast_from_non_hand_zone`
                            // spell filter, so the plain hand-cast default suffices here.
                            && self.spell_matches_filter(
                                filter,
                                def,
                                target,
                                spell_controller,
                                Zone::Hand,
                            )
                            && nth_each_turn.is_none_or(|n| cast_tally == u32::from(n))
                            && (!from_hand || cast_from_hand)
                    }
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// Fire the [`Trigger::ActivateAbility`] watchers (Unbound Flourishing's "or activate an
    /// ability … copy that ability", CR 707.10) when `activator` puts an `{X}`-cost activated
    /// ability on the stack from `source`. Called directly from [`Game::activate_ability`] once
    /// the ability is placed (like [`Self::queue_batch_attack_triggers`]), not from
    /// [`Self::enqueue_triggers`]'s per-event scan. `source` (the ability's own permanent) rides
    /// in [`TriggerContext::triggering_ability`] so a matched [`Effect::CopyTriggeringAbility`]
    /// can find that ability on the stack and copy it.
    pub(crate) fn queue_activate_ability_triggers(
        &mut self,
        activator: PlayerId,
        source: ObjectId,
    ) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let ctx = TriggerContext {
                // CR 603.4: the triggering ability's source, locked in when the watch fires — the
                // payoff's `Effect::CopyTriggeringAbility` reads it back via `contextualize_effect`.
                // The copied ability carries its own chosen `{X}` on its stack item, so no `cast_x`
                // context is needed here.
                triggering_ability: Some(source),
                ..TriggerContext::of(controller)
            };
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::ActivateAbility { caster }) => match caster {
                        CasterScope::You => activator == controller,
                        CasterScope::Opponent => activator != controller,
                        CasterScope::AnyPlayer => true,
                    },
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// "When you spend this mana to cast …" (Study Hall / Path of Ancestry / Opal Palace): fire
    /// the `Trigger::SpendManaToCast` of every land whose provenance-tagged mana
    /// ([`Player::mana_provenance`](crate::state)) funded `spell`, a qualifying cast by `caster`.
    /// `spend` is the multiset the cast's payment removed (the preceding `Event::ManaSpent` in the
    /// same batch). A tagged credit fires iff (a) its kind is present in `spend` and (b) `spell`
    /// matches the source's [`SpendToCastPredicate`]; the matched credit is consumed from both the
    /// spend copy (so two same-kind tags only both fire if two were actually spent) and provenance
    /// (so it can't fire again). See [`Player::mana_provenance`](crate::state) for the summed-pool
    /// approximation this leaves.
    pub(crate) fn queue_spend_to_cast_triggers(
        &mut self,
        caster: PlayerId,
        spell: ObjectId,
        spend: ManaPool,
    ) {
        if self.players[caster.0 as usize].mana_provenance.is_empty() {
            return;
        }
        let mut remaining = spend;
        let provenance = std::mem::take(&mut self.players[caster.0 as usize].mana_provenance);
        let mut kept = Vec::with_capacity(provenance.len());
        for (source, mana) in provenance {
            let Some(predicate) = self.spend_to_cast_predicate_of(source) else {
                kept.push((source, mana));
                continue;
            };
            if self.spell_matches_spend_predicate(predicate, caster, spell)
                && remaining.take_one(mana)
            {
                // Thread the funded spell in so Opal Palace's rider
                // (`Effect::CommanderEntersWithBonusCounters`) can key its bonus counters to it;
                // Study Hall / Path of Ancestry's scry payoffs ignore it (harmless).
                let ctx = TriggerContext {
                    triggering_spell: Some(spell),
                    ..TriggerContext::of(self.owner_of(source))
                };
                self.queue_trigger_group(
                    ctx,
                    source,
                    self.def_of(source),
                    Trigger::SpendManaToCast { predicate },
                );
                continue; // matched entry consumed — dropped from provenance
            }
            kept.push((source, mana));
        }
        self.players[caster.0 as usize].mana_provenance = kept;
    }

    /// The [`SpendToCastPredicate`] of the land at `source`'s `spend_mana_to_cast` ability, if it
    /// has one — read back so [`Self::queue_spend_to_cast_triggers`] fires the exact trigger the
    /// source's `[[abilities]]` prints.
    fn spend_to_cast_predicate_of(&self, source: ObjectId) -> Option<SpendToCastPredicate> {
        self.def_of(source)
            .abilities
            .iter()
            .find_map(|a| match a.timing {
                Timing::Triggered(Trigger::SpendManaToCast { predicate }) => Some(predicate),
                _ => None,
            })
    }

    /// Whether `spell` (a cast by `caster`) satisfies a [`SpendToCastPredicate`].
    fn spell_matches_spend_predicate(
        &self,
        predicate: SpendToCastPredicate,
        caster: PlayerId,
        spell: ObjectId,
    ) -> bool {
        match predicate {
            SpendToCastPredicate::Commander => {
                self.is_commander(spell) && self.owner_of(spell) == caster
            }
            SpendToCastPredicate::CreatureSharingTypeWithCommander => {
                self.spell_shares_creature_type_with_commander(caster, spell)
            }
        }
    }

    /// Fire CR 603.7 delayed one-shots armed by [`Effect::ScheduleNextCastTrigger`] (Brass
    /// Infiniscope's "When you next cast a spell with {X} in its mana cost this turn, …"): scans
    /// the just-applied batch for `Event::SpellCast` and, for every pending
    /// [`DelayedTriggers::pending_next_cast`](crate::state::DelayedTriggers::pending_next_cast)
    /// watch whose controller cast that spell and whose `filter` it matches
    /// ([`Game::spell_matches_filter`]), fires it exactly once — removed via
    /// [`Event::NextCastTriggerConsumed`] before its `TriggerGroup` is queued, CR 603.7's "next".
    /// A sibling of [`Self::fire_delayed_triggers`] (delayed-until-a-*step*) but event-armed
    /// rather than step-armed, hence its own drain rather than overloading `delayed_triggers.
    /// scheduled`. `then`'s `Amount::X`/`Amount::HalfXRoundedDown` are filled from the triggering
    /// cast's own chosen `{X}` via [`TriggerContext::cast_x`], same CR 603.4 last-known-
    /// information shape [`Self::queue_cast_spell_triggers`] already uses.
    pub(crate) fn fire_next_cast_triggers(&mut self, events: &mut Vec<Event>) {
        if self.delayed_triggers.pending_next_cast.is_empty() {
            return;
        }
        let casts: Vec<(PlayerId, ObjectId, CardDef, Option<Target>, u32)> = events
            .iter()
            .filter_map(|e| match *e {
                Event::SpellCast {
                    spell,
                    controller,
                    target,
                    x,
                    ..
                } => Some((controller, spell, self.def_of(spell), target, x)),
                _ => None,
            })
            .collect();
        for (spell_controller, spell, def, target, x) in casts {
            let matches: Vec<(PlayerId, ObjectId, SpellFilter, &'static [Effect])> = self
                .delayed_triggers
                .pending_next_cast
                .iter()
                .copied()
                .filter(|&(controller, _, filter, _)| {
                    controller == spell_controller
                        && self.spell_matches_filter(
                            filter,
                            def,
                            target,
                            spell_controller,
                            Zone::Hand,
                        )
                })
                .collect();
            for (controller, source, filter, then) in matches {
                self.push_apply(
                    events,
                    Event::NextCastTriggerConsumed { controller, source },
                );
                let ctx = TriggerContext {
                    cast_x: Some(x),
                    triggering_spell: Some(spell),
                    ..TriggerContext::of(controller)
                };
                let effect = match then {
                    [only] => *only,
                    _ => Effect::Sequence { steps: then },
                };
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source,
                    abilities: vec![Ability {
                        timing: Timing::Triggered(Trigger::CastSpell {
                            filter,
                            caster: CasterScope::You,
                            nth_each_turn: None,
                            from_hand: false,
                        }),
                        effect: contextualize_effect(effect, ctx),
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    }],
                });
            }
        }
    }

    /// Fire CR 603.7 delayed watches armed by [`Effect::ArmCombatDamageWatch`] (Stensian
    /// Sanguinist's "Whenever that creature deals combat damage to a player this combat, this
    /// creature becomes prepared"): scans the just-applied batch for
    /// [`Event::CombatDamageDealtToPlayer`] and, for every pending
    /// [`DelayedTriggers::pending_combat_damage_watch`](crate::state::DelayedTriggers::pending_combat_damage_watch)
    /// entry whose watched creature just dealt that damage, fires [`Effect::BecomePrepared`] on
    /// the arming ability's source — removed via [`Event::CombatDamageWatchConsumed`] before its
    /// `TriggerGroup` is queued, CR 603.7's "this combat" one-shot. A sibling of
    /// [`Self::fire_next_cast_triggers`] (filter-armed) but object-armed instead, hence its own
    /// drain rather than overloading `pending_next_cast`. `timing`/`condition` are fabricated
    /// placeholders — same shape as [`Self::fire_delayed_triggers`]'s note — since a delayed
    /// watch isn't one of `source`'s printed abilities.
    pub(crate) fn fire_combat_damage_watch_triggers(&mut self, events: &mut Vec<Event>) {
        if self.delayed_triggers.pending_combat_damage_watch.is_empty() {
            return;
        }
        let damages: Vec<ObjectId> = events
            .iter()
            .filter_map(|e| match *e {
                Event::CombatDamageDealtToPlayer { source, .. } => Some(source),
                _ => None,
            })
            .collect();
        for damage_source in damages {
            let matches: Vec<(PlayerId, ObjectId)> = self
                .delayed_triggers
                .pending_combat_damage_watch
                .iter()
                .copied()
                .filter(|&(_, _, watched)| watched == damage_source)
                .map(|(controller, source, _)| (controller, source))
                .collect();
            for (controller, source) in matches {
                self.push_apply(
                    events,
                    Event::CombatDamageWatchConsumed { controller, source },
                );
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source,
                    abilities: vec![Ability {
                        timing: Timing::Triggered(Trigger::DealsCombatDamageToPlayer {
                            who: CombatDamageScope::This,
                        }),
                        effect: Effect::BecomePrepared,
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    }],
                });
            }
        }
    }

    /// Fire CR 603.7's *repeatable* delayed watches armed by
    /// [`Effect::ScheduleThisTurnCombatDamageCopy`] (Surge to Victory's "Whenever a creature you
    /// control deals combat damage to a player this turn, copy the exiled card. You may cast the
    /// copy without paying its mana cost."): scans the just-applied batch for
    /// [`Event::CombatDamageDealtToPlayer`] and, for every armed
    /// [`DelayedTriggers::pending_combat_damage_copy`](crate::state::DelayedTriggers::pending_combat_damage_copy)
    /// entry whose `controller` controls the creature that just dealt the damage, queues a
    /// [`TriggerGroup`] that mints one free copy of the exiled `card` via
    /// [`Effect::MintFreeCopyOfExiledCard`]. Unlike
    /// [`Self::fire_combat_damage_watch_triggers`], entries are **not** removed here — CR 603.7's
    /// "this turn" fires again on every subsequent qualifying combat-damage event; an unconsumed
    /// entry is only cleared at the next turn's Untap step (`Game::apply`'s `Step::Untap` arm).
    /// `timing`/`condition` are fabricated placeholders, same shape as
    /// `fire_combat_damage_watch_triggers`'s own note.
    pub(crate) fn fire_combat_damage_copy_triggers(&mut self, events: &[Event]) {
        if self.delayed_triggers.pending_combat_damage_copy.is_empty() {
            return;
        }
        let damages: Vec<ObjectId> = events
            .iter()
            .filter_map(|e| match *e {
                Event::CombatDamageDealtToPlayer { source, .. } => Some(source),
                _ => None,
            })
            .collect();
        for damage_source in damages {
            let damage_controller = self.controller_of(damage_source);
            let matches: Vec<(PlayerId, ObjectId, ObjectId)> = self
                .delayed_triggers
                .pending_combat_damage_copy
                .iter()
                .copied()
                .filter(|&(controller, _, _)| controller == damage_controller)
                .collect();
            for (controller, source, card) in matches {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source,
                    abilities: vec![Ability {
                        timing: Timing::Triggered(Trigger::DealsCombatDamageToPlayer {
                            who: CombatDamageScope::YourCreatures,
                        }),
                        effect: Effect::MintFreeCopyOfExiledCard { card: Some(card) },
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    }],
                });
            }
        }
    }

    /// Queue [`Trigger::PlayerDraws`] watch triggers (Faerie Mastermind's "an opponent draws
    /// their second card each turn"): `drawer` just drew a card. Scans every battlefield
    /// permanent's `PlayerDraws{drawer, nth_each_turn}` ability and fires it when the drawer
    /// scope matches relative to the watcher's own controller and — if `nth_each_turn` is set —
    /// `nth` (this specific draw's ordinal for the drawer this turn, computed by the caller)
    /// equals it. Bespoke rather than routed through
    /// [`queue_trigger_group`](Self::queue_trigger_group), mirroring
    /// [`queue_cast_spell_triggers`](Self::queue_cast_spell_triggers): each watcher's own
    /// drawer/nth must be checked individually rather than matched against one fixed
    /// `Trigger` value.
    pub(crate) fn queue_player_draws_triggers(&mut self, drawer: PlayerId, nth: u32) {
        for id in self.battlefield() {
            let controller = self.owner_of(id);
            let ctx = TriggerContext::of(controller);
            let abilities: Vec<Ability> = self
                .functional_abilities(id)
                .iter()
                .filter(|a| match a.timing {
                    Timing::Triggered(Trigger::PlayerDraws {
                        drawer: scope,
                        nth_each_turn,
                    }) => {
                        let drawer_matches = match scope {
                            CasterScope::You => drawer == controller,
                            CasterScope::Opponent => drawer != controller,
                            CasterScope::AnyPlayer => true,
                        };
                        drawer_matches && nth_each_turn.is_none_or(|n| nth == u32::from(n))
                    }
                    _ => false,
                })
                .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
                .map(|a| Ability {
                    effect: contextualize_effect(a.effect, ctx),
                    ..*a
                })
                .collect();
            if !abilities.is_empty() {
                self.pending_trigger_groups.push(TriggerGroup {
                    expanded: false,
                    controller,
                    source: id,
                    abilities,
                });
            }
        }
    }

    /// Queue [`Trigger::BecomesTargeted`] triggers (CR 603.2c "becomes the target of a
    /// spell"): a spell just declared `spell_target`. Self-referential, like
    /// [`Game::queue_permanent_enters_triggers`]'s `Etb` sibling — unlike the
    /// battlefield-scanning watches above, there's exactly one possible source (the targeted
    /// permanent itself), so this looks it up directly rather than scanning every permanent.
    /// ponytail: the engine's spells carry a single [`Target`] (multi-target is unlanded), so
    /// this fires at most once per cast — faithful for Goldspan Dragon, the only consumer. A
    /// spell with no target, or one targeting a player rather than a permanent, fires nothing.
    pub(crate) fn queue_becomes_targeted_triggers(&mut self, spell_target: Option<Target>) {
        let Some(Target::Object(id)) = spell_target else {
            return;
        };
        if !matches!(self.objects[id as usize], Object::Permanent(_)) {
            return;
        }
        let controller = self.controller_of(id);
        let ctx = TriggerContext::of(controller);
        let abilities: Vec<Ability> = self
            .functional_abilities(id)
            .iter()
            .filter(|a| matches!(a.timing, Timing::Triggered(Trigger::BecomesTargeted)))
            .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
            .map(|a| Ability {
                effect: contextualize_effect(a.effect, ctx),
                ..*a
            })
            .collect();
        if !abilities.is_empty() {
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller,
                source: id,
                abilities,
            });
        }
    }

    /// Queue [`Trigger::SpellTargetsThisOnly`] triggers (Mirrorwing Dragon — CR 603.2c narrowed
    /// to "targets only this creature"): `def` was just cast, its lone `spell_target` the same
    /// single-[`Target`] field [`Self::queue_becomes_targeted_triggers`] reads. Self-referential,
    /// like that sibling — the only possible source is the targeted permanent itself.
    pub(crate) fn queue_spell_targets_this_only_triggers(
        &mut self,
        spell_target: Option<Target>,
        spell_controller: PlayerId,
        spell: ObjectId,
        def: CardDef,
    ) {
        let Some(Target::Object(id)) = spell_target else {
            return;
        };
        if !matches!(self.objects[id as usize], Object::Permanent(_)) {
            return;
        }
        let controller = self.controller_of(id);
        let ctx = TriggerContext {
            triggering_spell: Some(spell),
            ..TriggerContext::of(controller)
        };
        let abilities: Vec<Ability> = self
            .functional_abilities(id)
            .iter()
            .filter(|a| {
                let Timing::Triggered(Trigger::SpellTargetsThisOnly { filter }) = a.timing else {
                    return false;
                };
                self.spell_matches_filter(filter, def, spell_target, spell_controller, Zone::Hand)
            })
            .filter(|a| a.condition.is_none_or(|c| self.condition_holds(c, ctx)))
            .map(|a| Ability {
                effect: contextualize_effect(a.effect, ctx),
                ..*a
            })
            .collect();
        if !abilities.is_empty() {
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller,
                source: id,
                abilities,
            });
        }
    }

    /// Queue prowess triggers (CR 702.108): `spell_controller` just cast `def` (aimed at
    /// `target`). Every battlefield creature that creature controls and that carries
    /// [`Keyword::Prowess`] gets a one-ability [`TriggerGroup`] pumping itself +1/+1 until end
    /// of turn, provided the cast spell is noncreature (CR 702.108b — the pump is a real
    /// triggered ability, not a static bonus, so it rides the normal stack/APNAP placement path).
    // ponytail: prowess is synthesized here from the keyword rather than authored as a TOML
    //   `[[abilities]]` — the keyword *is* the whole ability (CR 702.108a), so there's nothing
    //   to script. `timing`/`condition` on the fabricated `Ability` are inert placeholders, same
    //   as `fire_delayed_upkeep_triggers`'s fabricated abilities above.
    pub(crate) fn queue_prowess_triggers(
        &mut self,
        spell_controller: PlayerId,
        def: CardDef,
        target: Option<Target>,
    ) {
        // Prowess's noncreature filter never reads the cast-from zone; pass the hand-cast default.
        if !self.spell_matches_filter(
            SpellFilter::NoncreatureSpells,
            def,
            target,
            spell_controller,
            Zone::Hand,
        ) {
            return;
        }
        for id in self.battlefield() {
            if self.controller_of(id) != spell_controller || !self.has_keyword(id, Keyword::Prowess)
            {
                continue;
            }
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller: spell_controller,
                source: id,
                abilities: vec![Ability {
                    timing: Timing::Triggered(Trigger::Magecraft),
                    effect: Effect::PumpSelfUntilEndOfTurn {
                        power: Amount::Fixed(1),
                        toughness: Amount::Fixed(1),
                    },
                    optional: false,
                    min_level: 0,
                    cost: Cost::FREE,
                    condition: None,
                    once_each_turn: false,
                }],
            });
        }
    }

    /// Queue Myriad (CR 702.114): `object` (an attacker carrying [`Keyword::Myriad`]) just
    /// attacked `defender`. The whole ability *is* the keyword (CR 702.114a) — like Prowess,
    /// there's no printed `[[abilities]]` to scan for, so it's synthesized here directly rather
    /// than authored in TOML (see [`Game::queue_prowess_triggers`]'s doc for the pattern this
    /// mirrors). A no-op unless `object` currently has the keyword (Muddle, the Ever-Changing
    /// grants it to itself temporarily via its magecraft ability).
    pub(crate) fn queue_myriad_triggers(&mut self, object: ObjectId, defender: PlayerId) {
        if !self.has_keyword(object, Keyword::Myriad) {
            return;
        }
        self.pending_trigger_groups.push(TriggerGroup {
            expanded: false,
            controller: self.controller_of(object),
            source: object,
            abilities: vec![Ability {
                timing: Timing::Triggered(Trigger::Attacks),
                effect: Effect::MyriadTokenCopies {
                    attacking_context: Some((self.controller_of(object), defender)),
                },
                optional: false,
                min_level: 0,
                cost: Cost::FREE,
                condition: None,
                once_each_turn: false,
            }],
        });
    }

    /// The `(power, +1/+1 counters)` this batch's `Game::dying_creature_stats` recorded for
    /// `source` the instant before it died, for a `Dies` trigger's `TriggerContext` — `None` if
    /// `source` didn't die this batch (every non-death trigger).
    fn dying_source_stats(&self, source: ObjectId) -> Option<(i32, i32)> {
        self.batch_trigger_scratch
            .dying_creature_stats
            .iter()
            .find(|&&(id, ..)| id == source)
            .map(|&(_, power, counters)| (power, counters))
    }

    /// Queue `def`'s abilities matching `trigger` (and whose intervening-if condition, if any,
    /// holds for `ctx`) as one group controlled by `ctx.controller`, sourced from `source`. A
    /// no-op when no ability qualifies.
    /// The abilities `target` has gained from a live Backup grant (CR 702.166 — Guardian
    /// Scalelord), read live off each grant's source's [`CardDef`] minus the ability that carried
    /// the grant itself (Backup grants only the source's *other* abilities). Empty for a permanent
    /// with no active grant. Read by [`Self::queue_trigger_group`] so a granted triggered ability
    /// fires for the target the same way an own one does.
    pub(crate) fn granted_source_abilities(&self, target: ObjectId) -> Vec<Ability> {
        let mut granted = Vec::new();
        for &(t, source) in &self.abilities_granted_until_eot {
            if t != target {
                continue;
            }
            for &ability in self.def_of(source).abilities {
                if ability_grants_source_abilities(ability) {
                    continue;
                }
                granted.push(ability);
            }
        }
        granted
    }

    pub(crate) fn queue_trigger_group(
        &mut self,
        ctx: TriggerContext,
        source: ObjectId,
        def: CardDef,
        trigger: Trigger,
    ) {
        // CR 613.1e/701 "loses all abilities": a live host under an ability-removing Aura
        // (Darksteel Mutation) contributes none of its own triggered abilities. A death
        // last-known-information `def` snapshot is never this case — the permanent has already
        // left the battlefield, so `host_loses_all_abilities` is false and the snapshot's
        // triggers still fire (CR 603.10).
        if self.host_loses_all_abilities(source) {
            return;
        }
        // A level-gated triggered ability functions only while its Class source is at least that
        // level (CR 717.5). Every ordinary ability is `min_level = 0`, and a non-permanent
        // source (a spell's `YouCastThis`, a graveyard card) is trivially "level 1".
        // ponytail: the bespoke trigger scanners (`queue_player_draws_triggers`,
        //   `queue_cast_spell_triggers`, …) don't route through here and so don't honor
        //   `min_level` — no pool Class gates one of those triggers; gate them there if one ever does.
        let source_level = self.as_permanent(source).map_or(1, |p| p.level);
        // A Backup grant (CR 702.166) makes `source` gain another permanent's abilities until end
        // of turn — so scan those too, alongside its own def's, addressing them as `source`'s.
        let granted = self.granted_source_abilities(source);
        let abilities: Vec<Ability> = def
            .abilities
            .iter()
            .copied()
            .chain(granted)
            .filter(|a| {
                a.timing == Timing::Triggered(trigger)
                    && a.min_level <= source_level
                    && a.condition
                        .is_none_or(|c| self.ability_condition_holds(c, source, ctx))
            })
            .map(|a| Ability {
                effect: contextualize_effect(a.effect, ctx),
                ..a
            })
            .collect();
        if !abilities.is_empty() {
            self.pending_trigger_groups.push(TriggerGroup {
                expanded: false,
                controller: ctx.controller,
                source,
                abilities,
            });
        }
    }

    /// Whether an intervening-if [`Condition`] holds for a trigger fired by `source`. A thin
    /// wrapper over [`Self::condition_holds`] that special-cases the source-object-based
    /// conditions `condition_holds` can't reach on its own (`TriggerContext` carries no source
    /// id) — [`Condition::ThisPermanentEnteredUntapped`] (Mystic Sanctuary),
    /// [`Condition::SourceHasNoCountersOfKind`] (mana_bloom's upkeep self-bounce), and
    /// [`Condition::SourceHasCounters`] (Ingenious Prodigy's upkeep may-draw).
    pub(crate) fn ability_condition_holds(
        &self,
        condition: Condition,
        source: ObjectId,
        ctx: TriggerContext,
    ) -> bool {
        match condition {
            Condition::ThisPermanentEnteredUntapped => {
                self.as_permanent(source).is_some_and(|p| !p.tapped)
            }
            Condition::SourceHasNoCountersOfKind { kind } => {
                self.counters_of_kind(source, kind) == 0
            }
            // Ingenious Prodigy's upkeep: "if this creature has one or more +1/+1 counters on
            // it" — source-object-based like the two conditions above.
            Condition::SourceHasCounters { at_least } => self.source_has_counters(source, at_least),
            _ => self.condition_holds(condition, ctx),
        }
    }

    /// Whether an intervening-if [`Condition`] holds against the current state and `ctx`.
    pub(crate) fn condition_holds(&self, condition: Condition, ctx: TriggerContext) -> bool {
        match condition {
            Condition::YouControlAtLeastCreatures { count } => {
                self.creatures_controlled(ctx.controller) as u32 >= count
            }
            // "that opponent has more life than another of your opponents": some other opponent
            // of the controller (not the attacked one) has strictly less life than the attacked.
            Condition::AttackedOpponentHasMoreLifeThanAnotherOpponent => {
                let Some((_, attacked)) = ctx.attack else {
                    return false;
                };
                let attacked_life = self.life(attacked);
                self.living_players()
                    .any(|p| p != ctx.controller && p != attacked && self.life(p) < attacked_life)
            }
            Condition::ControlsLandsWithSubtype { subtypes, count } => {
                self.lands_with_subtype_controlled(ctx.controller, subtypes) as u32 >= count
            }
            Condition::ControlsBasicLands { count } => {
                self.basic_lands_controlled(ctx.controller) as u32 >= count
            }
            Condition::OpponentsControlLands { count } => {
                self.lands_controlled_by_others(ctx.controller) as u32 >= count
            }
            Condition::HandHasLandWithSubtype { subtypes } => {
                self.hand_has_land_with_subtype(ctx.controller, subtypes)
            }
            Condition::OpponentControlsMoreLands => {
                let mine = self.lands_controlled(ctx.controller);
                // Archaeomancer's Map's landfall carries the entering land's controller in
                // `ctx.entering` — CR "if *that player* controls more lands than you" means
                // specifically them, not any opponent (a pod could have a different opponent
                // ahead on lands while the one whose land just entered isn't). Land Tax's upkeep
                // trigger has no `entering` context, so it keeps the "any opponent" reading its
                // own wording asks for.
                match ctx.entering {
                    Some(entering) => self.lands_controlled(self.controller_of(entering)) > mine,
                    None => self
                        .living_players()
                        .any(|p| p != ctx.controller && self.lands_controlled(p) > mine),
                }
            }
            Condition::YouControlLands { at_least } => {
                self.lands_controlled(ctx.controller) as u32 >= at_least
            }
            Condition::YouGainedLifeThisTurn => {
                self.players[ctx.controller.0 as usize].life_gained_this_turn > 0
            }
            Condition::ModifiedCreatureDiedThisTurn => {
                self.players[ctx.controller.0 as usize].modified_creature_died_this_turn
            }
            Condition::CardLeftYourGraveyardThisTurn => {
                self.players[ctx.controller.0 as usize].card_left_graveyard_this_turn
            }
            Condition::CastInstantOrSorceryThisTurn => {
                self.players[ctx.controller.0 as usize].instant_or_sorcery_cast_this_turn
            }
            Condition::YouControlNoSubtype {
                subtypes,
                token,
                types,
            } => self.controls_no_subtype(ctx.controller, subtypes, token, types),
            Condition::YouControlNoCreatureWithKeyword { keyword } => {
                self.controls_no_creature_with_keyword(ctx.controller, keyword)
            }
            // ponytail: `SourceHasCounters` is source-object-based (a permanent's own +1/+1
            // count, CR 702), but `TriggerContext` carries no source object — only the ability's
            // controller. Reachable two ways: directly against the object by
            // `Game::source_has_counters` (the characteristics recompute's conditional-keyword
            // gate, Primordial Hydra's trample), or through `Game::ability_condition_holds` (CR 702)
            // (Ingenious Prodigy's upkeep may-draw), which intercepts it before falling through
            // here.
            Condition::SourceHasCounters { .. } => false,
            // ponytail: source-object-based like `SourceHasCounters` above — reachable only
            // through `Game::ability_condition_holds` (mana_bloom's upkeep trigger, queued via
            // `queue_trigger_group`), which intercepts it before falling through here.
            Condition::SourceHasNoCountersOfKind { .. } => false,
            Condition::YouControlColorPermanents { color, at_least } => {
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.controller_of(id) == ctx.controller
                            && self.colors_of(id)[color.index()]
                    })
                    .count() as u32
                    >= at_least
            }
            // ponytail: source-object-based like `SourceHasCounters` above — `queue_trigger_group`
            // special-cases it directly against its own `source` parameter (see there) rather than
            // through `condition_holds`, since `TriggerContext` carries no source id either.
            // Unreachable through any other `condition_holds` caller (an activation-gate use has no
            // self-evident "this permanent" to read).
            Condition::ThisPermanentEnteredUntapped => false,
            // ponytail: target-based like `ThisPermanentEnteredUntapped` above — `TriggerContext`
            // carries no target either. Reachable only through the `Effect::Conditional` resolve
            // site (`Game::run`), which intercepts it directly against the shared
            // `target` before falling through here (Yavimaya Bloomsage's power-7 check).
            Condition::TargetPowerAtLeast { .. } => false,
            Condition::TriggeringSpellManaValueAtLeast { at_least } => ctx
                .cast_mana_value
                .is_some_and(|mv| mv >= u32::from(at_least)),
            Condition::YouHaveCitysBlessing => {
                self.players[ctx.controller.0 as usize].has_citys_blessing
            }
            Condition::AnyPlayerHandSizeAtMost { at_most } => self
                .living_players()
                .any(|p| self.hand_of(p).len() as u32 <= at_most),
            Condition::InstantOrSorceryCardsInYourGraveyardAtLeast { count } => {
                self.graveyard_cards(ctx.controller)
                    .into_iter()
                    .filter(|&id| matches!(self.def_of(id).kind, CardKind::Spell { .. }))
                    .count() as u32
                    >= count
            }
            Condition::ArtifactOrCreatureCardsInYourGraveyardAtLeast { count } => {
                self.graveyard_cards(ctx.controller)
                    .into_iter()
                    .filter(|&id| {
                        matches!(
                            self.def_of(id).kind,
                            CardKind::Artifact | CardKind::Creature { .. }
                        )
                    })
                    .count() as u32
                    >= count
            }
            Condition::AnOpponentHasLifeAtMost { at_most } => self
                .living_players()
                .any(|p| p != ctx.controller && self.life(p) <= at_most as i32),
            // ponytail: source-object-based like `TargetPowerAtLeast` above — `TriggerContext`
            // carries no source id either. Reachable only through the `Effect::Conditional`
            // resolve site (`Game::run`), which intercepts it directly against its own `source`
            // parameter before falling through here (Kinetic Ooze's X-threshold riders).
            Condition::SourceEnteredWithXAtLeast { .. } => false,
            Condition::All { conditions } => {
                conditions.iter().all(|&c| self.condition_holds(c, ctx))
            }
            Condition::LandEnteredUnderYourControlThisTurn => {
                self.players[ctx.controller.0 as usize].land_entered_under_your_control_this_turn
            }
            Condition::YouControlPrimeNumberOfLands => {
                is_prime(self.lands_controlled(ctx.controller))
            }
            Condition::DuringYourTurn => self.active_player == ctx.controller,
        }
    }

    /// Whether `object` (a permanent) has `at_least` or more +1/+1 counters on it — CR 702's
    /// counter-count check, read directly off [`Permanent::plus_counters`]. Backs
    /// `Condition::SourceHasCounters` where the caller already has the object in hand (the
    /// characteristics recompute's conditional-keyword gate) rather than a [`TriggerContext`].
    pub(crate) fn source_has_counters(&self, object: ObjectId, at_least: u32) -> bool {
        self.plus_counters(object) >= at_least as i32
    }

    /// Whether `controller` controls no permanent whose printed subtypes intersect `subtypes`
    /// (Ophiomancer's "you control no Snakes"; Pest Rescuer's "you don't control a Pest creature
    /// token"), optionally restricted by [`TokenFilter`] and [`TypeSet`] (Pest Rescuer: creature
    /// tokens only).
    pub(crate) fn controls_no_subtype(
        &self,
        controller: PlayerId,
        subtypes: &[&str],
        token: TokenFilter,
        types: TypeSet,
    ) -> bool {
        !self.battlefield().into_iter().any(|id| {
            if self.controller_of(id) != controller {
                return false;
            }
            let perm = self.permanent(id);
            if !types.is_empty() && !types.intersects(perm.def.kind.types()) {
                return false;
            }
            match token {
                TokenFilter::Any => {}
                TokenFilter::Token if !perm.token => return false,
                TokenFilter::Nontoken if perm.token => return false,
                _ => {}
            }
            self.def_of(id)
                .subtypes
                .iter()
                .any(|s| subtypes.contains(s))
        })
    }

    /// "You control no creatures with `keyword`" (Jadar's "no creatures with decayed") — the
    /// effective-keyword sibling of [`Self::controls_no_subtype`]: a granted/temp keyword counts,
    /// not just a printed one.
    pub(crate) fn controls_no_creature_with_keyword(
        &self,
        controller: PlayerId,
        keyword: Keyword,
    ) -> bool {
        !self.battlefield().into_iter().any(|id| {
            self.controller_of(id) == controller
                && matches!(self.def_of(id).kind, CardKind::Creature { .. })
                && self.has_keyword(id, keyword)
        })
    }

    /// How many lands `player` controls (Temple of the False God's "five or more lands"; the
    /// per-opponent count behind [`Condition::OpponentControlsMoreLands`]).
    pub(crate) fn lands_controlled(&self, player: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                self.controller_of(id) == player
                    && matches!(self.def_of(id).kind, CardKind::Land { .. })
            })
            .count()
    }

    /// Whether a permanent with `def` enters the battlefield tapped under `controller` (CR
    /// 614.13): the unconditional `enters_tapped` flag, or — if the card carries a conditional
    /// gate — the *negation* of that [`Condition`] ("enters tapped *unless* …"), evaluated right
    /// now at the land's one ETB site ([`Event::LandPlayed`]). Reuses [`Game::condition_holds`],
    /// the same intervening-if evaluator triggers use, so a land-count/subtype condition is
    /// written once and read from both places.
    pub(crate) fn enters_tapped(&self, def: CardDef, controller: PlayerId) -> bool {
        match def.enters_tapped_unless {
            Some(condition) => !self.condition_holds(condition, TriggerContext::of(controller)),
            None => def.enters_tapped,
        }
    }

    /// How many lands `controller` controls whose printed subtypes intersect `subtypes`
    /// (Clifftop Retreat: a Mountain or a Plains; Mystic Sanctuary: Islands).
    pub(crate) fn lands_with_subtype_controlled(
        &self,
        controller: PlayerId,
        subtypes: &[&str],
    ) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                self.controller_of(id) == controller
                    && match self.def_of(id).kind {
                        CardKind::Land {
                            subtypes: land_subtypes,
                            ..
                        } => land_subtypes.iter().copied().any(|s| subtypes.contains(&s)),
                        _ => false,
                    }
            })
            .count()
    }

    /// How many basic lands `controller` controls (Eclipsed Steppe's "two or more basic lands").
    pub(crate) fn basic_lands_controlled(&self, controller: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| self.controller_of(id) == controller && is_basic_land(self.def_of(id)))
            .count()
    }

    /// How many lands every player *other than* `controller` controls, combined (the
    /// turbulent_* cycle's "unless opponents control eight or more lands").
    pub(crate) fn lands_controlled_by_others(&self, controller: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                self.controller_of(id) != controller
                    && matches!(self.def_of(id).kind, CardKind::Land { .. })
            })
            .count()
    }

    /// Whether `controller`'s hand contains a card whose printed subtypes intersect `subtypes`
    /// (the reveal lands' automatic hand scan — see [`Condition::HandHasLandWithSubtype`]).
    pub(crate) fn hand_has_land_with_subtype(
        &self,
        controller: PlayerId,
        subtypes: &[&str],
    ) -> bool {
        self.objects.iter().any(|o| match o {
            Object::Card(c) if c.zone == Zone::Hand && c.owner == controller => match c.def.kind {
                CardKind::Land {
                    subtypes: land_subtypes,
                    ..
                } => land_subtypes.iter().copied().any(|s| subtypes.contains(&s)),
                _ => false,
            },
            _ => false,
        })
    }

    /// Resolve an [`Amount`] to a concrete number, in the context of an effect resolving for
    /// `controller`, sourced from `source`, aimed at `target`, with the casting spell's chosen `x`.
    /// The single amount evaluator — every numeric effect routes here, so a new derived value is
    /// one match arm.
    pub(crate) fn resolve_amount(
        &self,
        amount: Amount,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> i32 {
        match amount {
            Amount::Fixed(n) => n,
            Amount::X => x as i32,
            // "half X, rounded up" (CR: the round-up default).
            Amount::HalfX => x.div_ceil(2) as i32,
            // ponytail: a live read (`x = 0` for every non-spell resolution, see `Amount`'s own
            // doc) never actually reaches here for `Trigger::YouCastThis` — `fill_cast_x` rewrites
            // this to `Fixed` at trigger placement (CR 603.4). The arm exists only so this match
            // stays exhaustive, mirroring `TriggeringSpellManaValue` below.
            Amount::HalfXRoundedDown => (x / 2) as i32,
            Amount::TwiceX => 2 * x as i32,
            Amount::PerCreatureYouControl => self.creatures_controlled(controller) as i32,
            Amount::PerCreatureOnBattlefield => self.creatures_on_battlefield() as i32,
            Amount::PerPermanentMatching { filter, zone } => {
                self.count_matching(&filter, zone, controller, source) as i32
            }
            Amount::SourcePower => self.power(source),
            Amount::SourceToughness => self.toughness(source),
            Amount::TargetPower => {
                self.power(expect_object_target(target, "a power-derived amount"))
            }
            Amount::TargetToughness => {
                self.toughness(expect_object_target(target, "a toughness-derived amount"))
            }
            Amount::TargetManaValue => self
                .def_of(expect_object_target(target, "a mana-value amount"))
                .mana_value() as i32,
            Amount::PerCounterOnSource => self.plus_counters(source),
            Amount::PerCounterOfKindOnSource { kind } => self.counters_of_kind(source, kind) as i32,
            Amount::LifeGainedThisTurn => {
                self.players[controller.0 as usize].life_gained_this_turn as i32
            }
            Amount::SpellsCastThisTurn => {
                self.players[controller.0 as usize].spells_cast_this_turn as i32
            }
            // Reads the resolving spell's chosen player target's hand size (Rousing Refrain's
            // "for each card in target opponent's hand"), off the target like
            // `CommanderCastsFromCommandZone` above.
            Amount::CardsInTargetPlayerHand => match target {
                Some(Target::Player(player)) => self.hand_of(player).len() as i32,
                other => panic!(
                    "a target-player-hand amount resolves with a chosen player target, got {other:?}"
                ),
            },
            // A live read off the effect's controller (Empyrial Armor) — no target involved.
            Amount::CardsInYourHand => self.hand_of(controller).len() as i32,
            // ponytail: reads the single commander's counter (matches the shared command_casts
            // tax counter, apply.rs). A partner-commander pair would need to sum both commanders'
            // counts; no soc-pool player has more than one commander.
            Amount::CommanderCastsFromCommandZone => {
                // A chosen player target (Commander's Insight's "target player") reads off that
                // player; a no-target context (an anthem like Vanguard of the Restless, which
                // always reads its own controller's count) falls back to `controller`.
                let player = match target {
                    Some(Target::Player(player)) => player,
                    None => controller,
                    Some(other) => panic!(
                        "a command-zone-cast amount resolves with a chosen player target or no target, got {other:?}"
                    ),
                };
                self.players[player.0 as usize].command_casts as i32
            }
            Amount::CreaturesDiedThisTurn => {
                self.players[controller.0 as usize].creatures_died_this_turn as i32
            }
            Amount::NontokenCreaturesEnteredThisTurn => {
                self.players[controller.0 as usize].nontoken_creatures_entered_this_turn as i32
            }
            Amount::TotalPowerYouControl => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.owner_of(id) == controller
                        && matches!(self.def_of(id).kind, CardKind::Creature { .. })
                })
                .map(|id| self.power(id))
                .sum(),
            Amount::IfCondition { condition, then } => {
                if !self.condition_holds(condition, TriggerContext::of(controller)) {
                    return 0;
                }
                self.resolve_amount(*then, controller, source, target, x)
            }
            // A placeholder [`contextualize_sacrifice_effect`] must have already rewritten to
            // `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::SacrificedCreaturePower => panic!(
                "Amount::SacrificedCreaturePower must be contextualized to Fixed before resolving"
            ),
            // A placeholder [`contextualize_sacrifice_effect`] must have already rewritten to
            // `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::SacrificedCreatureToughness => panic!(
                "Amount::SacrificedCreatureToughness must be contextualized to Fixed before resolving"
            ),
            Amount::CommanderColorCount => self
                .commander_identity_of(controller)
                .iter()
                .filter(|&&has_color| has_color)
                .count() as i32,
            // ponytail: like `SacrificedCreaturePower` above, a placeholder — `fill_cast_mana_value`
            // must have already rewritten it to `Fixed` before the ability reaches the stack (every
            // `CastSpell`-triggered ability's effect is contextualized at placement), so a live read (CR 603, CR 113)
            // here never happens. The arm exists only so this match stays exhaustive.
            Amount::TriggeringSpellManaValue => 0,
            // Same placeholder shape as `TriggeringSpellManaValue` above, one arm down —
            // `fill_cast_mana_spent` rewrites it to `Fixed` before the ability reaches the stack.
            Amount::TriggeringSpellManaSpent => 0,
            Amount::SpellSacrificeCount => self.spell_sacrifice_count(source) as i32,
            Amount::PermanentsDiedThisTurn => self.permanents_died_this_turn as i32,
            // Reads the snapshot `Effect::DestroyAll`'s own `Game::run` special case just
            // recorded, restricted to `filter` (empty/default matches every destroyed permanent
            // — Culling Ritual's unfiltered mana count). No `permanent_matches` reuse: the
            // permanents are already off the battlefield by the time a following `Sequence`
            // step reads this, so matching runs against the snapshot's `def`/`controller`/
            // `token` facts instead of live board state.
            Amount::PermanentsDestroyedThisWay { filter } => self
                .destroyed_this_way
                .iter()
                .filter(|snap| destroyed_this_way_matches(&filter, controller, snap))
                .count() as i32,
            // Reads the snapshot `Effect::EachPlayerExilesFromGraveyard` recorded (Augusta's "put
            // that many +1/+1 counters"); resolution-scoped, like `PermanentsDestroyedThisWay`.
            Amount::NonlandCardsExiledThisWay => self.nonland_cards_exiled_this_way as i32,
            // Reads the tallies this resolution's own `Effect::CouncilsDilemmaVote` round
            // accumulated (Fateful Tempest); resolution-scoped, like `NonlandCardsExiledThisWay`.
            Amount::PastVotes => self.council_past_votes as i32,
            Amount::PresentVotes => self.council_present_votes as i32,
            // Reads the mana value the preceding `Effect::MillSelf` step snapshotted (Fateful
            // Tempest's "damage … equal to the total mana value of cards milled this way").
            Amount::TotalManaValueMilledThisWay => self.milled_mana_value_this_way as i32,
            // Reads the mana value the preceding `Effect::ExileTargetGraveyardCardRecordManaValue`
            // step snapshotted (Surge to Victory's team +X/+0 pump); `0` if unset — unreachable in
            // practice, since a fizzled target drops the whole ability before this reads.
            Amount::ExiledCardManaValueThisWay => {
                self.surge_exiled_card.map_or(0, |(_, mv)| mv as i32)
            }
            // A placeholder [`fill_auras_attached_to_dying_creature`] must have already rewritten
            // to `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::AurasYouControlledAttachedToDyingCreature => panic!(
                "Amount::AurasYouControlledAttachedToDyingCreature must be contextualized to \
                 Fixed before resolving"
            ),
            Amount::IfSpellKicked { then, else_ } => {
                let amount = if self.spell_was_kicked(source) {
                    *then
                } else {
                    *else_
                };
                self.resolve_amount(amount, controller, source, target, x)
            }
            Amount::GreatestInstantOrSorceryManaValueCastThisTurn => {
                self.players[controller.0 as usize]
                    .greatest_instant_or_sorcery_mana_value_cast_this_turn as i32
            }
            Amount::OnePlusInstantsAndSorceriesCastThisTurn => {
                self.players[controller.0 as usize].instants_and_sorceries_cast_this_turn as i32 + 1
            }
            // CR 303.4: any Aura attached, regardless of controller — unlike
            // `AurasYouControlledAttachedToDyingCreature`, no controller filter and no death
            // involved (Kor Spiritdancer reads its own live attachments).
            Amount::AurasAttachedToSource => self
                .attachments(source)
                .into_iter()
                .filter(|&a| matches!(self.def_of(a).kind, CardKind::Aura))
                .count() as i32,
            Amount::InstantOrSorceryCardsInYourGraveyard => self
                .graveyard_cards(controller)
                .into_iter()
                .filter(|&id| matches!(self.def_of(id).kind, CardKind::Spell { .. }))
                .count() as i32,
            // ponytail: like `TriggeringSpellManaValue` above, a placeholder — `fill_combat_damage`
            // must have already rewritten it to `Fixed` with the batch's summed damage before the
            // watch's ability reaches the stack (see `queue_zero_base_power_combat_damage_triggers`),
            // so a live read here never happens for the pool. The arm exists only so this match
            // stays exhaustive.
            Amount::CombatDamageDealt => 0,
        }
    }

    /// [`resolve_amount`](Self::resolve_amount) clamped to a non-negative count (for draw / mill /
    /// token / counter effects, which can't take a negative quantity).
    pub(crate) fn resolve_count(
        &self,
        amount: Amount,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> u32 {
        self.resolve_amount(amount, controller, source, target, x)
            .max(0) as u32
    }

    /// How many permanents (battlefield) or cards (graveyard) match `filter`. On the battlefield
    /// this reuses [`Game::permanent_matches`]; in a graveyard only the type / controller / mana-
    /// value axes apply (a card in a graveyard isn't tapped, enchanted, or a token/nontoken).
    pub(crate) fn count_matching(
        &self,
        filter: &PermanentFilter,
        zone: AmountZone,
        controller: PlayerId,
        source: ObjectId,
    ) -> usize {
        match zone {
            AmountZone::Battlefield => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(filter, id, controller, Some(source)))
                .count(),
            AmountZone::Graveyard => self
                .objects
                .iter()
                .filter_map(|o| match o {
                    Object::Card(c) if c.zone == Zone::Graveyard => Some(c),
                    _ => None,
                })
                .filter(|c| self.graveyard_card_matches(filter, c, controller))
                .count(),
        }
    }

    /// Whether a graveyard card matches the type / controller / mana-value axes of `filter`.
    fn graveyard_card_matches(
        &self,
        filter: &PermanentFilter,
        card: &Card,
        controller: PlayerId,
    ) -> bool {
        if !filter.types.is_empty() && !filter.types.intersects(card.def.kind.types()) {
            return false;
        }
        let yours = card.owner == controller;
        match filter.controller {
            FilterController::You if !yours => return false,
            FilterController::Opponent if yours => return false,
            _ => {}
        }
        if let Some(max) = filter.mv_max
            && card.def.mana_value() > max as u32
        {
            return false;
        }
        true
    }

    /// How many creatures are on the battlefield in total (all controllers).
    pub(crate) fn creatures_on_battlefield(&self) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| self.is_creature_on_battlefield(id))
            .count()
    }

    /// How many creatures `player` controls on the battlefield.
    pub(crate) fn creatures_controlled(&self, player: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                self.owner_of(id) == player
                    && matches!(self.def_of(id).kind, CardKind::Creature { .. })
            })
            .count()
    }

    /// How many permanents `player` controls (CR 110.1 — every permanent type, tokens included;
    /// no filter). Feeds Ascend's "control ten or more permanents" state-based check (CR
    /// 702.131b).
    pub(crate) fn permanents_controlled(&self, player: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| self.controller_of(id) == player)
            .count()
    }

    /// Put queued triggers on the stack. A group with several abilities raises an
    /// ordering choice and stops; single-ability groups go straight on the stack.
    pub(crate) fn place_pending_triggers(&mut self, events: &mut Vec<Event>) {
        // Don't place triggers while blocked on an unrelated choice.
        if self.pending_choice.is_some() {
            return;
        }
        // Trigger doubling (CR 603.3c — Harmonic Prodigy / Veyran): before placement, an on-board
        // trigger-doubling static makes each matching triggered ability trigger an additional time.
        self.double_pending_triggers(events);
        // APNAP (CR 603.3b): waiting triggers go on the stack active-player-first, then each
        // other player in turn order; a controller then orders their own simultaneous batch.
        // Stable sort keeps one controller's groups in trigger order. (No current pool card
        // makes two controllers trigger off one event, so this is untested-but-correct.)
        let active = self.active_player.0;
        let n = self.players.len() as u8;
        self.pending_trigger_groups
            .sort_by_key(|g| (g.controller.0 + n - active) % n);
        while let Some(group) = self.pending_trigger_groups.first() {
            // A modal *triggered* ability (CR 700.2 extended to a trigger — Shadrix Silverquill's
            // begin-combat "you may choose two"): its several same-timing abilities are that one
            // ability's modes, not several simultaneous triggers to order, so it diverts to its
            // own choice instead of `OrderTriggers` below. Guarded to a source with no
            // `Timing::Spell` ability of its own, so a modal *spell*'s incidental unrelated
            // trigger (none in the pool today) isn't misread as one of its cast-time modes —
            // `modal` scopes a card's *one* set of modes, spell-timed or trigger-timed, never both.
            // A source that has already left the game entirely (a token's own Dies trigger, fired (CR 603.6, CR 111, CR 603)
            // off its vanishing) can't be modal — no pool token is a modal card — and `def_of`
            // panics on a fully-`Removed` object, so that case is excluded up front rather than
            // read.
            let modal_modes = (!matches!(self.objects[group.source as usize], Object::Removed))
                .then(|| self.def_of(group.source))
                .filter(|def| {
                    def.modal && !def.abilities.iter().any(|a| a.timing == Timing::Spell)
                });
            if let Some(def) = modal_modes {
                let group = self.pending_trigger_groups.remove(0);
                crate::pending::raise_choice(
                    self,
                    PendingChoice::ChooseTriggerModes {
                        player: group.controller,
                        source: group.source,
                        modes: group.abilities.iter().map(|a| a.effect).collect(),
                        choose: def.modal_choose,
                        // The "may" gate lives on each mode ability's own `optional` (CR "you may
                        // choose N" — every mode shares the same optional flag; read the first).
                        optional: group.abilities.first().is_some_and(|a| a.optional),
                    },
                );
                return;
            }
            if group.abilities.len() >= 2 {
                let group = self.pending_trigger_groups.remove(0);
                // ponytail: multi-ability groups order first; per-effect optional/target choice
                // for a group with several such triggers isn't wired yet (no such card).
                let effects = group.abilities.iter().map(|a| a.effect).collect();
                crate::pending::raise_choice(
                    self,
                    PendingChoice::OrderTriggers {
                        player: group.controller,
                        source: group.source,
                        effects,
                    },
                );
                return;
            }
            let group = self.pending_trigger_groups.remove(0);
            let ability = group.abilities[0];
            let (player, source, effect) = (group.controller, group.source, ability.effect);

            // "This ability triggers only once each turn" (Morbid Opportunist, Tocasia's
            // Welcome): counted at placement (when it triggers), not resolution, per CR — so the
            // check sits above the optional gate below. A source that already placed this
            // ability this turn is dropped silently (CR: it doesn't trigger again); the first
            // placement records itself immediately, so a second group for the same source later
            // in this same batch (the "one or more … enter/die" collapse) sees the record too.
            if ability.once_each_turn {
                if self.once_per_turn.triggered.contains(&source) {
                    continue;
                }
                self.push_apply(events, Event::TriggeredAbilityThisTurn { source });
            }

            // An optional trigger pauses for a yes/no (or pay-or-decline) before the stack.
            if ability.optional {
                crate::pending::raise_choice(
                    self,
                    if ability.cost == Cost::FREE {
                        PendingChoice::MayYesNo {
                            player,
                            source,
                            effect,
                        }
                    } else {
                        PendingChoice::PayCost {
                            player,
                            source,
                            cost: ability.cost,
                            effect,
                        }
                    },
                );
                return;
            }

            // Place the (non-optional) ability: pause to choose a target, put it straight on the
            // stack, or drop it if it targets with no legal target (CR 603.3c — continue the loop).
            match self.place_targeted_ability(player, source, effect, events) {
                Placement::Paused => return,
                Placement::Placed | Placement::NoLegalTarget => continue,
            }
        }

        // Echo (CR 702.31c/d): once the ordinary trigger queue is empty, offer one queued
        // pay-or-sacrifice choice at a time (a second, if any, follows once this one resolves —
        // the same "one at a time, chained across submits" shape `pending_trigger_groups` uses).
        // A source that left the battlefield since being queued (removed some other way in the
        // interim) is skipped with nothing to sacrifice.
        while let Some(source) = self.pending_echo.first().copied() {
            self.pending_echo.remove(0);
            if self.as_permanent(source).is_none() {
                continue;
            }
            let cost = self
                .def_of(source)
                .echo
                .expect("only queued for a permanent with an echo cost");
            crate::pending::raise_choice(
                self,
                PendingChoice::PayEchoOrSacrifice {
                    player: self.owner_of(source),
                    source,
                    cost,
                },
            );
            return;
        }
    }

    /// Trigger doubling (CR 603.3c — Harmonic Prodigy, Veyran, Voice of Duality): for each pending
    /// trigger group, count the on-board [`Effect::TriggerDoublingStatic`] doublers whose filter
    /// matches it and queue that many identical copies, so the ability triggers one additional time
    /// per doubler (two doublers → three instances total). Each copy is minted `expanded` so a copy
    /// is never itself re-doubled, and every considered group is marked `expanded` so a re-entrant
    /// placement pass (a pending choice answered mid-batch re-runs the whole post-intent pipeline)
    /// doesn't double the same trigger twice.
    fn double_pending_triggers(&mut self, events: &[Event]) {
        let mut copies: Vec<TriggerGroup> = Vec::new();
        for i in 0..self.pending_trigger_groups.len() {
            if self.pending_trigger_groups[i].expanded {
                continue;
            }
            self.pending_trigger_groups[i].expanded = true;
            let source = self.pending_trigger_groups[i].source;
            for _ in 0..self.matching_trigger_doublers(source, events) {
                let mut copy = self.pending_trigger_groups[i].clone();
                copy.expanded = true;
                copies.push(copy);
            }
        }
        // Appended after all originals: the stable APNAP sort in `place_pending_triggers` keeps
        // each copy adjacent to its original (same controller), a controller's own simultaneous
        // instances they then order among themselves (CR 603.3b).
        self.pending_trigger_groups.extend(copies);
    }

    /// How many on-board [`Effect::TriggerDoublingStatic`] doublers make a triggered ability whose
    /// source permanent is `source` trigger an additional time (CR 603.3c). Both example cards
    /// double a triggered ability of a *permanent* (Harmonic: a Shaman/Wizard; Veyran: any
    /// permanent you control), so a non-permanent source (a spell's own cast trigger, a removed
    /// object) never matches.
    fn matching_trigger_doublers(&self, source: ObjectId, events: &[Event]) -> usize {
        // "of a permanent you control" / "of a Shaman or another Wizard you control": a spell on
        // the stack or a fully-removed object is not a permanent (a just-died creature's card,
        // still readable for its last-known subtypes/controller, is).
        if !matches!(
            self.objects[source as usize],
            Object::Permanent(_) | Object::Card(_)
        ) {
            return 0;
        }
        let source_controller = self.controller_of(source);
        let source_subtypes_on = self.def_of(source).subtypes;
        let mut count = 0;
        for doubler in self.battlefield() {
            for ability in self.functional_abilities(doubler) {
                let (
                    Timing::Static,
                    Effect::TriggerDoublingStatic {
                        source_subtypes,
                        source_other,
                        caused_by_instant_or_sorcery_cast,
                    },
                ) = (ability.timing, ability.effect)
                else {
                    continue;
                };
                // "you control" — the triggering ability's source permanent is controlled by the
                // doubler's controller.
                if self.controller_of(doubler) != source_controller {
                    continue;
                }
                // "another" (CR): the doubler doesn't double its own source permanent's triggers.
                if source_other && source == doubler {
                    continue;
                }
                // Subtype gate on the source permanent (Harmonic's Shaman/Wizard).
                if !source_subtypes.is_empty()
                    && !source_subtypes
                        .iter()
                        .any(|s| source_subtypes_on.contains(s))
                {
                    continue;
                }
                // ponytail: Veyran's "you casting or copying an instant/sorcery *causes*" is
                // approximated as "the same event batch holds the doubler-controller's instant or
                // sorcery cast/copy" — the engine has no per-trigger causal link. Exact for the
                // pool: a magecraft-style trigger is always placed in the same batch as its cause.
                if caused_by_instant_or_sorcery_cast
                    && !self.batch_has_instant_or_sorcery_cast_or_copy(source_controller, events)
                {
                    continue;
                }
                count += 1;
            }
        }
        count
    }

    /// Whether `events` (this placement batch) holds an instant or sorcery cast or copy by
    /// `player` — Veyran's magecraft cause (CR 603.3c). A spell copy is always an instant/sorcery,
    /// but the kind check keeps this honest for any future copy of a non-spell object.
    fn batch_has_instant_or_sorcery_cast_or_copy(
        &self,
        player: PlayerId,
        events: &[Event],
    ) -> bool {
        events.iter().any(|event| match *event {
            Event::SpellCast {
                spell, controller, ..
            } => controller == player && matches!(self.def_of(spell).kind, CardKind::Spell { .. }),
            Event::SpellCopied {
                copy, controller, ..
            } => controller == player && matches!(self.def_of(copy).kind, CardKind::Spell { .. }),
            _ => false,
        })
    }

    /// Place one resolved (past its optional gate) ability. If it targets, pause on a
    /// [`PendingChoice::ChooseTarget`] ([`Placement::Paused`]) — unless there's no legal target
    /// ([`Placement::NoLegalTarget`], CR 603.3c); if it's targetless, push it onto the stack
    /// ([`Placement::Placed`]). Shared by the trigger-placement loop and the optional accept paths.
    ///
    /// (No source colors: a targeted ability's source color isn't wired, so protection doesn't
    /// filter its targets — same scoping as `legal_targets`.)
    pub(crate) fn place_targeted_ability(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        effect: Effect,
        events: &mut Vec<Event>,
    ) -> Placement {
        let spec = effect.target();
        if spec == TargetSpec::None {
            self.push_ability_group(player, source, &[(effect, None)], events);
            return Placement::Placed;
        }
        let x = self.ability_source_x(source);
        // `ThisPermanent`/`EnchantedCreature`/`ThisAurasGraveyardTarget` are a fixed reference,
        // not a real choice (CR: these abilities never say "target", or — Animate Dead — the
        // choice already happened at cast) — resolve straight to the stack with no pause.
        if matches!(
            spec,
            TargetSpec::ThisPermanent
                | TargetSpec::EnchantedCreature
                | TargetSpec::ThisAurasGraveyardTarget
        ) {
            let legal = self.legal_targets_for(spec, source, player, [false; Color::COUNT], x);
            let Some(&fixed) = legal.first() else {
                return Placement::NoLegalTarget;
            };
            self.push_ability_group(player, source, &[(effect, Some(fixed))], events);
            return Placement::Placed;
        }
        let legal = self.legal_targets_for(spec, source, player, [false; Color::COUNT], x);
        if legal.is_empty() {
            // CR 603.3c drops a *mandatory*-target ability with no legal target. "Up to one"
            // (min 0) isn't mandatory — CR 601.2c already treats choosing zero of "up to N" as a
            // complete, legal choice — so it still goes on the stack with no target chosen, but
            // only when doing so accomplishes something (`has_target_independent_step`): Kinetic
            // Ooze's destroy-nothing still lets its X-threshold riders run, while Killian's
            // tap-and-goad (every step needs the same missing target) drops outright exactly as
            // before — parking a pure no-op on the stack would only add noise.
            if effect.target_count().min == 0 && effect.has_target_independent_step() {
                return self.place_ability_second_clause(player, source, effect, None, events);
            }
            return Placement::NoLegalTarget;
        }
        crate::pending::raise_choice(
            self,
            PendingChoice::ChooseTarget {
                player,
                source,
                effect,
                legal,
                optional: effect.target_count().min == 0,
            },
        );
        Placement::Paused
    }

    /// A triggered ability's *second* independent target clause (CR 603.3d — an ability may target
    /// more than once, each clause chosen as the trigger goes on the stack), or `None` when the
    /// ability targets at most once (or its gated second clause is off right now). Clause 0 is the
    /// ability's shared target ([`Effect::target`], read from `ctx.target` at resolution); the second
    /// clause is the step that reads its own `targets_second` list
    /// ([`Effect::reads_second_target_clause`] — Kinetic Ooze's X≥10 doubling), *not* a `Sequence`
    /// step that merely shares the one chosen target (Killian's goad). Walks the `Sequence` in
    /// printed order, honoring each intervening-if gate (CR 603.4) evaluated now at placement.
    fn ability_second_target_clause(
        &self,
        effect: Effect,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<(TargetSpec, TargetCount)> {
        let Effect::Sequence { steps } = effect else {
            return None;
        };
        self.second_clause_in(steps, source, controller)
    }

    /// The first second-clause step among `steps` (recursing a gate's `then`), or `None`. See
    /// [`Self::ability_second_target_clause`].
    fn second_clause_in(
        &self,
        steps: &[Effect],
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<(TargetSpec, TargetCount)> {
        for &step in steps {
            if let Effect::Conditional { condition, then } = step {
                // CR 603.4: a gated clause is only a real target clause when its intervening-if
                // holds as the trigger goes on the stack.
                if self.placement_condition_holds(condition, source, controller)
                    && let Some(found) = self.second_clause_in(then, source, controller)
                {
                    return Some(found);
                }
                continue;
            }
            if step.reads_second_target_clause() {
                return Some((step.target(), step.target_count()));
            }
        }
        None
    }

    /// Whether an intervening-if `condition` holds as a trigger goes on the stack (CR 603.4).
    /// ponytail: the only multi-clause gate in the pool is Kinetic Ooze's source-X threshold, so
    /// that's special-cased (`TriggerContext` carries no source id); every other condition falls
    /// through to the live board-state evaluator. A target-based gate (`TargetPowerAtLeast`) can't
    /// be judged before targets exist and no card needs one for a second clause.
    fn placement_condition_holds(
        &self,
        condition: Condition,
        source: ObjectId,
        controller: PlayerId,
    ) -> bool {
        match condition {
            Condition::SourceEnteredWithXAtLeast { at_least } => {
                self.ability_source_x(source) >= at_least
            }
            _ => self.condition_holds(condition, TriggerContext::of(controller)),
        }
    }

    /// After a triggered ability's first target clause is settled with `first` (its chosen target,
    /// or `None`), choose its *second* independent target clause too (CR 603.3d — Kinetic Ooze's
    /// X≥10 "double ... any number of other target creatures") before the ability goes on the stack,
    /// then push the assembled ability. An ability with only one target clause pushes immediately
    /// (empty `targets_second`). Pauses on a [`PendingChoice::ChooseAbilityTargets`] when the second
    /// clause is a real choice (more than one legal set); auto-fills when it's forced.
    pub(crate) fn place_ability_second_clause(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        effect: Effect,
        first: Option<Target>,
        events: &mut Vec<Event>,
    ) -> Placement {
        let Some((spec, count)) = self.ability_second_target_clause(effect, source, player) else {
            self.push_ability_with_targets(
                player,
                source,
                effect,
                first,
                TargetList::default(),
                events,
            );
            return Placement::Placed;
        };
        let x = self.ability_source_x(source);
        let legal = self.legal_targets_for(spec, source, player, [false; Color::COUNT], x);
        let n = legal.len();
        let lo = (count.min as usize).min(n) as u8;
        let hi = (count.max as usize).min(n) as u8;
        // Forced: exactly one legal set — take all `n` (the empty set when none are legal, an "any
        // number" of zero). No pause. Otherwise the controller chooses (CR 601.2c).
        if lo == hi && hi as usize == n {
            self.push_ability_with_targets(
                player,
                source,
                effect,
                first,
                TargetList::from_targets(&legal),
                events,
            );
            return Placement::Placed;
        }
        crate::pending::raise_choice(
            self,
            PendingChoice::ChooseAbilityTargets {
                player,
                source,
                effect,
                target: first,
                min: lo,
                max: hi,
                legal,
            },
        );
        Placement::Paused
    }

    /// Push triggered/activated abilities onto the stack, in the given order (each with its
    /// chosen target, or `None`), and hand priority to the active player.
    pub(crate) fn push_ability_group(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        abilities: &[(Effect, Option<Target>)],
        events: &mut Vec<Event>,
    ) {
        // Triggered abilities carry no `{X}` — an activated (or copied) `{X}` ability goes on the
        // stack via `push_activated_ability` instead, which threads its chosen X.
        self.push_ability_group_with_x(controller, source, abilities, 0, events);
    }

    /// [`push_ability_group`](Self::push_ability_group) threading a chosen `{X}` (CR 107.3) onto
    /// each ability — for an activated ability whose cost contains `{X}`, or a CR 707.10c copy of
    /// one, whose `Amount::X` reads that value at resolution.
    pub(crate) fn push_ability_group_with_x(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        abilities: &[(Effect, Option<Target>)],
        x: u32,
        events: &mut Vec<Event>,
    ) {
        for &(effect, target) in abilities {
            self.push_apply(
                events,
                Event::TriggeredAbilityOnStack {
                    controller,
                    source,
                    effect,
                    target,
                    targets_second: TargetList::default(),
                    x,
                },
            );
        }
        self.consecutive_passes = 0;
        self.priority = self.active_player;
    }

    /// Put a single triggered ability on the stack carrying both its first-clause `target` and its
    /// `targets_second` (a second independent target clause's chosen targets, CR 603.3d — Kinetic
    /// Ooze's X≥10 doubling rider). The ubiquitous single-clause callers go through
    /// [`push_ability_group`](Self::push_ability_group) (empty `targets_second`).
    pub(crate) fn push_ability_with_targets(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        effect: Effect,
        target: Option<Target>,
        targets_second: TargetList,
        events: &mut Vec<Event>,
    ) {
        self.push_apply(
            events,
            Event::TriggeredAbilityOnStack {
                controller,
                source,
                effect,
                target,
                targets_second,
                x: 0,
            },
        );
        self.consecutive_passes = 0;
        self.priority = self.active_player;
    }
}

/// Whether `n` is prime (Zimone, All-Questioning's "you control a prime number of lands" —
/// CR's own printed reminder: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31). `0`/`1` are not prime.
/// ponytail: trial division, O(sqrt n) — ample for a land count no real board ever approaches;
/// reach for a sieve only if a future card checks primality over a much larger domain.
fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    (2..=n.isqrt()).all(|d| !n.is_multiple_of(d))
}

/// Whether a sacrificed permanent (`def`, sacrificed by leaving the battlefield as `watcher`
/// looks on) matches a sacrifice-trigger's `filter` (CR 701.20). Only the type and "another
/// permanent" (`other`) axes apply — the permanent has already left the battlefield by the time
/// this runs, so the battlefield-only axes (`tapped`, `enchanted`, `mv_max`, `token`) never
/// distinguish a card that's already gone.
/// ponytail: type + other only — every pool sacrifice-trigger filter (Smothering Abomination's
/// "creature", Mazirek's "another permanent") needs just those two axes; widen when a card needs
/// mv_max/token/enchanted on a sacrifice watch.
fn sacrifice_matches(
    filter: &PermanentFilter,
    def: CardDef,
    watcher: ObjectId,
    sacrificed: ObjectId,
) -> bool {
    if filter.other && watcher == sacrificed {
        return false;
    }
    filter.types.is_empty() || filter.types.intersects(def.kind.types())
}

/// Whether a [`state::DestroyedThisWay`] snapshot matches `filter`, relative to `you` (the
/// resolving effect's controller) — the snapshot-data sibling of [`Game::permanent_matches`],
/// for [`Amount::PermanentsDestroyedThisWay`] counting permanents already off the battlefield.
/// ponytail: only the types/subtypes/controller/token axes — the pool's two cards (Ceaseless
/// Conflict's "nontoken creature you controlled", Culling Ritual's unfiltered count) need no
/// more; a destroyed permanent's snapshot has no live tapped/mv/power context to widen into.
fn destroyed_this_way_matches(
    filter: &PermanentFilter,
    you: PlayerId,
    snap: &state::DestroyedThisWay,
) -> bool {
    if !filter.types.is_empty() && !filter.types.intersects(snap.def.kind.types()) {
        return false;
    }
    if !filter.subtypes.is_empty()
        && !filter
            .subtypes
            .iter()
            .any(|s| snap.def.subtypes.contains(s))
    {
        return false;
    }
    match filter.controller {
        FilterController::Any => {}
        FilterController::You if snap.controller != you => return false,
        FilterController::Opponent if snap.controller == you => return false,
        _ => {}
    }
    match filter.token {
        TokenFilter::Any => {}
        TokenFilter::Token if !snap.token => return false,
        TokenFilter::Nontoken if snap.token => return false,
        _ => {}
    }
    true
}

/// Whether an ability carries Backup's grant (CR 702.166b) — used to exclude the granting ability
/// itself from the set a Backup source hands to another creature ("it gains the following
/// abilities", i.e. the source's *other* abilities). Looks one level into a [`Effect::Sequence`],
/// the shape a Backup ETB uses (counter + grant).
fn ability_grants_source_abilities(ability: Ability) -> bool {
    match ability.effect {
        Effect::GrantSourceAbilitiesUntilEndOfTurn => true,
        Effect::Sequence { steps } => steps
            .iter()
            .any(|s| matches!(s, Effect::GrantSourceAbilitiesUntilEndOfTurn)),
        _ => false,
    }
}
