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
    ) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextJoinForcesPayment { remaining, source },
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
        self.prompt_next_join_forces_payment(remaining, source);
        Ok(events)
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
            "past" => self.resolution_frame.council_past_votes += 1,
            "present" => self.resolution_frame.council_present_votes += 1,
            other => panic!("unknown council's-dilemma ballot {other:?}"),
        }
        self.prompt_next_vote(remaining, source, options);
        Ok(Vec::new())
    }

    /// Pause on the next seat in Conundrum Sphinx's name-a-card fan-out, or — when none remain —
    /// return, letting the enclosing sequence resume. Naming is mandatory (CR 201.2), so unlike a
    /// graveyard fan-out no seat is ever skipped.
    pub(crate) fn prompt_next_card_name(&mut self, remaining: Vec<PlayerId>, source: ObjectId) {
        crate::pending::raise(
            self,
            crate::pending::ChoiceRequest::NextCardName { remaining, source },
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
        if let Some(&card) = self.players[player.0 as usize].library.first() {
            let def = self.def_of(card);
            self.push_apply(
                &mut events,
                Event::RevealedTopOfLibrary { player, card, def },
            );
            if def.name == chosen {
                self.push_apply(
                    &mut events,
                    Event::SearchedToHand {
                        player,
                        object: self.next_object_id(),
                        from: card,
                        card: def,
                    },
                );
            } else {
                self.push_apply(&mut events, Event::PutOnBottomOfLibrary { player, card });
            }
        }
        self.prompt_next_card_name(remaining, source);
        Ok(events)
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
    /// ([`Effect::Choice(ChoiceEffect::MayReturnFromGraveyard)`] — Deadly Brew's rider).
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
    /// [`Effect::Choice(ChoiceEffect::Discard)`]'s [`PendingChoice::DiscardCards`]: move the chosen cards to the
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
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let distinct = cards.iter().collect::<std::collections::HashSet<_>>().len();
        let all_in_hand = cards.iter().all(|c| hand.contains(c));
        if player != chooser || !all_in_hand || cards.len() != count || distinct != cards.len() {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        for &from in cards.iter().rev() {
            let card = self.next_object_id();
            let def = self.def_of(from);
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
