//! Misc resolution choreography that needs `&mut self` — the pause-free "one-off" arms
//! peeled out of [`Game::run`] (card-dsl-and-card-pool spec deepen). Pure event mint for these effect variants
//! lives in [`crate::resolution::misc`]; this module is the choreography twin, calling into
//! game state directly (RNG, snapshotted resolution-frame reads, arm-armed runtime flags,
//! per-player fan-outs) rather than through the pure `mint_*` families.

use crate::*;

impl Game {
    /// Resolve one of the misc, no-pause choreography arms behind [`Game::run`]. Each match
    /// arm mirrors the (formerly inline) [`Game::run`] arm 1:1 — no behavior change, just
    /// the body relocated so [`Game::run`] can stay a thin dispatcher.
    pub(crate) fn run_misc_choreo(
        &mut self,
        effect: Effect,
        ctx: ResolveCtx,
        events: &mut Vec<Event>,
    ) {
        let ResolveCtx {
            controller,
            source,
            target,
            x,
            ..
        } = ctx;
        match effect {
            // Glasses of Urza's "{T}: Look at target player's hand." Nothing moves and nothing is
            // chosen — the resolution's whole product is that one seat now knows those cards.
            Effect::Dig(DigEffect::LookAtTargetPlayersHand) => {
                let Some(Target::Player(looked_at)) = target else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::LookedAtHand {
                        player: controller,
                        target: looked_at,
                    },
                )
            }
            // Creative Technique's "Shuffle your library, then reveal…" lead-in step.
            Effect::Dig(DigEffect::ShuffleLibrary) => {
                self.push_apply(events, Event::LibraryShuffled { player: controller })
            }
            // "Each player creates a 0/0 green and blue Fractal creature token and puts a number
            // of +1/+1 counters on it equal to the total power of creatures they controlled that
            // were exiled this way." (Oversimplify): mint one `token` per living player in APNAP
            // order, applying each mint before computing its counters — `counters_after_replacements`
            // reads the token's controller off game state, mirroring `CreateToken`'s `enters_with`
            // below. No player choice, so this resolves in one pass, never pausing.
            Effect::Choice(ChoiceEffect::EachPlayerCreatesFractalFromExiledPower { token }) => {
                for player in self.apnap_order() {
                    let minted = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::TokenCreated {
                            token: minted,
                            controller: player,
                            def: intern_card_def(token.clone()),
                            creator: source,
                        },
                    );
                    let power: i32 = self
                        .resolution_frame
                        .power_exiled_this_way
                        .iter()
                        .filter(|snap| snap.controller == player)
                        .map(|snap| snap.power)
                        .sum();
                    let n = self.counters_after_replacements(player, minted, power);
                    if n > 0 {
                        self.push_apply(
                            events,
                            Event::CountersPlaced {
                                object: minted,
                                count: n,
                                source_name: self.def_of(source).name,
                            },
                        );
                    }
                }
            }
            // "Each player discards their hand, then draws seven cards." (Wheel of Fortune):
            // loop APNAP order, each living player discarding their whole hand (`discard_ids` —
            // no choice, so no `PendingChoice`, unlike a partial-hand `Effect::Choice(ChoiceEffect::Discard)`) then
            // drawing `count`.
            Effect::Choice(ChoiceEffect::EachPlayerDiscardsHandThenDraws { count }) => {
                let n = self.resolve_count(count, controller, source, target, x);
                for player in self.apnap_order() {
                    let hand = self.hand_of(player);
                    self.discard_ids(&hand, player, events);
                    for event in self.draw_events(player, n) {
                        self.push_apply(events, event);
                    }
                }
            }
            // "Each player shuffles their hand and graveyard into their library, then draws seven
            // cards." (Timetwister), and Winds of Change's hands-only "then draws that many
            // cards": the same APNAP loop as the wheel above, but each old card is tucked back
            // into the library (`TuckedToLibrary`, the zone move the graveyard shuffle-backs
            // already use) and the library shuffled once per player before they draw. The spell
            // itself is still on the stack here, so it isn't swept up (CR 608.2m puts it into the
            // graveyard only once the effect has finished).
            Effect::Choice(ChoiceEffect::EachPlayerShufflesHandThenDraws {
                include_graveyard,
                count,
            }) => {
                for player in self.apnap_order() {
                    let hand = self.hand_of(player);
                    // "That many" is this player's hand size, read before they shuffle — so it is
                    // per player, unlike Timetwister's one number for the table.
                    let n = match count {
                        Some(count) => self.resolve_count(count, controller, source, target, x),
                        None => hand.len() as u32,
                    };
                    let recycled: Vec<ObjectId> = hand
                        .into_iter()
                        .chain(if include_graveyard {
                            self.graveyard_cards(player)
                        } else {
                            Vec::new()
                        })
                        .collect();
                    for from in recycled {
                        self.push_apply(
                            events,
                            Event::TuckedToLibrary {
                                card: self.next_object_id(),
                                from,
                                to_top: false,
                                second_from_top: false,
                            },
                        );
                    }
                    self.push_apply(events, Event::LibraryShuffled { player });
                    for event in self.draw_events(player, n) {
                        self.push_apply(events, event);
                    }
                }
            }
            // Malfegor's "discard your hand": the controller discards their whole hand (no choice,
            // so no `PendingChoice`), setting `cards_discarded_this_way` to its size so a following
            // Sequence step (Malfegor's each-opponent sacrifice) reads "for each card discarded
            // this way".
            Effect::Choice(ChoiceEffect::DiscardYourHand) => {
                let hand = self.hand_of(controller);
                self.resolution_frame.cards_discarded_this_way = hand.len() as u32;
                self.discard_ids(&hand, controller, events);
            }
            // Advanced Reconstruction's base ability: "exile a card from your graveyard at
            // random. You may play the exiled card this turn." The card is picked by the
            // injected RNG here (needs `&mut self`, unlike `ExileFromGraveyardMayPlay`'s
            // trigger-supplied card), then reuses that same event/permission plumbing.
            Effect::Dig(DigEffect::ExileRandomFromGraveyardMayPlay) => {
                let graveyard = self.graveyard_cards(controller);
                // CR 701.19a: if there's nothing to exile, this is a no-op.
                if graveyard.is_empty() {
                    return;
                }
                let idx = self.with_op_rng(controller, |rng| rng.gen_index(graveyard.len()));
                let from = graveyard[idx];
                self.push_apply(
                    events,
                    Event::ExiledFromGraveyardMayPlay {
                        player: controller,
                        card: self.next_object_id(),
                        from,
                    },
                );
            }
            // Ruhan of the Fomori's base ability: "Choose an opponent at random. ~ attacks that
            // player this combat if able." The opponent is picked by the injected RNG here (needs
            // `&mut self`), then reuses the same `must_attack` requirement plumbing a token's
            // `must_attack_defender` uses.
            Effect::Misc(MiscEffect::MustAttackRandomOpponent) => {
                let opponents: Vec<PlayerId> =
                    self.living_players().filter(|&p| p != controller).collect();
                // CR 800.4a: no living opponents (a solitaire test rig) — nothing to choose.
                if opponents.is_empty() {
                    return;
                }
                let idx = self.with_op_rng(controller, |rng| rng.gen_index(opponents.len()));
                self.push_apply(
                    events,
                    Event::MustAttackDeclared {
                        object: source,
                        defender: opponents[idx],
                    },
                );
            }
            // Basandra, Battle Seraph's {R} ability: "Target creature attacks this turn if able."
            // Reuses the same `must_attack` requirement plumbing as `MustAttackRandomOpponent`
            // above, but names no specific required opponent — recording the target's own
            // controller as the `defender` is the sentinel `declare_attackers` already reads as
            // "must attack, any legal defender" (its `required_legal` gate short-circuits on
            // `required == player`, the same escape hatch `must_attack_each_combat` uses).
            // Siren's Call: "Creatures the active player controls attack this turn if able." The
            // same per-creature requirement the targeted clause below mints, over a board scan —
            // and with the same own-controller sentinel for "any legal defender", since the card
            // names no defender either. The set is fixed here, as this resolves.
            Effect::Misc(MiscEffect::MustAttackAll { filter }) => {
                for id in self.battlefield() {
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    self.push_apply(
                        events,
                        Event::MustAttackDeclared {
                            object: id,
                            defender: self.controller_of(id),
                        },
                    );
                }
            }
            // Blaze of Glory: "Target creature defending player controls can block any number of
            // creatures this turn. It blocks each attacking creature this turn if able."
            // Turn-scoped combat bookkeeping, written straight rather than minted as an event —
            // the same as `prevent_all_combat_damage_this_turn` below; nothing replays a ceiling.
            Effect::Misc(MiscEffect::BlocksEachAttackerIfAble { .. }) => {
                let Some(Target::Object(creature)) = target else {
                    return;
                };
                self.combat_extras.may_block_any_number.push(creature);
                self.combat_extras.must_block_all.push(creature);
            }
            Effect::Misc(MiscEffect::MustAttackTarget { .. }) => {
                let Some(Target::Object(creature)) = target else {
                    return;
                };
                self.push_apply(
                    events,
                    Event::MustAttackDeclared {
                        object: creature,
                        defender: self.controller_of(creature),
                    },
                );
            }
            // Tariel, Reckoner of Souls: "Choose a creature card at random from target opponent's
            // graveyard. Put that card onto the battlefield under your control." The opponent is
            // a real target (`target`, already resolved ahead of this resolution — hexproof/
            // protection already checked); the creature card among their graveyard is picked by
            // the injected RNG here (needs `&mut self`), then reuses the same reanimation event
            // `ReanimateToBattlefield` mints through.
            Effect::Zone(ZoneEffect::ReanimateRandomFromTargetOpponentGraveyard { .. }) => {
                let Some(Target::Player(opponent)) = target else {
                    unreachable!("resolves against a targeted opponent")
                };
                let creatures: Vec<ObjectId> = self
                    .graveyard_cards(opponent)
                    .into_iter()
                    .filter(|&id| matches!(self.def_of(id).kind, CardKind::Creature { .. }))
                    .collect();
                // No creature card in that graveyard — nothing to choose, a no-op.
                if creatures.is_empty() {
                    return;
                }
                let idx = self.with_op_rng(controller, |rng| rng.gen_index(creatures.len()));
                let card = creatures[idx];
                let event = self.reanimate_event(card, controller, false);
                self.push_apply(events, event);
            }
            // "Discards a card at random" (Hypnotic Specter) / "discards X cards at random" (Mind
            // Twist). Unlike every other discard this raises no pause — nobody chooses, so the
            // cards come off the injected per-op RNG (needs `&mut self`) here rather than from an
            // answered `ChoiceRequest::Discard`. Discarding fewer than asked when the hand runs
            // short is ordinary CR 701.8c, not a rejection. The discard itself still routes through
            // the shared `discard_ids`, so discard watchers, madness and Containment Construct see
            // a random pitch exactly as they see a chosen one.
            Effect::Choice(ChoiceEffect::Discard { count, who, .. }) => {
                let Some(player) = self.sole_player_in(who, controller, target) else {
                    return;
                };
                let count = self
                    .resolve_amount(count, controller, source, target, x)
                    .max(0) as usize;
                let mut hand = self.hand_of(player);
                let mut picked = Vec::new();
                for _ in 0..count.min(hand.len()) {
                    let idx = self.with_op_rng(player, |rng| rng.gen_index(hand.len()));
                    picked.push(hand.swap_remove(idx));
                }
                self.discard_ids(&picked, player, events);
            }
            // Inkshield (CR 615): arm a this-turn combat-damage prevention shield protecting the
            // ability's controller ("dealt to *you*"), carrying the Inkling profile minted per
            // point prevented. The tokens are created at the prevention itself (in `damage_player`),
            // not here — at resolution no combat damage has been prevented yet. Runtime
            // orchestration state (like the delayed combat-damage watches), not an event.
            Effect::Misc(MiscEffect::PreventCombatDamageToYouCreatingTokens { token }) => self
                .combat_extras
                .combat_damage_prevention_shields
                .push((controller, token)),
            // Moment's Peace (#150): arm the this-turn table-wide combat-damage shield — every
            // player's combat damage, not just this ability's controller's, and no token mint.
            // Runtime orchestration state (like Inkshield's shield above), not an event.
            Effect::Misc(MiscEffect::PreventAllCombatDamageThisTurn) => {
                self.combat_extras.prevent_all_combat_damage_this_turn = true;
            }
            // "Prevent the next N damage that would be dealt to any target this turn" (CR 615 —
            // Healing Salve, Samite Healer, Conservator): arm a consumable shield worth `amount`
            // points on the chosen target, or on this ability's controller when the card takes no
            // target ("dealt to you"). Runtime orchestration state like the shields above; only
            // the *spending* is an event (`Event::DamagePrevented`), because it happens inside the
            // pure damage mint.
            Effect::Misc(MiscEffect::PreventNextDamage {
                amount,
                from_color,
                gain_life,
                redirect_to_controller,
                shield_source,
                all_but,
                target_is_source,
                any_recipient,
                combat_only,
                from_filter,
                from_relation,
                all_damage,
                ..
            }) => {
                // No point total is "prevent *that* damage" (the Circles, Reverse Damage) — the
                // whole of the next hit, so there is nothing to compute and nothing that can
                // round down to a no-op shield.
                let points = match amount {
                    None => None,
                    Some(amount) => {
                        let points = self.resolve_amount(amount, controller, source, target, x);
                        if points <= 0 {
                            return;
                        }
                        Some(points)
                    }
                };
                // Forcefield names a *source* rather than a protectee: the chosen creature is
                // what the shield stops, and what it stands in front of is this ability's own
                // controller.
                let from_source = match (target_is_source, target) {
                    (true, Some(Target::Object(creature))) => Some(creature),
                    (true, _) => return,
                    (false, _) => None,
                };
                self.damage_prevention_shields
                    .push(crate::state::PreventionShield {
                        // Personal Incarnation's shield sits on the permanent that armed it;
                        // every other one covers whatever the ability targeted, or its controller.
                        target: if shield_source {
                            Target::Object(source)
                        } else if target_is_source {
                            Target::Player(controller)
                        } else {
                            target.unwrap_or(Target::Player(controller))
                        },
                        amount: points,
                        keep: all_but.map(|keep| {
                            self.resolve_amount(keep, controller, source, target, x)
                                .max(0)
                        }),
                        from_color,
                        from_source,
                        any_recipient,
                        combat_only,
                        from_filter,
                        from_relation,
                        persistent: all_damage,
                        gain_life,
                        redirect_to: redirect_to_controller.then_some(Target::Player(controller)),
                    });
            }
            // Guardian Angel's "until end of turn, you may pay {1} any time you could cast an
            // instant. If you do, prevent the next 1 damage that would be dealt to that permanent
            // or player this turn": record the standing offer rather than a shield — nothing is
            // prevented until it is paid for. "That permanent or player" is the target the
            // enclosing `Effect::Sequence` already chose for the first sentence. Runtime
            // orchestration state like the shields above; the *payment* is what reaches the action
            // list, and each one mints an ordinary shield.
            Effect::Misc(MiscEffect::OfferPreventionTopUp { cost, amount }) => {
                self.standing_preventions
                    .push(crate::state::StandingPrevention {
                        player: controller,
                        target: target.unwrap_or(Target::Player(controller)),
                        cost,
                        amount,
                    });
            }
            // Master Warcraft: hand the attack / block declaration to this spell's controller for
            // the rest of the turn. Runtime orchestration state like the shields above — the
            // declarations themselves stay ordinary, only the seat that makes them moves.
            Effect::Misc(MiscEffect::YouChooseWhichCreaturesAttack) => {
                self.combat_extras.attack_declarer = Some(controller);
            }
            Effect::Misc(MiscEffect::YouChooseWhichCreaturesBlock) => {
                self.combat_extras.block_declarer = Some(controller);
            }
            // "Exile [this card] with N time counters on it" (Rousing Refrain): mark the resolving
            // spell so `finish_instant_sorcery_resolution` sends it to exile with time counters
            // instead of the graveyard (the resolving spell, `source`, is the card exiled).
            Effect::Zone(ZoneEffect::ExileSelfWithTimeCounters { counters, .. }) => {
                self.resolution_finish = Some(FinishPolicy::ExileWithTimeCounters(counters));
            }
            // "Then put [this card] on the bottom of its owner's library" (Spell Crumple): mark
            // the resolving spell so `finish_instant_sorcery_resolution` sends it to the bottom
            // of its owner's library instead of the graveyard (`source`, the resolving spell
            // itself, is the card tucked).
            Effect::Zone(ZoneEffect::TuckSelfToLibraryBottom) => {
                self.resolution_finish = Some(FinishPolicy::TuckLibraryBottom);
            }
            // "Exile [this card]" (Vengeful Rebirth): mark the resolving spell so
            // `finish_instant_sorcery_resolution` sends it to exile instead of the graveyard
            // (`source`, the resolving spell itself, is the card exiled).
            Effect::Zone(ZoneEffect::ExileSelfOnResolve) => {
                self.resolution_finish = Some(FinishPolicy::Exile);
            }
            // Opal Palace's spend-to-cast rider: the commander spell (baked in as
            // `triggering_spell` when the `SpendManaToCast` trigger fired) is still on the stack, so
            // record the additional-counter count keyed by its id for `resolve_spell` to place as it
            // enters. Guard-return if that spell already left the stack (countered in response, CR
            // 603.4) — nothing to enter, so nothing to record.
            Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters {
                triggering_spell,
                count,
            }) => {
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
            Effect::Dig(DigEffect::ExileTargetGraveyardSpellCastFree { .. }) => {
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
            Effect::Dig(DigEffect::ExileTargetGraveyardCardRecordManaValue { .. }) => {
                let object =
                    expect_object_target(target, "exile target graveyard card, record mana value");
                let mana_value = self.def_of(object).mana_value();
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(object, exiled);
                self.push_apply(events, move_event);
                self.resolution_frame.surge_exiled_card = Some((exiled, mana_value));
            }
            _ => unreachable!("misc resolution choreo received a non-family effect"),
        }
    }
}
