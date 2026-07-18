//! Dig-loop, cascade, dance, exile-cast answers and kickoffs.

use crate::*;

impl Game {
    pub(crate) fn choose_exiled_with_card(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledWithCard {
            source, candidates, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            let is_land = matches!(self.def_of(card).kind, CardKind::Land { .. });
            let graveyard_event = self.graveyard_or_command(card, self.next_object_id());
            self.push_apply(&mut events, graveyard_event);
            self.push_apply(
                &mut events,
                Event::CardExiledWithSourceLeftExile {
                    source,
                    object: card,
                },
            );
            let token_event = Event::TokenCreated {
                token: self.next_object_id(),
                controller: player,
                // ponytail: the real token is a black Rogue (CR would set color + subtype); token
                // color/creature-subtype isn't modeled yet (the #10 gap) — a plain colorless,
                // subtype-less body stands in either way.
                def: if is_land {
                    treasure_token()
                } else {
                    rogue_token_stub()
                },
                creator: source,
            };
            self.push_apply(&mut events, token_event);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseExiledWithCardToCast`]: grant the free-cast permission
    /// (CR 118.5, "without paying its mana cost") for the chosen card, or decline
    /// (`choice = None`). Unlike [`Game::choose_exiled_with_card`], the chosen card stays in the
    /// exiled-with pile — this only grants a permission, it doesn't cash the card out.
    pub(crate) fn choose_exiled_with_card_to_cast(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledWithCardToCast { candidates, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
            // CR 614.6 replacement rider — see `PlayPermissions::stack_object_bottoms_library_on_leave`.
            self.push_apply(
                &mut events,
                Event::CastFromExileFreeBottomsLibraryOnLeave { card },
            );
        }
        Ok(events)
    }

    /// Resolve [`Effect::ExileTopCastMatchingFree`] (Herald of Amity's dig): exile the top
    /// `count` cards of `controller`'s library face-up (public, CR 701.17), then raise a
    /// choose-up-to-one over the exiled cards matching `filter`. A short library exiles only
    /// what's there (CR 120-style "as many as possible"); an empty library raises no choice (a
    /// harmless no-op, mirroring empty-library ArrangeTop). No exiled card matching `filter`
    /// also raises no choice — there's nothing to offer — but "put the rest on the bottom" still
    /// has to run, so the whole batch is bottomed immediately instead.
    pub(crate) fn exile_top_cast_matching_free(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        count: u32,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let top: Vec<ObjectId> = self.players[controller.0 as usize]
            .library
            .iter()
            .take(count as usize)
            .copied()
            .collect();
        if top.is_empty() {
            return; // nothing to exile — legal no-op.
        }
        let exiled: Vec<ObjectId> = top
            .iter()
            .map(|&from| {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player: controller,
                        card,
                        from,
                        face_down: false,
                    },
                );
                card
            })
            .collect();
        let candidates: Vec<ObjectId> = exiled
            .iter()
            .copied()
            .filter(|&id| filter.matches(self.def_of(id)))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledDigToCastFree {
                player: controller,
                source,
                candidates,
                exiled: exiled.clone(),
            },
        );
        if !self.resolution_is_paused() {
            self.bottom_exiled_dig(&exiled, events);
        }
    }

    /// Answer a [`PendingChoice::ChooseExiledDigToCastFree`]: grant the free-cast permission
    /// (CR 118.5) for the chosen card — it stays in exile — or decline (`choice = None`). Either
    /// way, every other card in the exiled batch (non-matching, or simply not chosen) goes to the
    /// bottom of the library (CR "put the rest on the bottom of your library").
    pub(crate) fn choose_exiled_dig_to_cast_free(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledDigToCastFree {
            candidates, exiled, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(card) = choice {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
        }
        let rest: Vec<ObjectId> = exiled
            .into_iter()
            .filter(|&id| Some(id) != choice)
            .collect();
        self.bottom_exiled_dig(&rest, &mut events);
        Ok(events)
    }

    /// Put every exiled-dig card in `cards` on the bottom of its owner's library in a random
    /// order (CR "in a random order" — shared by Herald of Amity's dig and Cascade). The order is
    /// randomized with the injected splitmix PRNG (Fisher-Yates), keeping the engine deterministic
    /// and pure — no `rand`. Shared by [`Self::exile_top_cast_matching_free`]'s /
    /// [`Self::cascade`]'s no-hit fast paths and [`Self::choose_exiled_dig_to_cast_free`]'s
    /// answer.
    pub(crate) fn bottom_exiled_dig(&mut self, cards: &[ObjectId], events: &mut Vec<Event>) {
        let mut order = cards.to_vec();
        for i in (1..order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for from in order {
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::TuckedToLibrary {
                    card,
                    from,
                    to_top: false,
                },
            );
        }
    }

    /// Put every card in `cards` (already `player`'s own library cards) on the bottom of that
    /// library in a random order (CR "in a random order" — shared by Songbirds' Blessing's
    /// [`Self::reveal_until_may_deploy`] and Creative Technique's
    /// [`Self::reveal_until_exile_cast_free`]). Same Fisher-Yates-with-the-injected-PRNG
    /// shuffle as [`Self::bottom_exiled_dig`], but each card is already in the library — a
    /// same-zone reorder via [`Event::PutOnBottomOfLibrary`], not a zone change.
    pub(crate) fn bottom_pile_in_library(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        let mut order = cards.to_vec();
        for i in (1..order.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        for card in order {
            self.push_apply(events, Event::PutOnBottomOfLibrary { player, card });
        }
    }

    /// The opponent who makes an "an opponent chooses" decision on `controller`'s behalf (Plargg
    /// and Nassari's nonland pick): the next living player reached walking turn order forward
    /// from `controller`.
    /// ponytail: hardcodes the next opponent in turn order rather than the controller's own pick
    /// (the same approximation [`Self::choose_splitting_opponent`] used to carry for
    /// Abstract Performance and Fact or Fiction, before their shared chooser fixed it) — migrate
    /// Plargg and Nassari to that chooser if a pool card ever needs "an opponent" here to be a
    /// genuine choice. `None` only if `controller` is the sole living player left (unreachable in
    /// a real game).
    pub(crate) fn next_opponent_in_turn_order(&self, controller: PlayerId) -> Option<PlayerId> {
        let n = self.players.len();
        (1..n)
            .map(|i| PlayerId(((controller.0 as usize + i) % n) as u8))
            .find(|&p| !self.players[p.0 as usize].lost)
    }

    /// Exile the top `n` cards of `player`'s library into a pile, returning the new exile-object
    /// ids top-to-bottom. Face-up (public, CR 701.17) unless `face_down` (CR 701.9 — Abstract
    /// Performance's first pile, hidden from every viewer but `player` while it awaits the
    /// opponent's pick; see [`Card::face_down`]). Applied immediately, so a second call reads the
    /// library's new top (Abstract Performance's two consecutive four-card piles).
    fn exile_top_into_pile(
        &mut self,
        player: PlayerId,
        n: usize,
        face_down: bool,
        events: &mut Vec<Event>,
    ) -> Vec<ObjectId> {
        let top: Vec<ObjectId> = self.players[player.0 as usize]
            .library
            .iter()
            .take(n)
            .copied()
            .collect();
        top.into_iter()
            .map(|from| {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player,
                        card,
                        from,
                        face_down,
                    },
                );
                card
            })
            .collect()
    }

    /// Route the cards in `exiled` that weren't `chosen` (granted the free cast) once a
    /// [`PendingChoice::ChooseExiledToCastFree`] is answered: to their owner's hand if
    /// `rest_to_hand` (Abstract Performance's "put the rest into your hand"), else leave them in
    /// exile (Plargg and Nassari's uncast cards stay exiled). A chosen card stays in exile until
    /// it's actually cast under the permission.
    fn route_exiled_rest(
        &mut self,
        exiled: &[ObjectId],
        chosen: &[ObjectId],
        rest_to_hand: bool,
        events: &mut Vec<Event>,
    ) {
        if !rest_to_hand {
            return;
        }
        for &from in exiled {
            if chosen.contains(&from) {
                continue;
            }
            self.push_apply(
                events,
                Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from,
                },
            );
        }
    }

    /// Offer `controller` up to `count` free casts (CR 118.5) over the castable (nonland) cards in
    /// `exiled`, raising [`ChoiceRequest::ChooseExiledToCastFree`]. With no castable card
    /// there's nothing to offer, so the rest routes immediately. Shared by Abstract Performance
    /// (`count = 1`, rest to hand) and Plargg and Nassari (`count = 2`, rest stays exiled).
    pub(crate) fn choose_exiled_to_cast_free_pile(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        exiled: Vec<ObjectId>,
        count: u8,
        rest_to_hand: bool,
        events: &mut Vec<Event>,
    ) {
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledToCastFree {
                player: controller,
                source,
                exiled: exiled.clone(),
                count,
                rest_to_hand,
            },
        );
        if !self.resolution_is_paused() {
            self.route_exiled_rest(&exiled, &[], rest_to_hand, events);
        }
    }

    /// Answer a [`PendingChoice::ChooseExiledToCastFree`]: grant the free-cast permission (CR
    /// 118.5) to each `chosen` card (up to `count`, all distinct candidates), then route the rest.
    pub(crate) fn choose_exiled_to_cast_free(
        &mut self,
        player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseExiledToCastFree {
            candidates,
            exiled,
            count,
            rest_to_hand,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let distinct = chosen
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        if chosen.len() > count as usize
            || distinct != chosen.len()
            || chosen.iter().any(|c| !candidates.contains(c))
        {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &card in &chosen {
            self.push_apply(
                &mut events,
                Event::CastFromExileFreePermissionGranted { card, player },
            );
        }
        self.route_exiled_rest(&exiled, &chosen, rest_to_hand, &mut events);
        Ok(events)
    }

    /// Resolve Dance with Calamity's push-your-luck loop
    /// ([`Effect::ExileTopUntilStopCastFreeUnderBudget`]): raise a first
    /// [`ChoiceRequest::DanceExileMore`] over an empty exile pile. An already-empty library has
    /// nothing to exile, so it resolves the payoff straight away (a zero tally — nothing to cast).
    pub(crate) fn dance_with_calamity(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        budget: u32,
        events: &mut Vec<Event>,
    ) {
        if self.players[controller.0 as usize].library.is_empty() {
            self.finish_dance(controller, source, Vec::new(), 0, budget, events);
            return;
        }
        pending::raise(
            self,
            pending::ChoiceRequest::DanceExileMore {
                player: controller,
                source,
                exiled: Vec::new(),
                total_mv: 0,
                budget,
            },
        );
    }

    /// Answer a [`PendingChoice::DanceExileMore`]: on `yes`, exile the top card of the caster's
    /// library face-up (CR 701.17, public) and add its mana value to the running tally, then
    /// re-raise (unless the library is now empty). On `no` — or once the library empties — stop and
    /// resolve the payoff ([`Self::finish_dance`]).
    pub(crate) fn dance_exile_more(
        &mut self,
        player: PlayerId,
        yes: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DanceExileMore {
            source,
            mut exiled,
            mut total_mv,
            budget,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        if yes && let Some(&from) = self.players[player.0 as usize].library.first() {
            let card = self.next_object_id();
            total_mv += self.def_of(from).mana_value();
            self.push_apply(
                &mut events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player,
                    card,
                    from,
                    face_down: false,
                },
            );
            exiled.push(card);
        }
        // Keep offering as long as the caster wants more and the library still has a card to exile.
        if yes && !self.players[player.0 as usize].library.is_empty() {
            pending::raise(
                self,
                pending::ChoiceRequest::DanceExileMore {
                    player,
                    source,
                    exiled,
                    total_mv,
                    budget,
                },
            );
            return Ok(events);
        }
        self.finish_dance(player, source, exiled, total_mv, budget, &mut events);
        Ok(events)
    }

    /// Resolve Dance with Calamity's payoff once the exile loop stops: if `total_mv <= budget` the
    /// caster may cast any number of the exiled (nonland) cards for free (CR 118.5) — raising
    /// [`ChoiceRequest::ChooseExiledToCastFree`] over the whole pile, uncast cards staying exiled.
    /// On a bust (`total_mv > budget`) nothing is offered and every exiled card stays exiled (a
    /// bust never returns them).
    fn finish_dance(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        exiled: Vec<ObjectId>,
        total_mv: u32,
        budget: u32,
        events: &mut Vec<Event>,
    ) {
        if total_mv > budget {
            return; // bust — the exiled cards stay exiled with no free-cast permission.
        }
        // "any number" — the cap is the whole exiled pile (u8-bounded; a legal Dance can never
        // exile more than a handful of nonland cards under a 13-MV budget).
        let count = exiled.len().min(u8::MAX as usize) as u8;
        self.choose_exiled_to_cast_free_pile(controller, source, exiled, count, false, events);
    }

    /// Resolve Abstract Performance ([`Effect::OpponentSplitsExilePiles`]): exile the top four
    /// (CR 701.9 face-down — hidden from every viewer but `controller`) then the next four
    /// (face-up) of `controller`'s library into two piles, then hand off to the shared
    /// [`Self::choose_splitting_opponent`] chooser to pick who picks a pile.
    pub(crate) fn opponent_splits_exile_piles(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let pile_a = self.exile_top_into_pile(controller, 4, true, events);
        let pile_b = self.exile_top_into_pile(controller, 4, false, events);
        self.choose_splitting_opponent(
            controller,
            source,
            SplittingContinuation::ExilePiles { pile_a, pile_b },
        );
    }

    /// Resolve Fact or Fiction ([`Effect::RevealTopSplitPiles`]): reveal the top five of
    /// `controller`'s library (all public, CR 701.16; a short library reveals only what's there,
    /// CR 120.3 "as many as possible" — an empty library reveals nothing and raises no pause),
    /// then hand off to the shared [`Self::choose_splitting_opponent`] chooser to pick who
    /// partitions them.
    pub(crate) fn reveal_top_split_piles(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let revealed: Vec<ObjectId> = self.players[controller.0 as usize]
            .library
            .iter()
            .take(5)
            .copied()
            .collect();
        for &card in &revealed {
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card,
                    def: self.def_of(card),
                },
            );
        }
        if revealed.is_empty() {
            return; // an empty library reveals nothing — no pile to split.
        }
        self.choose_splitting_opponent(
            controller,
            source,
            SplittingContinuation::Partition { revealed },
        );
    }

    /// The shared "an opponent ..." chooser for [`Effect::OpponentSplitsExilePiles`] and
    /// [`Effect::RevealTopSplitPiles`]: with more than one opponent alive, `controller` picks
    /// which one on a [`ChoiceRequest::ChooseSplittingOpponent`]; with at most one, resume
    /// immediately (the "single-legal-choice" collapse — no real choice to offer). `then` carries
    /// the split data already computed, so it's ready the instant the opponent is known.
    pub(crate) fn choose_splitting_opponent(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        then: SplittingContinuation,
    ) {
        let legal: Vec<PlayerId> = self.living_players().filter(|&p| p != controller).collect();
        match legal.as_slice() {
            [] => {} // ponytail: no opponent to choose or split — unreachable in a real game.
            [only] => self.resume_splitting_opponent(*only, controller, source, then),
            _ => pending::raise(
                self,
                pending::ChoiceRequest::ChooseSplittingOpponent {
                    player: controller,
                    source,
                    legal,
                    then,
                },
            ),
        }
    }

    /// Answer a [`PendingChoice::ChooseSplittingOpponent`]: `opponent` must be one of the choice's
    /// `legal` candidates. Resumes via [`Self::resume_splitting_opponent`].
    pub(crate) fn choose_splitting_opponent_answer(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        legal: Vec<PlayerId>,
        then: SplittingContinuation,
        targets: Vec<Target>,
    ) -> Result<Vec<Event>, Reject> {
        let [Target::Player(opponent)] = targets[..] else {
            return Err(Reject::IllegalChoice);
        };
        if !legal.contains(&opponent) {
            return Err(Reject::IllegalTarget);
        }
        self.finish_answer();
        self.resume_splitting_opponent(opponent, controller, source, then);
        Ok(Vec::new())
    }

    /// Resume [`Self::choose_splitting_opponent`] once the splitting opponent is known:
    /// raise `opponent` on whichever pause `then` names.
    pub(crate) fn resume_splitting_opponent(
        &mut self,
        opponent: PlayerId,
        controller: PlayerId,
        source: ObjectId,
        then: SplittingContinuation,
    ) {
        match then {
            SplittingContinuation::ExilePiles { pile_a, pile_b } => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::OpponentChoosesPile {
                        player: opponent,
                        controller,
                        source,
                        pile_a,
                        pile_b,
                    },
                );
            }
            SplittingContinuation::Partition { revealed } => {
                pending::raise(
                    self,
                    pending::ChoiceRequest::PartitionRevealed {
                        player: opponent,
                        controller,
                        source,
                        revealed,
                    },
                );
            }
        }
    }

    /// Answer a [`PendingChoice::PartitionRevealed`] (Fact or Fiction): `pile_a` names the subset
    /// of `revealed` the chosen opponent puts into pile A (reusing
    /// [`Intent::ChooseSacrifices`]'s "name the subset" wire shape); everything else in `revealed`
    /// forms pile B. Either may be empty. Pauses `controller` on a
    /// [`PendingChoice::ChoosePileForHand`] to pick which pile to keep.
    pub(crate) fn partition_revealed(
        &mut self,
        _player: PlayerId,
        pile_a: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PartitionRevealed {
            controller,
            source,
            revealed,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        for (i, id) in pile_a.iter().enumerate() {
            // Each card in pile A must be one of the revealed five, named at most once.
            if !revealed.contains(id) || pile_a[..i].contains(id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();
        let pile_b: Vec<ObjectId> = revealed
            .into_iter()
            .filter(|id| !pile_a.contains(id))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChoosePileForHand {
                player: controller,
                source,
                pile_a,
                pile_b,
            },
        );
        Ok(Vec::new())
    }

    /// Answer a [`PendingChoice::ChoosePileForHand`] (Fact or Fiction): the chosen pile goes to
    /// `player`'s hand (CR "Put one pile into your hand"), the other is milled into their
    /// graveyard (CR "and the other into your graveyard" — [`Event::Milled`], since these cards
    /// never left the library).
    pub(crate) fn choose_pile_for_hand(
        &mut self,
        player: PlayerId,
        pile: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChoosePileForHand { pile_a, pile_b, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let (to_hand, to_graveyard) = match pile {
            0 => (pile_a, pile_b),
            1 => (pile_b, pile_a),
            _ => return Err(Reject::IllegalChoice),
        };
        self.finish_answer();

        let mut events = Vec::new();
        for from in to_hand {
            self.push_apply(
                &mut events,
                Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from,
                    card: self.def_of(from),
                },
            );
        }
        for from in to_graveyard {
            self.push_apply(
                &mut events,
                Event::Milled {
                    player,
                    card: self.next_object_id(),
                    from,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::OpponentChoosesPile`]: the chosen pile goes to `controller`'s
    /// graveyard; the other pile is offered to `controller` on a
    /// [`PendingChoice::ChooseExiledToCastFree`] (up to one free cast, the rest to hand).
    pub(crate) fn choose_opponent_pile(
        &mut self,
        _player: PlayerId,
        pile: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::OpponentChoosesPile {
            controller,
            source,
            pile_a,
            pile_b,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let (chosen, other) = match pile {
            0 => (pile_a, pile_b),
            1 => (pile_b, pile_a),
            _ => return Err(Reject::IllegalChoice),
        };
        self.finish_answer();

        let mut events = Vec::new();
        for from in chosen {
            self.push_apply(
                &mut events,
                Event::MovedToGraveyard {
                    card: self.next_object_id(),
                    from,
                },
            );
        }
        self.choose_exiled_to_cast_free_pile(controller, source, other, 1, true, &mut events);
        Ok(events)
    }

    /// Resolve Plargg and Nassari ([`Effect::EachPlayerExilesUntilNonlandOpponentPicks`]): each
    /// living player, in APNAP order, exiles cards from the top of their own library until they
    /// exile a nonland (all face-up, public); an opponent then picks one of the exiled nonlands.
    pub(crate) fn each_player_exiles_until_nonland(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let mut exiled: Vec<ObjectId> = Vec::new();
        let mut nonlands: Vec<ObjectId> = Vec::new();
        for p in self.apnap_order() {
            while let Some(&from) = self.players[p.0 as usize].library.first() {
                let card = self.next_object_id();
                self.push_apply(
                    events,
                    Event::ExiledFromLibraryToChooseCastFree {
                        player: p,
                        card,
                        from,
                        face_down: false,
                    },
                );
                exiled.push(card);
                if !matches!(self.def_of(card).kind, CardKind::Land { .. }) {
                    nonlands.push(card);
                    break;
                }
            }
        }
        let Some(opponent) = self.next_opponent_in_turn_order(controller) else {
            return; // ponytail: no opponent to choose — unreachable in a real game.
        };
        pending::raise(
            self,
            pending::ChoiceRequest::OpponentChoosesExiledNonland {
                player: opponent,
                controller,
                source,
                nonlands,
                exiled,
            },
        );
    }

    /// Answer a [`PendingChoice::OpponentChoosesExiledNonland`]: the picked nonland stays exiled;
    /// `controller` is then offered up to two free casts over the *other* exiled cards.
    pub(crate) fn choose_opponent_exiled_nonland(
        &mut self,
        _player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::OpponentChoosesExiledNonland {
            controller,
            source,
            nonlands,
            exiled,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // The opponent must choose one exiled nonland — declining isn't legal (CR "an opponent
        // chooses a nonland card exiled this way").
        let Some(picked) = choice.filter(|c| nonlands.contains(c)) else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        let others: Vec<ObjectId> = exiled.into_iter().filter(|&id| id != picked).collect();
        self.choose_exiled_to_cast_free_pile(controller, source, others, 2, false, &mut events);
        Ok(events)
    }

    /// Resolve [`Effect::RevealUntilMayDeploy`] (Songbirds' Blessing's enchanted-creature-
    /// attacks trigger): reveal `controller`'s own top cards one at a time until the first card
    /// matching `filter` or the library runs out (CR 120.3), collecting every non-match along the
    /// way. The matching card is left unmoved on top of the library and offered on a
    /// [`ChoiceRequest::RevealedCardToBattlefieldOrHand`]; either way (a hit or a whiff), the
    /// collected non-matches are bottomed together, shuffled, via
    /// [`Self::bottom_pile_in_library`].
    pub(crate) fn reveal_until_may_deploy(
        &mut self,
        controller: PlayerId,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let library: Vec<ObjectId> = self.players[controller.0 as usize].library.clone();
        let mut rest: Vec<ObjectId> = Vec::new();
        for card in library {
            let def = self.def_of(card);
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card,
                    def,
                },
            );
            if !filter.matches(def) {
                rest.push(card);
                continue;
            }
            self.bottom_pile_in_library(controller, &rest, events);
            pending::raise(
                self,
                pending::ChoiceRequest::RevealedCardToBattlefieldOrHand {
                    player: controller,
                    card,
                },
            );
            return;
        }
        self.bottom_pile_in_library(controller, &rest, events);
    }

    /// Answer a [`PendingChoice::RevealedCardToBattlefieldOrHand`]: `choice = Some(card)` puts
    /// the revealed card onto the battlefield untapped; `None` puts it into hand instead. An
    /// Aura deployed this way may pause again on [`PendingChoice::ChooseAttachHost`]
    /// (CR 303.4f) — see [`Self::maybe_pause_attach_deployed_aura`].
    pub(crate) fn revealed_card_to_battlefield_or_hand(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::RevealedCardToBattlefieldOrHand { card, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| c != card) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if choice.is_some() {
            let permanent = self.next_object_id();
            self.push_apply(
                &mut events,
                Event::SearchedToBattlefield {
                    permanent,
                    from: card,
                    controller: player,
                    tapped: false,
                },
            );
            self.maybe_pause_attach_deployed_aura(permanent, player);
        } else {
            let def = self.def_of(card);
            self.push_apply(
                &mut events,
                Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from: card,
                    card: def,
                },
            );
        }
        Ok(events)
    }

