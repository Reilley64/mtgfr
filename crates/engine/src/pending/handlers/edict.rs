//! Edict / devour / copy / sacrifice fan-out answers.

use crate::*;

impl Game {
    pub(crate) fn apnap_order(&self) -> Vec<PlayerId> {
        self.turn_order_from(self.active_player)
    }

    /// Every living player in turn order starting with `first` — a "starting with you" round
    /// (CR 101.4: council's dilemma, join forces), which is [`Self::apnap_order`] when `first` is
    /// the active player.
    pub(crate) fn turn_order_from(&self, first: PlayerId) -> Vec<PlayerId> {
        let n = self.players.len();
        let start = first.0 as usize;
        (0..n)
            .map(|i| PlayerId(((start + i) % n) as u8))
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
    pub(crate) fn sacrifice_ids(
        &mut self,
        ids: &[ObjectId],
        by: PlayerId,
        events: &mut Vec<Event>,
    ) {
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

    /// Resolve a multi-player sacrifice edict ([`Effect::Choice(ChoiceEffect::EachPlayerSacrifices)`]): each affected
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
        self.resolution_frame.sacrificed_by_edict_controller = false;
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
            self.resolution_frame.sacrificed_by_edict_controller = true;
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
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextCasterKeep {
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
        // reachable (the planeswalker slot is dropped, see `CasterKeepsOneOfEachTypePerPlayer`),
        // so small brute force suffices.
        let slots = [TypeSet::ARTIFACT, TypeSet::CREATURE, TypeSet::ENCHANTMENT];
        let keep_masks: Vec<TypeSet> = keeps
            .iter()
            .map(|&id| self.def_of(id).kind.types())
            .collect();
        if !super::assign_to_distinct_slots(&keep_masks, &slots, 0) {
            return false;
        }
        let option_masks: Vec<TypeSet> = options
            .iter()
            .map(|&id| self.def_of(id).kind.types())
            .collect();
        keeps.len() == super::max_distinct_slots(&option_masks, &slots)
    }
}
