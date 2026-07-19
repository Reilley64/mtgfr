//! May / pay-or / echo / sacrifice-unless answers.

use crate::*;

impl Game {
    pub(crate) fn answer_may(&mut self, player: PlayerId, yes: bool) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayYesNo { source, effect, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();
        let mut events = Vec::new();
        if yes {
            // A resolution-time "may copy this spell" rider (`Effect::CopyThisSpell`'s
            // `optional` gate, CR 707.10c — Sevinne's Reclamation) mints inline as part of the
            // still-resolving spell rather than going on the stack as a new triggered ability — (CR 603, CR 405, CR 601)
            // the mandatory storm/Gravestorm mint this mirrors never leaves the stack either.
            if let Effect::CopyThisSpell { count, .. } = effect {
                self.mint_spell_copies(count, player, source, None, 0, &mut events);
            } else if let Effect::Demonstrate { spell } = effect {
                // Demonstrate's controller copy mints only once an opponent is chosen for the
                // second copy (CR 702.147a "choose an opponent to also copy it") — see the
                // `Effect::Demonstrate` branch in `Game::choose_targets`.
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
                        effect: Effect::Demonstrate { spell },
                        legal,
                        count: TargetCount::default(),
                        x: 0,
                        activated: false,
                    },
                );
            } else if let Effect::TargetPlayerMayDraw { count, .. } = effect {
                // Questing Phelddagrif's blue rider: `player` here is the *targeted* opponent who
                // just answered "yes" (not the ability's own controller, unlike every other arm
                // in this function) — draw them `count` cards directly, no further pause (CR
                // 601.2c: no pay window rides behind this rider).
                let n = self.resolve_count(count, player, source, None, 0);
                let evs = self.draw_events(player, n);
                self.apply_all(&evs);
                events.extend(evs);
            } else if let Effect::MayDrawUnlessPays { cost, caster } = effect {
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
            } else {
                // A targeted "may" (Sun Titan) pauses again to choose its target; a targetless
                // one (Solemn's dies-draw) goes straight on the stack. NoLegalTarget = accepted
                // but nothing to aim at, so it fizzles harmlessly.
                self.place_targeted_ability(player, source, effect, 0, false, &mut events);
            }
        }
        Ok(events)
    }

    /// Answer a [`PendingChoice::MayDrawUpTo`] (CR 120.4 / 601.2c — Arcane Denial's "may draw up to
    /// two cards"): draw exactly `count` cards, any number `0..=max`. An out-of-range `count` is
    /// rejected with the pause left live so the player can answer again.
    pub(crate) fn answer_may_draw_up_to(
        &mut self,
        player: PlayerId,
        count: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::MayDrawUpTo { max, .. }) = self.pending_choice else {
            return Err(Reject::IllegalChoice);
        };
        if count > max {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();
        let events = self.draw_events(player, count as u32);
        self.apply_all(&events);
        Ok(events)
    }

    /// Answer a [`PendingChoice::TradeSecretsCasterDraw`]: draw exactly `count` cards (`0..=max`),
    /// then pause `opponent` on [`PendingChoice::TradeSecretsRepeat`] to decide whether to run the
    /// whole process again. An out-of-range `count` is rejected with the pause left live.
    pub(crate) fn answer_trade_secrets_caster_draw(
        &mut self,
        player: PlayerId,
        count: u8,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::TradeSecretsCasterDraw {
            max,
            opponent,
            source,
            ..
        }) = self.pending_choice
        else {
            return Err(Reject::IllegalChoice);
        };
        if count > max {
            return Err(Reject::IllegalChoice);
        }
        self.finish_answer();
        let events = self.draw_events(player, count as u32);
        self.apply_all(&events);
        pending::raise_choice(
            self,
            PendingChoice::TradeSecretsRepeat {
                player: opponent,
                caster: player,
                max,
                source,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::TradeSecretsRepeat`]: on `yes`, `player` (the target opponent)
    /// draws two more cards, then `caster` is paused again on
    /// [`PendingChoice::TradeSecretsCasterDraw`] (self-rescheduling, like
    /// [`Game::dance_exile_more`]). On `no`, the loop stops.
    pub(crate) fn answer_trade_secrets_repeat(
        &mut self,
        player: PlayerId,
        yes: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::TradeSecretsRepeat {
            caster,
            max,
            source,
            ..
        }) = self.pending_choice
        else {
            return Err(Reject::IllegalChoice);
        };
        self.finish_answer();
        if !yes {
            return Ok(Vec::new());
        }
        let events = self.draw_events(player, 2);
        self.apply_all(&events);
        pending::raise_choice(
            self,
            PendingChoice::TradeSecretsCasterDraw {
                player: caster,
                max,
                opponent: player,
                source,
            },
        );
        Ok(events)
    }

    /// Answer a [`PendingChoice::PayCost`]: pay the cost to get the optional trigger, or decline.
    /// An unaffordable "pay" leaves the choice pending so the player can still decline.
    pub(crate) fn pay_optional_cost(
        &mut self,
        player: PlayerId,
        pay: bool,
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
        // Settle the mana (auto-tapping lands for a pool shortfall); unaffordable leaves the
        // choice pending with nothing tapped.
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
        if let Effect::CopyThisSpell { count, .. } = effect {
            // Chain Lightning's reflexive rider (`Effect::MayPayToCopyThis`): mint inline as part
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
        let Some(PendingChoice::PayOrCounter { cost, spell, .. }) = self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let evs = self.counter_spell(spell);
            self.apply_all(&evs);
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
                Effect::SacrificeObject {
                    object: Some(source),
                },
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
    /// `Effect::TuckFromGraveyard` uses). An invalid non-empty answer (wrong count, mixed
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
                Effect::SacrificeObject {
                    object: Some(source),
                },
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
    /// the events are pushed directly rather than routed through `Effect::SacrificeObject`, which
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

    /// Answer a [`PendingChoice::SacrificeUnlessPay`]: pay `cost` to keep `source`, or decline
    /// and sacrifice it (CR 701.16). Rupture Spire's own-ETB twin of [`Game::pay_echo`] — same
    /// [`Intent::PayOptionalCost`] shape and polarity, kept as its own handler since it isn't
    /// Echo (see the variant's doc). An unaffordable "pay" leaves the choice pending.
    pub(crate) fn pay_sacrifice_unless(
        &mut self,
        player: PlayerId,
        pay: bool,
    ) -> Result<Vec<Event>, Reject> {
        let Some(PendingChoice::SacrificeUnlessPay { source, cost, .. }) =
            self.pending_choice.clone()
        else {
            return Err(Reject::IllegalChoice);
        };

        if !pay {
            self.finish_answer();
            let mut events = Vec::new();
            self.run(
                Effect::SacrificeObject {
                    object: Some(source),
                },
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
        let mut events = Vec::new();
        self.settle_payment(player, cost, None, None, &mut events)?;
        self.finish_answer();
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
                Effect::SacrificeObject {
                    object: Some(source),
                },
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