    /// After an Aura or Equipment is put onto the battlefield without being cast (Songbirds'
    /// Blessing's [`Self::revealed_card_to_battlefield_or_hand`], Armored Skyhunter's
    /// [`Self::select_from_top`] `TopDest::Battlefield` branch), pause its controller to choose a
    /// host among the battlefield creatures it could legally attach to. An Aura's candidates are
    /// creatures it could legally enchant (CR 303.4f), reusing the same `enchant`-restriction
    /// legality check an Aura spell's own cast target uses ([`Game::required_target`]); the
    /// choice is mandatory. Equipment's candidates are creatures its controller controls (CR
    /// 301.5c "you may attach it to a creature you control"); the choice is optional (may
    /// decline, leaving it unattached — legal for Equipment, unlike an unattached Aura). Any
    /// other permanent kind is untouched, no pause. Either way, no legal host means no pause: an
    /// Aura stays unattached and the existing Aura-legality state-based action (CR 704.5m) sweeps
    /// it to the graveyard once this submission's SBA sweep runs; a hostless Equipment simply
    /// sits unattached, which is always legal for Equipment.
    pub(crate) fn maybe_pause_attach_deployed_aura(
        &mut self,
        deployed: ObjectId,
        controller: PlayerId,
    ) {
        let def = self.def_of(deployed);
        let (host_filter, optional) = match def.kind {
            CardKind::Aura => (
                def.enchant
                    .unwrap_or(PermanentFilter::of(TypeSet::CREATURE)),
                false,
            ),
            _ if def.subtypes.contains(&"Equipment") => (
                PermanentFilter {
                    controller: FilterController::You,
                    ..PermanentFilter::of(TypeSet::CREATURE)
                },
                true,
            ),
            _ => return,
        };
        let candidates: Vec<ObjectId> = self
            .battlefield()
            .into_iter()
            .filter(|&id| self.permanent_matches(&host_filter, id, controller, Some(deployed)))
            .collect();
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseAttachHost {
                player: controller,
                attachment: deployed,
                candidates,
                optional,
            },
        );
    }

    /// Answer a [`PendingChoice::ChooseAttachHost`]: `host = Some(id)` attaches the deployed
    /// Aura/Equipment to `id` (CR 303.4f / CR 301.5c — the controller chooses among the objects
    /// it could legally attach to). `host = None` only answers an `optional` choice (an
    /// Equipment declining to attach); a mandatory Aura host rejects it.
    pub(crate) fn choose_attach_host(
        &mut self,
        _player: PlayerId,
        host: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseAttachHost {
            attachment,
            candidates,
            optional,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let Some(host) = host else {
            if !optional {
                return Err(Reject::IllegalChoice);
            }
            self.finish_answer();
            return Ok(Vec::new());
        };
        if !candidates.contains(&host) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::AttachedTo {
                object: attachment,
                host: Some(host),
            },
        );
        Ok(events)
    }

    /// Shared core of [`Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost`] (Screams from
    /// Within's immediate dies-return, Ghoulish Impetus's delayed one once its schedule fires):
    /// move this Aura (`source`) from the graveyard to the battlefield unattached — under its
    /// owner's control, the same reanimate-a-card-from-your-own-graveyard idiom
    /// [`Effect::ReturnThisAuraAttachedTo`]'s resolve arm uses — then pause on
    /// [`PendingChoice::ChooseAttachHost`] via
    /// [`Self::maybe_pause_attach_deployed_aura`], the same choose-host surface a deployed Aura
    /// already uses (CR 303.4f). Guard-returns with no events if this Aura has since left the
    /// graveyard (CR 603.10a last-known information — milled/exiled elsewhere in the meantime).
    pub(crate) fn return_aura_from_graveyard_attached_to_chosen_host(
        &mut self,
        source: ObjectId,
        events: &mut Vec<Event>,
    ) {
        let card = self.current_id(source);
        if self.zone_of(card) != Zone::Graveyard {
            return;
        }
        let owner = self.owner_of(card);
        let event = self.reanimate_event(card, owner, false);
        let Event::ReanimatedToBattlefield { permanent, .. } = event else {
            unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
        };
        self.push_apply(events, event);
        self.maybe_pause_attach_deployed_aura(permanent, owner);
    }

    /// Resolve [`Effect::RevealUntilExileCastFree`] (Creative Technique's reveal-until-nonland
    /// dig, run after its preceding [`Effect::ShuffleLibrary`] step): reveal `controller`'s own
    /// top cards one at a time — same loop shape as [`Self::reveal_until_may_deploy`] —
    /// until the first card matching `filter` or the library runs out, collecting every non-match
    /// along the way. The matching card is exiled face-up and raises the shared
    /// [`ChoiceRequest::ChooseExiledDigToCastFree`] (a single-candidate batch); either way (a hit
    /// or a whiff), the collected non-matches are bottomed together, shuffled, via
    /// [`Self::bottom_pile_in_library`].
    pub(crate) fn reveal_until_exile_cast_free(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        filter: CardFilter,
        events: &mut Vec<Event>,
    ) {
        let library: Vec<ObjectId> = self.players[controller.0 as usize].library.clone();
        let mut rest: Vec<ObjectId> = Vec::new();
        for from in library {
            let def = self.def_of(from);
            self.push_apply(
                events,
                Event::RevealedTopOfLibrary {
                    player: controller,
                    card: from,
                    def,
                },
            );
            if !filter.matches(def) {
                rest.push(from);
                continue;
            }
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player: controller,
                    card,
                    from,
                    face_down: false,
                },
            );
            self.bottom_pile_in_library(controller, &rest, events);
            pending::raise(
                self,
                pending::ChoiceRequest::ChooseExiledDigToCastFree {
                    player: controller,
                    source,
                    candidates: vec![card],
                    exiled: vec![card],
                },
            );
            return;
        }
        self.bottom_pile_in_library(controller, &rest, events);
    }

    /// Resolve [`Effect::Cascade`] (CR 702.85). Reveal cards from the top of `controller`'s
    /// library one at a time, exiling each face-up (public, reusing the dig's
    /// [`Event::ExiledFromLibraryToChooseCastFree`]), until the just-exiled card is a **nonland**
    /// with mana value strictly less than `mana_value` (the cascading spell's own mana value), or
    /// the library runs out (CR 702.85c "as many as possible"). A hit raises a may-cast-it-free
    /// choice over exactly that card (reusing [`ChoiceRequest::ChooseExiledDigToCastFree`]); with
    /// no hit, the whole reveal is bottomed immediately in a random order.
    pub(crate) fn cascade(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        mana_value: u32,
        events: &mut Vec<Event>,
    ) {
        let mut exiled: Vec<ObjectId> = Vec::new();
        let mut hit: Option<ObjectId> = None;
        while let Some(&from) = self.players[controller.0 as usize].library.first() {
            let card = self.next_object_id();
            self.push_apply(
                events,
                Event::ExiledFromLibraryToChooseCastFree {
                    player: controller,
                    card,
                    from,
                    face_down: false,
                },
            );
            exiled.push(card);
            let def = self.def_of(card);
            if !matches!(def.kind, CardKind::Land { .. }) && def.mana_value() < mana_value {
                hit = Some(card);
                break;
            }
        }
        let Some(hit) = hit else {
            self.bottom_exiled_dig(&exiled, events); // whiff — bottom everything revealed
            return;
        };
        pending::raise(
            self,
            pending::ChoiceRequest::ChooseExiledDigToCastFree {
                player: controller,
                source,
                candidates: vec![hit],
                exiled,
            },
        );
    }
}
