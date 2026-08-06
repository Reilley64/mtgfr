//! Vote / keep / counter-target / discard / decline-untap answers.

use crate::*;

impl Game {
    pub(crate) fn prompt_next_counter_target(
        &mut self,
        remaining: Vec<PlayerId>,
        chooser: PlayerId,
        source: ObjectId,
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextCounterTarget {
                remaining,
                chooser,
                source,
            },
        );
    }

    /// Pause on the next graveyard still owed a card (skipping any with no matching card in it),
    /// or — when none remain — return, letting the enclosing resolution finish. Serves all three
    /// shapes: one graveyard (Deadly Brew's single return), the same graveyard repeated (Recall's
    /// "a card … for each card discarded this way"), and one per dying creature's owner (Glyph of
    /// Reincarnation).
    pub(crate) fn prompt_next_graveyard_return(
        &mut self,
        graveyards: Vec<PlayerId>,
        chooser: PlayerId,
        source: ObjectId,
        filter: CardFilter,
        mandatory: bool,
        to_battlefield: bool,
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::MayReturnFromGraveyard {
                player: chooser,
                source,
                filter,
                mandatory,
                to_battlefield,
                graveyards,
            },
        );
    }

    /// Answer a [`PendingChoice::ChooseCounterTargetForPlayer`]: `chosen` is the up-to-one creature
    /// the chooser counters for `target_player` (empty declines — CR 603.3d). Put one +1/+1 counter
    /// on it through the replacement pipeline [`Effect::Counters(CountersEffect::PutCounters)`] uses, then advance to the next
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
            let n = self.counters_after_replacements(chooser, object, 1);
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
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextGraveyardExile { remaining, source },
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
            self.resolution_frame.nonland_cards_exiled_this_way += 1;
        }
        let card = self.next_object_id();
        self.push_apply(&mut events, Event::MovedToExile { card, from: id });
        self.prompt_next_graveyard_exile(remaining, source);
        Ok(events)
    }

    /// Pause on the next opponent with a card to discard (skipping any with an empty hand), or —
    /// when none remain — return, letting the enclosing sequence resume into the draw payoff.
    pub(crate) fn prompt_next_discard_edict(
        &mut self,
        remaining: Vec<PlayerId>,
        source: ObjectId,
        floor: Option<u32>,
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextDiscardEdict {
                remaining,
                source,
                floor,
            },
        );
    }

    /// Answer a [`PendingChoice::DiscardEdict`]: discard the one chosen hand card (Syphon Mind's
    /// "Each other player discards a card"), tallying it into
    /// [`ResolutionFrame::cards_discarded_this_way`](crate::resolution::ResolutionFrame), then move
    /// on to the next opponent — the discard twin of [`Self::choose_graveyard_exile`].
    pub(crate) fn answer_discard_edict(
        &mut self,
        player: PlayerId,
        discards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DiscardEdict {
            options,
            remaining,
            source,
            count,
            floor,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // Mandatory: exactly `count` distinct offered cards (declining isn't legal when they have
        // them). One for a plain fan-out; under Balance, their whole excess in one answer.
        let distinct = discards
            .iter()
            .enumerate()
            .all(|(i, id)| !discards[..i].contains(id));
        if discards.len() as u32 != count
            || !distinct
            || discards.iter().any(|id| !options.contains(id))
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.discard_ids(&discards, player, &mut events);
        self.resolution_frame.cards_discarded_this_way += discards.len() as u32;
        self.prompt_next_discard_edict(remaining, source, floor);
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
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextVote {
                remaining,
                source,
                options,
            },
        );
    }

    /// Pause on the next seat in a join-forces payment round, or — when none remain — return,
    /// letting the enclosing sequence resume.
    pub(crate) fn prompt_next_join_forces_payment(
        &mut self,
        remaining: Vec<PlayerId>,
        source: ObjectId,
        prevent_up_to: Option<u8>,
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextJoinForcesPayment {
                remaining,
                source,
                prevent_up_to,
            },
        );
    }

    /// Answer a [`PendingChoice::JoinForcesPayment`]: pay `x` mana into the round's total, or
    /// decline (`pay: false`) and add nothing. An unaffordable amount leaves the choice pending
    /// with nothing spent, so the payer can answer again with less.
    pub(crate) fn answer_join_forces_payment(
        &mut self,
        player: PlayerId,
        pay: bool,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::JoinForcesPayment {
            player: payer,
            source,
            remaining,
            prevent_up_to,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != payer {
            return Err(Reject::NotYourPriority);
        }
        // "Any amount of mana" is paid as that much generic (CR 202.2 — generic accepts any type).
        let amount = if pay { x } else { 0 };
        if amount > u8::MAX as u32 {
            return Err(Reject::CannotPayCost);
        }

        let mut events = Vec::new();
        if amount > 0 {
            let cost = Cost {
                generic: amount as u8,
                ..Default::default()
            };
            self.settle_payment(player, cost, None, None, &mut events)?;
        }
        self.finish_answer();
        self.resolution_frame.join_forces_mana += amount;
        // Power Leak: "Prevent X of that damage, where X is the amount of mana that player paid
        // this way" (CR 615). The shield goes up here, between the payment and the damage step
        // waiting in the enclosing `Sequence`, and is capped at that step's damage so overpaying
        // banks nothing against an unrelated hit later in the turn.
        if let Some(cap) = prevent_up_to {
            let points = amount.min(u32::from(cap)) as i32;
            if points > 0 {
                self.damage_prevention_shields
                    .push(crate::state::PreventionShield {
                        target: crate::Target::Player(payer),
                        amount: Some(points),
                        keep: None,
                        from_color: crate::ColorFilter::Any,
                        from_source: None,
                        any_recipient: false,
                        combat_only: false,
                        from_filter: None,
                        from_relation: None,
                        persistent: false,
                        gain_life: false,
                        redirect_to: None,
                    });
            }
        }
        self.prompt_next_join_forces_payment(remaining, source, prevent_up_to);
        Ok(events)
    }

    /// Answer a [`PendingChoice::CastVote`]: `choice` is the index into the ballot's `options`.
    /// A council's-dilemma ballot (`["past", "present"]`) tallies the vote; Archangel of Strife's
    /// war/peace ballot instead records the voter's own answer against the asking permanent on
    /// `Player::war_choices`. Either way, move on to the next player.
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

        // ponytail: ballots hardcoded to the pool's two voting cards. Generalize to a
        // label→outcome map when a third, differently-balloted voting card lands.
        match ballot {
            "past" => self.resolution_frame.council_past_votes += 1,
            "present" => self.resolution_frame.council_present_votes += 1,
            "war" | "peace" => {
                self.players[voter.0 as usize]
                    .war_choices
                    .push((source, ballot == "war"));
                // A `war_choice`-gated anthem just started/stopped applying to every creature
                // this voter owns — same scope as `Event::CitysBlessingGained`'s invalidation.
                self.characteristics_cache
                    .write(|cache| cache.invalidate_owner(self, voter));
            }
            other => panic!("unknown vote ballot {other:?}"),
        }
        self.prompt_next_vote(remaining, source, options);
        Ok(Vec::new())
    }

    /// Pause on the next seat in Conundrum Sphinx's name-a-card fan-out, or — when none remain —
    /// return, letting the enclosing sequence resume. Naming is mandatory (CR 201.2), so unlike a
    /// graveyard fan-out no seat is ever skipped.
    pub(crate) fn prompt_next_card_name(
        &mut self,
        remaining: Vec<PlayerId>,
        source: ObjectId,
        use_: CardNameUse,
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextCardName {
                remaining,
                source,
                use_,
            },
        );
    }

    /// Answer a [`PendingChoice::ChooseCardName`] (Conundrum Sphinx's attack trigger — CR
    /// 201.2/703.2j "choose a card name"): `name` is the freely chosen card name, only checked
    /// for shape (trimmed non-empty, bounded length) at this trust boundary — never validated
    /// against any real card list (CR 201.3 lets a player name a nonexistent card). Reveals the
    /// answering player's own top library card and resolves the match immediately: a name match
    /// puts it into their hand, a miss puts it on the bottom of their library (CR 201.2/703.2j) —
    /// before advancing to the next seat. An empty library reveals nothing, so naming still
    /// consumes the seat but nothing moves.
    pub(crate) fn answer_choose_card_name(
        &mut self,
        player: PlayerId,
        name: String,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCardName {
            player: chooser,
            source,
            remaining,
            use_,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if player != chooser {
            return Err(Reject::NotYourPriority);
        }
        // Trust boundary: bounded, non-blank shape only (CR 201.2 — a real name is never blank);
        // the longest printed card name to date is well under this bound.
        let chosen = name.trim();
        if chosen.is_empty() || chosen.chars().count() > 200 {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        match use_ {
            CardNameUse::RevealTopOfOwnLibrary { miss_to_graveyard } => {
                self.reveal_top_against_name(player, chosen, miss_to_graveyard, &mut events);
            }
            CardNameUse::SubjectRevealsHandAtRandomThenDiscards { subject, count } => {
                self.reveal_hand_at_random_then_discard_named(subject, count, chosen, &mut events);
            }
        }
        self.prompt_next_card_name(remaining, source, use_);
        Ok(events)
    }

    /// The Sphinxes' half of a named card (CR 201.2/703.2j): reveal `player`'s own top library
    /// card; a name match goes to their hand, a miss to their graveyard (Petra Sphinx) or the
    /// bottom of their library (Conundrum Sphinx). An empty library reveals nothing.
    fn reveal_top_against_name(
        &mut self,
        player: PlayerId,
        chosen: &str,
        miss_to_graveyard: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(&card) = self.players[player.0 as usize].library.first() else {
            return;
        };
        let def = self.def_id_of(card);
        let printed = card_def(def);
        self.push_apply(events, Event::RevealedTopOfLibrary { player, card, def });
        if printed.name == chosen {
            self.push_apply(
                events,
                Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from: card,
                    card: def,
                },
            );
            return;
        }
        if miss_to_graveyard {
            self.push_apply(
                events,
                Event::Milled {
                    player,
                    card: self.next_object_id(),
                    from: card,
                },
            );
            return;
        }
        self.push_apply(events, Event::PutOnBottomOfLibrary { player, card });
    }

    /// Nebuchadnezzar's half: `subject` reveals `count` cards at random from their hand (CR 701.30
    /// — the injected per-op RNG, so a replay reveals the same cards), then discards every card
    /// revealed this way whose name is `chosen`. Revealing more than the hand holds simply reveals
    /// the whole hand.
    fn reveal_hand_at_random_then_discard_named(
        &mut self,
        subject: PlayerId,
        count: u32,
        chosen: &str,
        events: &mut Vec<Event>,
    ) {
        let mut hand = self.hand_of(subject);
        let mut revealed = Vec::new();
        for _ in 0..(count as usize).min(hand.len()) {
            let idx = self.with_op_rng(subject, |rng| rng.gen_index(hand.len()));
            revealed.push(hand.swap_remove(idx));
        }
        let mut named = Vec::new();
        for card in revealed {
            let def = self.def_id_of(card);
            self.push_apply(
                events,
                Event::RevealedFromHand {
                    player: subject,
                    card,
                    def,
                },
            );
            if card_def(def).name == chosen {
                named.push(card);
            }
        }
        // The shared discard path, so discard watchers, madness and Containment Construct see
        // these exactly as they see a chosen pitch.
        self.discard_ids(&named, subject, events);
    }

    /// Answer a [`PendingChoice::MaySacrifice`]: `sacrifices` is empty to decline, or names
    /// exactly `count` distinct permanents (from the choice's `options`) sacrificed to gain
    /// `then`'s effects (CR 601.2f-style "you may … if you do"). Declining runs `otherwise` —
    /// Mold Demon's "sacrifice it unless you sacrifice two Swamps" — which is empty for the plain
    /// no-penalty shape.
    pub(crate) fn answer_may_sacrifice(
        &mut self,
        player: PlayerId,
        sacrifices: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MaySacrifice {
            source,
            options,
            count,
            then,
            otherwise,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // "Two Swamps" is a price, not a maximum: pay it in full or not at all. Each named
        // permanent must be a distinct offered one — the same Swamp twice isn't two Swamps.
        let paid = !sacrifices.is_empty();
        if (paid && sacrifices.len() != count as usize)
            || sacrifices.iter().any(|id| !options.contains(id))
            || (1..sacrifices.len()).any(|i| sacrifices[i..].contains(&sacrifices[i - 1]))
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &sacrifices {
            let def = self.def_id_of(id);
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
        let ctx = ResolveCtx {
            controller: player,
            source,
            target: None,
            targets_second: TargetList::default(),
            x: 0,
            spent_mana: [0; 6],
        };
        // Paid → the "if you do" rider; declined → the price of declining ("unless you sacrifice
        // two Swamps, sacrifice it"), which is empty for every card that only offers upside.
        let branch = if paid { then } else { otherwise };
        self.run_sequence(branch, ctx, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayPutCounterOnCreature`]: `choice` is `None` to decline, or one
    /// of the choice's `options` (a battlefield creature) to put a single +1/+1 counter on
    /// (Zimone's Hypothesis' primer, CR 601.2c). Non-targeted, so a `Some` id must be a currently
    /// offered creature; the enclosing `Sequence`'s next step (the parity bounce) runs regardless,
    /// resumed by [`Game::resume_deferred_sequence`] after this returns.
    pub(crate) fn answer_may_put_counter_on_creature(
        &mut self,
        _player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayPutCounterOnCreature {
            source, options, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|id| !options.contains(&id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(object) = choice {
            let n = self.counters_after_replacements(_player, object, 1);
            if n > 0 {
                self.push_apply(
                    &mut events,
                    Event::CountersPlaced {
                        object,
                        count: n,
                        source_name: self.source_name_of(source),
                    },
                );
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseBlockTarget`]: `choice` is `None` to decline the "you may",
    /// or one of the choice's `options` (a declared attacker) for the pulled creature to block
    /// (False Orders, CR 601.2c). The re-aimed block is a real block declaration — the same
    /// [`Event::BlockerDeclared`] the declare-blockers step emits — so it fires the "blocks" /
    /// "becomes blocked" watches and makes the attacker blocked for the rest of combat.
    pub(crate) fn answer_choose_block_target(
        &mut self,
        _player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseBlockTarget {
            blocker, options, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|id| !options.contains(&id)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(attacker) = choice else {
            return Ok(events);
        };
        self.push_apply(&mut events, Event::BlockerDeclared { blocker, attacker });
        let blocks = [(blocker, attacker)];
        self.queue_blocks_or_becomes_blocked_triggers(&blocks);
        self.queue_blocks_or_becomes_blocked_by_triggers(&blocks);
        self.queue_attacks_or_blocks_block_triggers(&blocks);
        self.queue_rampage_triggers(&blocks);
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayReturnFromGraveyard`]: `choice` is empty to decline, or names
    /// the one graveyard card (one of the choice's `options`) returned to `player`'s hand
    /// ([`Effect::Choice(ChoiceEffect::MayReturnFromGraveyard)`] — Deadly Brew's rider).
    pub(crate) fn answer_may_return_from_graveyard(
        &mut self,
        _player: PlayerId,
        choice: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayReturnFromGraveyard {
            player: chooser,
            source,
            options,
            filter,
            mandatory,
            to_battlefield,
            then_graveyards,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.len() > 1 || choice.iter().any(|id| !options.contains(id)) {
            return Err(Reject::IllegalChoice);
        }
        // "you return" (mandatory): a legal card must be chosen — declining is illegal (CR 700.2).
        if mandatory && choice.is_empty() {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &id in &choice {
            // Glyph of Reincarnation puts it onto the battlefield "under its owner's control";
            // every other user of this variant returns it to the chooser's hand.
            let event = match to_battlefield {
                true => self.reanimate_event(id, self.owner_of(id), false),
                false => Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: id,
                },
            };
            self.push_apply(&mut events, event);
        }
        if !then_graveyards.is_empty() {
            self.prompt_next_graveyard_return(
                then_graveyards,
                chooser,
                source,
                filter,
                mandatory,
                to_battlefield,
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayExileDiscardedToPlay`]: `choice` is empty to decline, or names
    /// the one discarded nonland card (one of the choice's `options`) exiled from `player`'s
    /// graveyard face-up with impulse-play permission
    /// ([`Effect::Choice(ChoiceEffect::MayExileDiscardedNonlandMayPlay)`] — Conspiracy Theorist).
    /// The impulse-play twin of [`Self::answer_may_return_from_graveyard`] — minting the same
    /// [`Event::ExiledFromGraveyardMayPlay`] as [`MillEffect::ExileFromGraveyardMayPlay`].
    pub(crate) fn answer_may_exile_discarded_to_play(
        &mut self,
        _player: PlayerId,
        choice: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayExileDiscardedToPlay {
            player, options, ..
        }) = self.pending_choice.clone()
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
                Event::ExiledFromGraveyardMayPlay {
                    player,
                    card: self.next_object_id(),
                    from: id,
                },
            );
        }
        Ok(events)
    }

    /// Answer a discard choice — either a cleanup [`PendingChoice::DiscardToHandSize`] or an
    /// [`Effect::Choice(ChoiceEffect::Discard)`]'s [`PendingChoice::DiscardCards`]: move the chosen cards to the
    /// graveyard. A cleanup discard then resumes the interrupted step-transition (carrying the turn
    /// to the next player); an effect discard leaves any deferred sequence tail for
    /// [`Game::resume_deferred_sequence`].
    pub(crate) fn answer_discard(
        &mut self,
        player: PlayerId,
        cards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let (chooser, hand, count, or_one_matching, is_cleanup, draw_replacement) =
            match self.pending_choice.clone() {
                Some(PendingChoice::DiscardToHandSize {
                    player,
                    hand,
                    count,
                }) => (player, hand, count, None, true, false),
                Some(PendingChoice::DiscardCards {
                    player,
                    hand,
                    count,
                    or_one_matching,
                    draw_replacement,
                }) => (
                    player,
                    hand,
                    count,
                    or_one_matching,
                    false,
                    draw_replacement,
                ),
                _ => return Err(Reject::IllegalChoice),
            };
        // Exactly `count` distinct cards, each currently in this player's hand — or, when the
        // effect carries a land-escape-valve filter, a single matching card instead (Compulsive
        // Research's "unless they discard a land card").
        let distinct = cards.iter().collect::<std::collections::HashSet<_>>().len();
        let all_in_hand = cards.iter().all(|c| hand.contains(c));
        let full_discard = cards.len() == count && distinct == cards.len();
        let land_escape = or_one_matching
            .is_some_and(|filter| cards.len() == 1 && filter.matches(&self.def_of(cards[0])));
        if player != chooser || !all_in_hand || !(full_discard || land_escape) {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        // CR 701.8: every discard fires "whenever you discard" watchers — a cleanup hand-size
        // trim counts exactly the same as an effect discard. But the cleanup trim is a turn-based
        // action (CR 514.1), not something a spell or ability *caused*, and it runs with whatever
        // `resolve_top` last armed still in the frame — so clear the cause before it discards.
        if is_cleanup {
            self.resolution_frame.discard_cause = None;
        }
        // Chains of Mephistopheles' substituted discard runs outside any resolution too, and the
        // frame still holds whatever asked for the *draw* it replaced. The discard is caused by
        // Chains' own static ability (CR 614), so Psychic Purge's "a spell or ability an opponent
        // controls causes you to discard this" reads Chains' controller, not the drawing spell's.
        // Restored afterwards: the resolution this interrupted may discard again on its own account.
        let stale_cause = draw_replacement.then(|| {
            let chains = self.chains_controller();
            std::mem::replace(&mut self.resolution_frame.discard_cause, chains)
        });
        self.discard_ids(&cards, player, &mut events);
        if let Some(cause) = stale_cause {
            self.resolution_frame.discard_cause = cause;
        }
        // Recall's "for each card discarded this way": tally what an *effect* discard actually
        // took, so a following step in the same resolution can read it back through
        // `Amount::CardsDiscardedThisWay`. A short hand discards fewer than asked (CR 700.2), and
        // that smaller number is what the rider is owed. A cleanup trim is not "this way".
        // Chains of Mephistopheles' substituted discard is nobody's resolution either — it
        // replaced a draw, it isn't a step of the effect that asked for one.
        if !is_cleanup && !draw_replacement {
            self.resolution_frame.cards_discarded_this_way = cards.len() as u32;
        }
        // A cleanup discard resumes the step-transition loop it interrupted; an effect discard's
        // sequence tail (if any) is resumed by [`Game::resume_deferred_sequence`] after this returns.
        if is_cleanup {
            events.extend(self.advance_step());
        }
        // "If the player discards a card this way, they draw a card." — then the rest of the draw
        // batch this discard interrupted.
        if draw_replacement {
            self.finish_chains_draw(player, &mut events);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PutFromHandOnTop`] (Brainstorm's "put two cards from your hand
    /// on top of your library in any order"): move the chosen cards to the top of the library,
    /// preserving the chosen order. Events apply bottom-to-top — the last-named card lands first
    /// (deepest), so the first-named card, applied last, ends up literally on top.
    pub(crate) fn answer_put_from_hand_on_top(
        &mut self,
        player: PlayerId,
        cards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PutFromHandOnTop {
            player: chooser,
            hand,
            count,
            life_per_declined,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let distinct = cards.iter().collect::<std::collections::HashSet<_>>().len();
        let all_in_hand = cards.iter().all(|c| hand.contains(c));
        // A price on the declined cards (Sylvan Library) makes `count` a ceiling — putting none
        // back and paying for all of them is a legal answer. Without one (Brainstorm) it's exact.
        let counted = if life_per_declined > 0 {
            cards.len() <= count
        } else {
            cards.len() == count
        };
        if player != chooser || !all_in_hand || !counted || distinct != cards.len() {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        // "For each of those cards, pay 4 life or put the card on top of your library": every
        // card of the `count` that didn't go back is paid for instead.
        let declined = (count - cards.len()) as u32 * life_per_declined;
        if declined > 0 {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -(declined as i32),
                    source: None,
                },
            );
        }
        for &from in cards.iter().rev() {
            let card = self.next_object_id();
            let def = self.def_id_of(from);
            self.push_apply(
                &mut events,
                Event::PutFromHandOnTop {
                    card,
                    from,
                    def,
                    player,
                },
            );
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
            at_most_one,
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // The answer must come from the asked player and only name permanents that were offered.
        if player != chooser || !keep_tapped.iter().all(|id| permanents.contains(id)) {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }
        // Smoke / Winter Orb (CR 502.2): a cap is a ceiling, not a quota — keeping every member of
        // a group tapped is a legal answer, letting two of one group up is not.
        if at_most_one
            .iter()
            .any(|group| group.iter().filter(|id| !keep_tapped.contains(id)).count() > 1)
        {
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
