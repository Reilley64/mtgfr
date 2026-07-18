//! Library arrange/search/mode/color answers.

use crate::*;

impl Game {
    pub(crate) fn arrange_top(
        &mut self,
        player: PlayerId,
        top: Vec<ObjectId>,
        bottom: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ArrangeTop {
            cards,
            to_graveyard,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if !is_partition(&top, &bottom, &cards) {
            return Err(Reject::IllegalChoice); // not a split of exactly the shown cards
        }
        self.finish_answer();

        // The untouched remainder of the library, below the looked-at cards (captured before any
        // mutation — no other action is legal while the choice is pending, so it's still intact).
        let count = cards.len();
        let rest: Vec<ObjectId> = self.players[player.0 as usize].library[count..].to_vec();

        let mut events = Vec::new();
        if to_graveyard {
            // Surveil: the bottom pile is put into the graveyard — the same library→graveyard
            // zone change as a mill (each mints a fresh graveyard-object id in order).
            let base = self.next_object_id();
            for (i, &from) in bottom.iter().enumerate() {
                let event = Event::Milled {
                    player,
                    card: base + i as u32,
                    from,
                };
                self.apply(&event);
                events.push(event);
            }
        }

        // Rebuild the library's top in the chosen order: kept cards first, then the untouched
        // remainder, then (for scry) the bottom pile at the very bottom.
        // ponytail: library order isn't event-sourced (neither is `shuffle`) — mutate it directly.
        let mut library = top;
        library.extend(rest);
        if !to_graveyard {
            library.extend(bottom);
        }
        self.players[player.0 as usize].library = library;
        Ok(events)
    }

    /// Answer a [`PendingChoice::SelectFromTop`]: move each `selected` card (up to `up_to`, each
    /// offered and matching the choice's `filter`) to `dest`, and put every non-selected looked-at
    /// card into `rest`. Selecting fewer than `up_to` is legal down to `min` (Dig Through Time's
    /// mandatory "put two of them into your hand"; `min: 0` is the "may" default, always legal
    /// down to zero). `min` is bounded by how many cards were actually looked at (CR 120-style "as
    /// many as possible" on a short library). The remainder of the library below the looked-at
    /// cards is untouched.
    pub(crate) fn select_from_top(
        &mut self,
        player: PlayerId,
        selected: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SelectFromTop {
            cards,
            filter,
            up_to,
            min,
            dest,
            dest_tapped,
            rest,
            mv_budget,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if selected.len() > up_to as usize {
            return Err(Reject::IllegalChoice); // may take at most `up_to`
        }
        if selected.len() < (min as usize).min(cards.len()) {
            return Err(Reject::IllegalChoice); // must take at least `min` (clamped to the look)
        }
        // A total-mana-value budget (Ao, the Dawn Sky's "total mana value 4 or less") caps the
        // summed mana value of the selected cards, independent of the count bounds above.
        if let Some(budget) = mv_budget {
            let total: u32 = selected
                .iter()
                .map(|&id| self.def_of(id).mana_value())
                .sum();
            if total > budget {
                return Err(Reject::IllegalChoice);
            }
        }
        for (i, &id) in selected.iter().enumerate() {
            // Each selected card must be one of the looked-at cards, match the filter, and appear
            // at most once.
            if !cards.contains(&id)
                || !filter.matches(self.def_of(id))
                || selected[..i].contains(&id)
            {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        // Selected cards leave the library for `dest` (each move retain-removes its id from the
        // library, leaving the non-selected looked-at cards — the `bottomed` pile below — still
        // sitting in place at the top).
        let mut events = Vec::new();
        let mut deployed: Option<ObjectId> = None;
        for &from in &selected {
            let event = match dest {
                TopDest::Hand => Event::SearchedToHand {
                    player,
                    object: self.next_object_id(),
                    from,
                    card: self.def_of(from),
                },
                TopDest::Battlefield => {
                    let permanent = self.next_object_id();
                    deployed = Some(permanent);
                    Event::SearchedToBattlefield {
                        permanent,
                        from,
                        controller: player,
                        tapped: dest_tapped,
                    }
                }
            };
            self.push_apply(&mut events, event);
        }
        // An Aura among the deployed permanents may need a host chosen (CR 303.4f). Scoped to a
        // lone deployed permanent (Armored Skyhunter's `up_to = 1`) — a multi-permanent batch
        // that happens to include an Aura (no pool card does today) is left to the existing
        // hostless-Aura state-based action, same as before this pause existed. (CR 704, CR 303.4)
        if selected.len() == 1
            && let Some(permanent) = deployed
        {
            self.maybe_pause_attach_deployed_aura(permanent, player);
        }

        // Every non-selected looked-at card goes to `rest`, in a random order (CR "in a random
        // order" — the same PRNG-shuffle-then-bottom idiom the `reveal_until` family uses).
        let bottomed: Vec<ObjectId> = cards
            .iter()
            .copied()
            .filter(|c| !selected.contains(c))
            .collect();
        match rest {
            RestDest::Bottom => self.bottom_pile_in_library(player, &bottomed, &mut events),
            RestDest::Hand => {
                for from in bottomed {
                    let event = Event::SearchedToHand {
                        player,
                        object: self.next_object_id(),
                        from,
                        card: self.def_of(from),
                    };
                    self.push_apply(&mut events, event);
                }
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::DistributeTop`]: route `to_hand` to hand, `to_bottom` to the
    /// library bottom, and `to_exile_may_play` into exile with permission to play this turn — the
    /// same impulse-draw events [`Game::exile_top_may_play_events`] mints, one card at a time. The
    /// three lists must each match the choice's slot size, be drawn from the looked-at `cards`,
    /// and share no card.
    pub(crate) fn distribute_top(
        &mut self,
        player: PlayerId,
        to_hand: Vec<ObjectId>,
        to_bottom: Vec<ObjectId>,
        to_exile_may_play: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::DistributeTop {
            cards,
            to_hand: hand_slot,
            to_bottom: bottom_slot,
            to_exile_may_play: exile_slot,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if to_hand.len() != hand_slot as usize
            || to_bottom.len() != bottom_slot as usize
            || to_exile_may_play.len() != exile_slot as usize
        {
            return Err(Reject::IllegalChoice);
        }
        let mut assigned: Vec<ObjectId> = Vec::new();
        for &id in to_hand.iter().chain(&to_bottom).chain(&to_exile_may_play) {
            // Each routed card must be one of the looked-at cards and appear in exactly one slot.
            if !cards.contains(&id) || assigned.contains(&id) {
                return Err(Reject::IllegalChoice);
            }
            assigned.push(id);
        }
        self.finish_answer();

        // The untouched remainder of the library, below the looked-at cards (captured before any
        // mutation — no other action is legal while the choice is pending, so it's still intact).
        let count = cards.len();
        let remainder: Vec<ObjectId> = self.players[player.0 as usize].library[count..].to_vec();

        let mut events = Vec::new();
        for &from in &to_hand {
            let event = Event::SearchedToHand {
                player,
                object: self.next_object_id(),
                from,
                card: self.def_of(from),
            };
            self.push_apply(&mut events, event);
        }
        for &from in &to_exile_may_play {
            let event = Event::ExiledFromLibraryMayPlay {
                player,
                card: self.next_object_id(),
                from,
                until_next_turn: false,
            };
            self.push_apply(&mut events, event);
        }

        // ponytail: Expressive Iteration's "bottom" slot is always exactly one card (no printed
        // "random order," unlike `LookAtTop`'s rest pile) — a direct rebuild is faithful as-is;
        // revisit only if a second `DistributeTop` card bottoms more than one card at once.
        let mut library = remainder;
        library.extend(to_bottom);
        self.players[player.0 as usize].library = library;

        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseManaColor`] (CR 106.4's "add N mana of any one color" —
    /// Lotus Field, Kami of Whispered Hopes): the pending `amount` is added as one `color` credit
    /// — no `Mana::Any` involved, so the credits can't later split across colors at payment time.
    pub(crate) fn choose_mana_color(
        &mut self,
        player: PlayerId,
        color: Color,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseManaColor { amount, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::ManaAdded {
                player,
                mana: Mana::Color(color),
                amount,
                persist: false,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCreatureType`] (CR 614.12/700.9-style "as ~ enters,
    /// choose a creature type" — Patchwork Banner): `subtype` must name one of the pending
    /// choice's offered `options`; the matching entry (not the wire string itself) is stored on
    /// `source`, keeping [`Permanent::chosen_subtype`] a leaked `&'static str` like every other
    /// subtype field. `_player` isn't needed beyond `submit`'s choice-gate actor check — unlike
    /// [`Game::choose_mana_color`], nothing here is scoped by player.
    pub(crate) fn choose_creature_type(
        &mut self,
        _player: PlayerId,
        subtype: String,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCreatureType {
            source, options, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        let Some(&chosen) = options.iter().find(|&&t| t == subtype) else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::CreatureTypeChosen {
                object: source,
                subtype: chosen,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseColor`] (CR 614.12/700.9-style "as ~ enters, choose a
    /// color" — Flickering Ward): store the chosen `color` on `source`
    /// ([`Permanent::chosen_color`]). Any of the five colors is a legal answer (no game-state
    /// legality to violate), so `color` is taken as given. `_player` isn't needed beyond `submit`'s
    /// choice-gate actor check.
    pub(crate) fn choose_color(
        &mut self,
        _player: PlayerId,
        color: Color,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseColor { source, .. }) = self.pending_choice.clone() else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::ColorChosen {
                object: source,
                color,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ShuffleFromGraveyard`]: shuffle up to `max` (`0` = unbounded) of
    /// the offered `candidates` from `owner`'s graveyard into `owner`'s library, via the same
    /// [`Event::TuckedToLibrary`] zone move [`Effect::TuckFromGraveyard`] uses, then shuffle that
    /// library (CR 701.19-style mandatory shuffle after cards enter it).
    pub(crate) fn shuffle_from_graveyard(
        &mut self,
        // Already validated against the pending choice's answerer by `Game::submit`'s generic
        // choice-actor check; the graveyard/library affected is `owner` (from the pending
        // choice), read below — this may differ from the answering player (Quandrix Command).
        _player: PlayerId,
        chosen: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ShuffleFromGraveyard {
            candidates,
            owner,
            max,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if max != 0 && chosen.len() as u32 > max {
            return Err(Reject::IllegalChoice);
        }
        for (i, &id) in chosen.iter().enumerate() {
            if !candidates.contains(&id) || chosen[..i].contains(&id) {
                return Err(Reject::IllegalChoice);
            }
        }
        self.finish_answer();

        let mut events = Vec::new();
        for &from in &chosen {
            self.push_apply(
                &mut events,
                Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from,
                    to_top: false,
                },
            );
        }
        self.push_apply(&mut events, Event::LibraryShuffled { player: owner });
        Ok(events)
    }

    /// Answer a [`PendingChoice::SearchLibrary`]: move the chosen card (one of the offered
    /// matches) to its destination, or fail to find (`choice = None`). A found pick with more
    /// finds remaining and matches still on offer re-pauses for the next pick instead of
    /// shuffling (CR 701.19f — an "up to N" search shuffles once, after the last pick);
    /// otherwise (search exhausted, no matches left, or a fail-to-find) the library is shuffled
    /// and the search ends. When `overflow` is set (Cultivate), the re-pause routes every find
    /// *after this one* to `overflow` instead of `dest` — the first find still lands on `dest`.
    pub(crate) fn search_library(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SearchLibrary {
            matches,
            dest,
            tapped,
            remaining,
            overflow,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        // A found card must be one of the offered matches; `None` (fail to find) is always legal.
        if choice.is_some_and(|c| !matches.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        let Some(from) = choice else {
            // Fail to find ends the search outright (CR 701.19c is always legal).
            self.shuffle(player);
            return Ok(events);
        };
        let event = match dest {
            SearchDest::Hand => Event::SearchedToHand {
                player,
                object: self.next_object_id(),
                from,
                card: self.def_of(from),
            },
            SearchDest::Battlefield => Event::SearchedToBattlefield {
                permanent: self.next_object_id(),
                from,
                controller: player,
                tapped,
            },
            // Enlightened Tutor / Sterling Grove: the found card is revealed in place (CR
            // 701.30) — it hasn't left the library yet, so no zone-move event here; the shuffle
            // and top-placement below finish the job.
            SearchDest::LibraryTop => Event::RevealedTopOfLibrary {
                player,
                card: from,
                def: self.def_of(from),
            },
        };
        self.push_apply(&mut events, event);

        let remaining = remaining.saturating_sub(1);
        let still_matching: Vec<ObjectId> = matches.into_iter().filter(|&id| id != from).collect();
        if remaining > 0 && !still_matching.is_empty() {
            // Every find after the first routes to `overflow` if the search has one (Cultivate:
            // first pick lands on `dest` = battlefield tapped, every later pick on `overflow` =
            // hand); a search with no overflow (Land Tax) keeps re-pausing on the same `dest`.
            self.pause_for(PendingChoice::SearchLibrary {
                player,
                matches: still_matching,
                dest: overflow.unwrap_or(dest),
                tapped,
                remaining,
                overflow,
            });
            return Ok(events);
        }
        // Always shuffle at the true end of the search (CR 701.19). ponytail: library order
        // isn't event-sourced (like scry / `shuffle`) — mutate it directly.
        if dest == SearchDest::LibraryTop {
            // "…then shuffle and put that card on top" — `from` never left the library, so put
            // it back on top after the shuffle instead of shuffling it in with everything else.
            self.shuffle_then_put_on_top(player, from);
        } else {
            self.shuffle(player);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PutLandFromHand`]: put the chosen land (one of the offered
    /// candidates) onto the battlefield, or decline (`choice = None`).
    pub(crate) fn put_land_from_hand(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PutLandFromHand {
            candidates, tapped, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if choice.is_some_and(|c| !candidates.contains(&c)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        if let Some(from) = choice {
            self.push_apply(
                &mut events,
                Event::PutOntoBattlefieldFromHand {
                    permanent: self.next_object_id(),
                    from,
                    controller: player,
                    tapped,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::CastCreatureFaceDown`]: cast the chosen hand creature (one of the
    /// offered candidates) face down as a 2/2 creature spell (CR 708.2) without paying its mana
    /// cost, or decline (`choice = None`).
    pub(crate) fn cast_creature_face_down(
        &mut self,
        player: PlayerId,
        choice: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::CastCreatureFaceDown { candidates, .. }) =
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
            // No mana was spent casting it (cast "without paying its mana cost"), so no colors.
            // Masked (CR 615): Illusionary Mask's face-down creature turns face up when it would
            // assign or deal damage, be dealt damage, or become tapped.
            self.push_face_down_spell_cast(player, card, [false; Color::COUNT], true, &mut events);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseMode`]: resolve the chosen mode of a "choose one" triggered
    /// ability ([`Effect::ChooseOne`]) through the ordinary resolution pipeline, carrying the
    /// trigger's own `source`/`target`/`x` context. The chosen sub-effect may itself pause.
    pub(crate) fn answer_choose_mode(
        &mut self,
        player: PlayerId,
        mode: usize,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseMode {
            source,
            target,
            x,
            modes,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if mode >= modes.len() {
            return Err(Reject::IllegalMode);
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.run(
            modes[mode],
            ResolveCtx {
                controller: player,
                source,
                target,
                targets_second: TargetList::default(),
                x,
                spent_mana: [0; 6],
            },
            &mut events,
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseTriggerModes`]: a modal *triggered* ability's "choose N"
    /// (CR 700.2) — `modes` names `choose` distinct (printed-mode index, chosen Player target)
    /// pairs. An empty `modes` is legal only when the choice is `optional` (CR "you may") and
    /// drops the whole ability with no events. Otherwise: exactly `choose` distinct mode indices,
    /// each in range and paired with a target legal for that mode's own [`Effect::target`], and
    /// every chosen target a pairwise-distinct player (Shadrix Silverquill's "each mode must
    /// target a different player").
    /// ponytail: the pairwise-distinct-player rule is enforced unconditionally here — see
    /// [`PendingChoice::ChooseTriggerModes`]'s doc.
    pub(crate) fn answer_choose_trigger_modes(
        &mut self,
        player: PlayerId,
        modes: Vec<(usize, Option<Target>)>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseTriggerModes {
            source,
            modes: mode_effects,
            choose,
            optional,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if modes.is_empty() {
            if !optional {
                return Err(Reject::IllegalChoice);
            }
            self.finish_answer();
            return Ok(Vec::new());
        }
        if modes.len() != choose as usize {
            return Err(Reject::IllegalMode);
        }
        let def = self.def_of(source);
        let x = self.ability_source_x(source);
        let mut seen_modes: Vec<usize> = Vec::new();
        let mut seen_players: Vec<PlayerId> = Vec::new();
        let mut resolved: Vec<(Effect, Option<Target>)> = Vec::new();
        for (m, target) in modes {
            let (Some(&effect), false) = (mode_effects.get(m), seen_modes.contains(&m)) else {
                return Err(Reject::IllegalMode);
            };
            seen_modes.push(m);
            let spec = effect.target();
            if spec == TargetSpec::None {
                if target.is_some() {
                    return Err(Reject::IllegalTarget);
                }
                resolved.push((effect, None));
                continue;
            }
            let Some(Target::Player(chosen_player)) = target else {
                return Err(Reject::IllegalTarget);
            };
            if seen_players.contains(&chosen_player)
                || !self
                    .legal_targets_for(spec, source, player, color_identity(def), x)
                    .contains(&target.expect("checked Some above"))
            {
                return Err(Reject::IllegalTarget);
            }
            seen_players.push(chosen_player);
            resolved.push((effect, target));
        }
        self.finish_answer();

        let mut events = Vec::new();
        self.push_ability_group(player, source, &resolved, false, &mut events);
        Ok(events)
    }
}
