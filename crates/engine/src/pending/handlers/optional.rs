//! May / pay-or / echo / sacrifice-unless answers.

use crate::*;

impl Game {
    pub(crate) fn answer_may(&mut self, player: PlayerId, yes: bool) -> Result<Vec<Event>, Reject> {
        if let Some(PendingChoice::MayRevealLandFromHand { land, subtypes, .. }) =
            self.pending_choice.clone()
        {
            return self.answer_may_reveal_land_from_hand(player, land, subtypes, yes);
        }
        let Some(PendingChoice::MayYesNo {
            source,
            effect,
            resume,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();
        // Island Sanctuary's draw-step replacement: "yes" skips the draw and arms the shield,
        // "no" takes the draw the pause stood in front of (dredge's own replacement included).
        // Either way this resumes the step loop the pause interrupted, as a dredge answer does.
        if let MayYesNoResume::SkipDrawStepDraw = resume {
            let Effect::Static(StaticEffect::MaySkipDrawForCantBeAttackedBy { filter }) = effect
            else {
                return Err(Reject::IllegalChoice);
            };
            let mut events = Vec::new();
            if yes {
                self.combat_extras
                    .repelled_until_next_turn
                    .push((player, filter));
            } else {
                self.draw_step_draw(player, &mut events);
            }
            // A declined skip can land on dredge's own pause, which resumes the step loop itself.
            if self.pending_choice.is_none() {
                events.extend(self.advance_step());
            }
            return Ok(events);
        }
        if let MayYesNoResume::TradeSecretsRepeat { caster, max } = resume {
            if !yes {
                return Ok(Vec::new());
            }
            let events = self.draw_events(player, 2);
            self.apply_all(&events);
            pending::raise_choice(
                self,
                PendingChoice::MayDrawUpTo {
                    player: caster,
                    max,
                    effect: Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat {
                        count: Amount::Fixed(i32::from(max)),
                    }),
                    resume: MayDrawUpToResume::TradeSecretsRepeat {
                        opponent: player,
                        source,
                    },
                },
            );
            return Ok(events);
        }
        let mut events = Vec::new();
        if yes {
            // A resolution-time "may copy this spell" rider (`Effect::Copy(CopyEffect::ThisSpell)`'s
            // `optional` gate, CR 707.10c — Sevinne's Reclamation) mints inline as part of the
            // still-resolving spell rather than going on the stack as a new triggered ability — (CR 603, CR 405, CR 601)
            // the mandatory storm/Gravestorm mint this mirrors never leaves the stack either.
            if let Effect::Copy(CopyEffect::ThisSpell { count, .. }) = effect {
                self.mint_spell_copies(count, player, source, None, 0, &mut events);
            } else if let Effect::Copy(CopyEffect::Demonstrate { spell }) = effect {
                // Demonstrate's controller copy mints only once an opponent is chosen for the
                // second copy (CR 702.147a "choose an opponent to also copy it") — see the
                // `Effect::Copy(CopyEffect::Demonstrate)` branch in `Game::choose_targets`.
                let legal: Vec<Target> = self
                    .legal_targets_for(
                        TargetSpec::OpponentPlayer,
                        spell,
                        player,
                        [false; Color::COUNT],
                        0,
                    )
                    .into_iter()
                    .collect();
                // ponytail: no legal opponent is unreachable in a real (2+ player) Commander game.
                if legal.is_empty() {
                    return Ok(events);
                }
                pending::raise(
                    self,
                    pending::ChoiceRequest::ChooseTarget {
                        player,
                        source,
                        effect: Effect::Copy(CopyEffect::Demonstrate { spell }),
                        legal,
                        count: TargetCount::default(),
                        x: 0,
                        activated: false,
                    },
                );
            } else if let Effect::Choice(ChoiceEffect::TargetPlayerMayDraw { count, .. }) = effect {
                // Questing Phelddagrif's blue rider: `player` here is the *targeted* opponent who
                // just answered "yes" (not the ability's own controller, unlike every other arm
                // in this function) — draw them `count` cards directly, no further pause (CR
                // 601.2c: no pay window rides behind this rider).
                let n = self.resolve_count(count, player, source, None, 0);
                let evs = self.draw_events(player, n);
                self.apply_all(&evs);
                events.extend(evs);
            } else if let Effect::Choice(ChoiceEffect::DamagingCreatureControllerMayDraw {
                count,
                ..
            }) = effect
            {
                // Edric: `player` here is the controller of the creature that dealt the combat
                // damage (baked in at trigger placement), not Edric's own controller — draw them
                // `count` cards directly, the same shape as `TargetPlayerMayDraw` above.
                let evs = self.draw_events(player, count);
                self.apply_all(&evs);
                events.extend(evs);
            } else if let Effect::Choice(ChoiceEffect::MayDrawUnlessPays { cost, caster }) = effect
            {
                // Rhystic Study: `player` (the controller) said they want to draw, so now
                // `caster` (the triggering opponent, baked in by `contextualize_effect`) gets a
                // chance to pay `cost` and stop it — see `Game::pay_or_controller_draws`.
                let caster = caster.expect(
                    "caster baked in by contextualize_effect at CastSpell trigger placement",
                );
                let generic = self.resolve_count(cost, player, source, None, 0);
                pending::raise_choice(
                    self,
                    PendingChoice::PayOrControllerDraws {
                        player: caster,
                        controller: player,
                        cost: Cost {
                            generic: generic as u8,
                            ..Cost::FREE
                        },
                    },
                );
            } else if let Effect::Dig(DigEffect::MayShuffleTargetPlayersLibrary { owner }) = effect
            {
                // Natural Selection: the caster said yes, so the player whose library they just
                // sorted shuffles it — order and all.
                let owner =
                    owner.expect("the targeted player is baked in when the yes/no is raised");
                let event = Event::LibraryShuffled { player: owner };
                self.apply(&event);
                events.push(event);
            } else if matches!(resume, MayYesNoResume::ResolveInline)
                && matches!(effect, Effect::Dig(DigEffect::SearchLibrary { .. }))
            {
                // Mid-resolution "may search" (`DigEffect::SearchLibrary::optional` +
                // `MayYesNoResume::ResolveInline`): accepting re-runs the (now non-optional)
                // search under the answering player as part of the still-resolving ability —
                // not a fresh stack object. Ability-level optional tutors (Borderland Ranger)
                // keep `MayYesNoResume::Default` and fall through to `place_targeted_ability`.
                self.run(
                    effect,
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
            } else {
                // A targeted "may" (Sun Titan) pauses again to choose its target; a targetless
                // one (Solemn's dies-draw) goes straight on the stack. NoLegalTarget = accepted
                // but nothing to aim at, so it fizzles harmlessly.
                self.place_targeted_ability(player, source, effect, 0, false, &mut events);
            }
        } else if matches!(resume, MayYesNoResume::ResolveInline)
            && matches!(effect, Effect::Dig(DigEffect::SearchLibrary { .. }))
        {
            // Declining a mid-resolution optional search still advances an AllPlayers fan-out
            // (Veteran Explorer style) so later seats aren't dropped; a single-searcher decline
            // is a no-op. Ability-level optional declines (Default resume) do nothing here.
            self.continue_search_fanout();
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayRevealLandFromHand`]: yes reveals one matching hand land and
    /// plays `land` untapped; no plays it tapped with no reveal.
    pub(crate) fn answer_may_reveal_land_from_hand(
        &mut self,
        player: PlayerId,
        land: ObjectId,
        subtypes: &'static [&'static str],
        yes: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayRevealLandFromHand { .. }) = self.pending_choice else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        let revealed = yes
            .then(|| self.first_hand_land_with_subtype(player, subtypes))
            .flatten();
        if let Some(card) = revealed {
            events.push(Event::RevealedFromHand {
                player,
                card,
                def: self.def_id_of(card),
            });
        }
        let permanent = self.next_object_id();
        let printed = card_def(self.def_id_of(land));
        events.push(Event::LandPlayed {
            permanent,
            from: land,
            player,
            tapped: revealed.is_none(),
        });
        self.apply_all(&events);
        self.push_enters_with_counters(&printed, permanent, player, None, 0, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayDrawUpTo`] (CR 120.4 / 601.2c — Arcane Denial's "may draw up to
    /// two cards", Trade Secrets' caster draw, and similar count choices): draw exactly `count`
    /// cards, any number `0..=max`. An out-of-range `count` is rejected with the pause left live so
    /// the player can answer again.
    pub(crate) fn answer_may_draw_up_to(
        &mut self,
        player: PlayerId,
        count: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayDrawUpTo { max, resume, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if count > max {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();
        let events = self.draw_events(player, count as u32);
        self.apply_all(&events);
        if let MayDrawUpToResume::TradeSecretsRepeat { opponent, source } = resume {
            pending::raise_choice(
                self,
                PendingChoice::MayYesNo {
                    player: opponent,
                    source,
                    effect: Effect::Choice(ChoiceEffect::MayDrawUpToThenOpponentMayRepeat {
                        count: Amount::Fixed(i32::from(max)),
                    }),
                    resume: MayYesNoResume::TradeSecretsRepeat {
                        caster: player,
                        max,
                    },
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCost`]: pay the cost to get the optional trigger, or decline.
    /// An unaffordable "pay" leaves the choice pending so the player can still decline.
    /// When `cost.additional.discard > 0`, `discard_cost` must name that many distinct hand cards
    /// (Conspiracy Theorist's "pay {1} and discard a card") — paying is all-or-nothing with the
    /// mana; a short/illegal discard list rejects without settling either half.
    pub(crate) fn pay_optional_cost(
        &mut self,
        player: PlayerId,
        pay: bool,
        discard_cost: &[ObjectId],
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayCost {
            source,
            cost,
            effect,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let mut events = Vec::new();
        if !pay {
            self.finish_answer();
            return Ok(events);
        }
        let discard_n = cost.additional.discard as usize;
        if discard_cost.len() != discard_n {
            return Err(Reject::IllegalChoice);
        }
        let hand = self.hand_of(player);
        let mut named: Vec<ObjectId> = Vec::with_capacity(discard_n);
        for &id in discard_cost {
            if named.contains(&id) || !hand.contains(&id) {
                return Err(Reject::IllegalChoice);
            }
            named.push(id);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped / discarded.
        self.settle_payment(player, cost, None, None, &mut events)?;
        for &id in &named {
            let card = self.next_object_id();
            let def = self.def_id_of(id);
            self.push_apply(&mut events, Event::MovedToGraveyard { card, from: id });
            self.push_apply(
                &mut events,
                Event::Discarded {
                    card,
                    from: id,
                    def,
                    player,
                },
            );
        }
        self.finish_answer();
        if let Effect::Copy(CopyEffect::ThisSpell { count, .. }) = effect {
            // Chain Lightning's reflexive rider (`Effect::Copy(CopyEffect::MayPayToCopyThis)`): mint inline as part
            // of the still-resolving spell, matching `Game::answer_may`'s optional-copy shape,
            // rather than placing a fresh ability — `source` is that still-resolving spell, and
            // the copy mints under `player`, the PAYER (this pause's reflexively-targeted damaged
            // player/controller), not the ability's own controller.
            self.mint_spell_copies(count, player, source, None, 0, &mut events);
        } else {
            // A targeted paid trigger pauses to choose its target; a targetless one goes on the stack.
            self.place_targeted_ability(player, source, effect, 0, false, &mut events);
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCost`] whose `cost` carries a chosen `{X}` (CR 107.3 —
    /// Decree of Justice's "When you cycle this card, you may pay {X}."): pay `cost.with_x(x)`
    /// to get the optional trigger, threading `x` onto the placed ability the same way an
    /// activated ability's own `{X}` cost does (see [`Game::push_ability_group_with_x`]), so its
    /// `Amount::X` reads the chosen value — or decline (`x` ignored). An unaffordable "pay"
    /// leaves the choice pending so the player can still decline, mirroring
    /// [`Game::pay_optional_cost`].
    /// ponytail: targetless only (`push_ability_group_with_x` skips the target-choice dance in
    /// [`Game::place_targeted_ability`]) — no pool card pairs an `{X}`-cost optional trigger with
    /// a target; route through `place_targeted_ability`'s own X-threading path if one ever does.
    pub(crate) fn pay_optional_cost_with_x(
        &mut self,
        player: PlayerId,
        pay: bool,
        x: u32,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayCost {
            source,
            cost,
            effect,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let mut events = Vec::new();
        if !pay {
            self.finish_answer();
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall, folding the chosen `{X}`
        // into generic per CR 107.3); unaffordable leaves the choice pending with nothing tapped.
        self.settle_payment(player, cost.with_x(x), None, None, &mut events)?;
        self.finish_answer();
        self.push_ability_group_with_x(
            player,
            source,
            &[(effect, None)],
            x,
            [0; 6],
            false,
            &mut events,
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayOrCounter`]: pay `cost` to save the target spell, or decline
    /// and let it be countered. The mirror image of [`Game::pay_optional_cost`] — same
    /// [`Intent::PayOptionalCost`] shape, opposite default (declining here *does* something: the
    /// counter). An unaffordable "pay" leaves the choice pending so the player can still decline.
    /// ponytail: reuses `PayOptionalCost` rather than a dedicated intent — the wire shape (a bare
    /// pay/decline bool) is identical, and `Game::submit`'s choice gate already routes by the
    /// pending choice's kind, not the intent's.
    pub(crate) fn pay_or_counter(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayOrCounter {
            cost,
            spell,
            strips_mana_on_decline,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut evs = self.counter_spell(spell);
            self.apply_all(&evs);
            // Power Sink's "if that player doesn't, they tap all lands with mana abilities they
            // control and lose all unspent mana" — the penalty rides on this decline, which is why
            // it lives here and not as a following resolution step. A plain tap, not a tap for
            // mana (CR 106.11): nothing is produced, and the pool goes next anyway.
            if strips_mana_on_decline {
                let taps: Vec<Event> = self
                    .battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.controller_of(id) == player
                            && !self.is_tapped(id)
                            && self.taps_for_mana(id)
                            && self.effective_types(id).intersects(TypeSet::LAND)
                    })
                    .map(|object| Event::Tapped { object })
                    .collect();
                self.apply_all(&taps);
                evs.extend(taps);
                let drain = Event::ManaEmptied {
                    player,
                    end_of_turn: true,
                    to: None,
                };
                self.apply(&drain);
                evs.push(drain);
            }
            return Ok(evs);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        // Paying leaves the spell on the stack — it resolves normally, untouched.
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayOrControllerDraws`]: `player` (the triggering opponent) pays
    /// `cost` to stop `controller`'s draw, or declines and lets it happen — Rhystic Study's
    /// "unless that player pays {1}". Same [`Intent::PayOptionalCost`] shape and "declining does
    /// something" polarity as [`Game::pay_or_counter`], but the "something" is a draw rather than
    /// a counter.
    pub(crate) fn pay_or_controller_draws(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayOrControllerDraws {
            controller, cost, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let evs = self.draw_events(controller, 1);
            self.apply_all(&evs);
            return Ok(evs);
        }
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        // Paying stops the draw outright — nothing further happens.
        Ok(events)
    }

    /// Answer a [`PendingChoice::ChooseCounteredSpellDestination`] (Hinder's CR 701.5b rider):
    /// `top` puts the already-countered `spell` on top of its owner's library instead of the
    /// bottom. `_player` isn't needed beyond `submit`'s choice-gate actor check (like
    /// [`Game::choose_color`]).
    pub(crate) fn choose_countered_spell_destination(
        &mut self,
        _player: PlayerId,
        top: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::ChooseCounteredSpellDestination { spell, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        self.push_apply(
            &mut events,
            Event::TuckedToLibrary {
                card: self.next_object_id(),
                from: spell,
                to_top: top,
                second_from_top: false,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayEchoOrSacrifice`]: pay Echo's cost to keep `source`, or
    /// decline and sacrifice it (CR 702.31d). The permanent-scoped twin of
    /// [`Game::pay_or_counter`] — same [`Intent::PayOptionalCost`] shape and "declining does
    /// something" polarity (there, countering the spell; here, sacrificing the source). An
    /// unaffordable "pay" leaves the choice pending so the player can still decline.
    pub(crate) fn pay_echo(&mut self, player: PlayerId, pay: bool) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayEchoOrSacrifice { source, cost, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut events = Vec::new();
            self.run(
                Effect::Sacrifice(SacrificeEffect::Object {
                    object: Some(source),
                }),
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
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        // CR 702.31e: this upkeep is now "since your last upkeep" — echo won't ask again.
        self.permanent_mut(source).echo_unpaid = false;
        self.finish_answer();
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCumulativeUpkeepOrSacrifice`]: an empty `cards` list
    /// declines and sacrifices `source` (CR 702.24a), the same "declining does something"
    /// polarity as [`Game::pay_echo`]; otherwise `cards` must be exactly `count` ids from
    /// `options` that all share one owner (CR "a single graveyard") — each is put on the bottom
    /// of that owner's library ([`Event::TuckedToLibrary`], the same zone move Mistveil Plains's
    /// `Effect::Zone(ZoneEffect::TuckFromGraveyard)` uses). An invalid non-empty answer (wrong count, mixed
    /// owners, an id not offered) rejects, leaving the choice pending so the player can still
    /// decline.
    pub(crate) fn pay_cumulative_upkeep(
        &mut self,
        player: PlayerId,
        cards: Vec<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayCumulativeUpkeepOrSacrifice {
            source,
            options,
            count,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if cards.is_empty() {
            self.finish_answer();
            let mut events = Vec::new();
            self.run(
                Effect::Sacrifice(SacrificeEffect::Object {
                    object: Some(source),
                }),
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
            return Ok(events);
        }

        let distinct = cards.iter().collect::<std::collections::HashSet<_>>().len();
        let all_offered = cards.iter().all(|c| options.contains(c));
        let single_owner = cards
            .windows(2)
            .all(|w| self.owner_of(w[0]) == self.owner_of(w[1]));
        if cards.len() != count as usize || distinct != cards.len() || !all_offered || !single_owner
        {
            return Err(Reject::IllegalChoice); // invalid — the choice stays pending
        }

        self.finish_answer();
        let mut events = Vec::new();
        for &from in &cards {
            let card = self.next_object_id();
            self.push_apply(
                &mut events,
                Event::TuckedToLibrary {
                    card,
                    from,
                    to_top: false,
                    second_from_top: false,
                },
            );
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayRecoverOrExile`]: pay Recover's cost to return `source` from
    /// the graveyard to hand, or decline and exile it (CR 702.59a). The graveyard-scoped twin of
    /// [`Game::pay_echo`] — same [`Intent::PayOptionalCost`] shape and "declining does something"
    /// polarity (there, sacrificing a battlefield permanent; here, exiling a graveyard card, so
    /// the events are pushed directly rather than routed through `Effect::Destroy(DestroyEffect::SacrificeObject)`, which
    /// only knows battlefield objects). An unaffordable "pay" leaves the choice pending so the
    /// player can still decline.
    pub(crate) fn pay_recover(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayRecoverOrExile { source, cost, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        let mut events = Vec::new();
        if !pay {
            self.finish_answer();
            let event = self.exile_or_command(source, self.next_object_id());
            self.push_apply(&mut events, event);
            return Ok(events);
        }
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        self.push_apply(
            &mut events,
            Event::ReturnedToHand {
                card: self.next_object_id(),
                from: source,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayOrElse`]: pay `cost`, or decline and take the card's printed
    /// penalty (CR 701.16) — usually sacrificing `source` (Rupture Spire, Phantasmal Forces),
    /// sometimes not (Force of Nature's 8 damage). The twin of [`Game::pay_echo`] — same
    /// [`Intent::PayOptionalCost`] shape and polarity, kept as its own handler since it isn't
    /// Echo (see the variant's doc). An unaffordable "pay" leaves the choice pending.
    pub(crate) fn pay_sacrifice_unless(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayOrElse {
            source,
            cost,
            otherwise,
            ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut events = Vec::new();
            for effect in otherwise {
                self.run(
                    effect.clone(),
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
            return Ok(events);
        }
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayLifeOrEntersTapped`]: pay `life` for the land to enter
    /// untapped, or decline and have it enter tapped (CR 614.12 — Overgrown Tomb's "As this
    /// land enters, you may pay 2 life. If you don't, it enters tapped."). The land-drop twin of
    /// [`Game::pay_sacrifice_unless`] — same [`Intent::PayOptionalCost`] shape, opposite
    /// consequence (there, sacrifice; here, tapped). `source` is still the land *card*: the
    /// permanent doesn't exist until this answer mints [`Event::LandPlayed`] (CR 614.12's
    /// replacement locks in before the land is on the battlefield).
    pub(crate) fn pay_life_or_enters_tapped(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::PayLifeOrEntersTapped { source, life, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();

        let mut events = Vec::new();
        if pay {
            self.push_apply(
                &mut events,
                Event::LifeChanged {
                    player,
                    amount: -(life as i32),
                    source: Some(source),
                },
            );
        }
        let def = self.def_of(source);
        let permanent = self.next_object_id();
        self.push_apply(
            &mut events,
            Event::LandPlayed {
                permanent,
                from: source,
                player,
                tapped: !pay,
            },
        );
        self.push_enters_with_counters(&def, permanent, player, None, 0, &mut events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::SacrificeUnlessReturnLand`]: `land` (one of the offered
    /// candidates) returns to its owner's hand and `source` stays; `None` declines and
    /// sacrifices `source` instead (CR 701.16).
    pub(crate) fn return_land_or_sacrifice(
        &mut self,
        player: PlayerId,
        land: Option<ObjectId>,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SacrificeUnlessReturnLand {
            source, candidates, ..
        }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        if land.is_some_and(|l| !candidates.contains(&l)) {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();

        let mut events = Vec::new();
        match land {
            None => self.run(
                Effect::Sacrifice(SacrificeEffect::Object {
                    object: Some(source),
                }),
                ResolveCtx {
                    controller: player,
                    source,
                    target: None,
                    targets_second: TargetList::default(),
                    x: 0,
                    spent_mana: [0; 6],
                },
                &mut events,
            ),
            Some(chosen) => {
                let card = self.next_object_id();
                self.push_apply(&mut events, Event::ReturnedToHand { card, from: chosen });
            }
        }
        Ok(events)
    }
}
