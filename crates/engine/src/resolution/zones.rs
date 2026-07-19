//! Zones-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (ADR 0002 / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_zones_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            // Reality Shift's rider (CR 701.34): the *target's* controller manifests their top
            // library card — puts it onto the battlefield face down as a 2/2. Reads the target's
            // owner (control/ownership conflation, same as `GainLifeTargetController`), which stays
            // correct across the target's own exile (`owner_of` follows `Object::Moved`).
            Effect::Manifest => {
                let object = expect_object_target(target, "a manifest");
                let player = self.owner_of(object);
                let Some(&card) = self.players[player.0 as usize].library.first() else {
                    return Vec::new(); // an empty library manifests nothing (CR 701.34d).
                };
                vec![Event::Manifested {
                    permanent: self.next_object_id(),
                    from: card,
                    controller: player,
                }]
            }
            // Flicker (CR 400.7 — a new object, Momentary Blink/Mistmeadow Witch): exile the
            // target creature, then either return it immediately under its owner's control
            // (`return_at` absent) or schedule that return as a real CR 603.7 delayed triggered
            // ability at `return_at`'s step (`ReturnFlickeredCard`, carrying the specific card now
            // sitting in exile).
            Effect::FlickerTarget { return_at, .. } => {
                let object = expect_object_target(target, "flicker");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — nothing to flicker back.
                let permanent = self
                    .as_permanent(object)
                    .expect("flicker resolves against a battlefield permanent");
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let owner = permanent.owner;
                let mut next = self.next_object_id();
                let exiled = next;
                next += 1;
                let exile_event = self.exile_or_command(object, exiled);
                // CR 903.9b: a commander diverted to the command zone instead of exile was never
                // exiled — nothing returns.
                if matches!(exile_event, Event::MovedToCommandZone { .. }) {
                    return vec![exile_event];
                }
                match return_at {
                    None => vec![
                        exile_event,
                        Event::FlickeredToBattlefield {
                            permanent: next,
                            from: exiled,
                            controller: owner,
                        },
                    ],
                    Some(fire_at) => vec![
                        exile_event,
                        Event::DelayedTriggerScheduled {
                            controller,
                            source,
                            fire_at,
                            effect: Effect::ReturnFlickeredCard {
                                exiled: Some(exiled),
                            },
                        },
                    ],
                }
            }
            // The delayed payload `FlickerTarget` schedules when it carries a `return_at`
            // (Mistmeadow Witch): return the specific card `exiled` names to the battlefield under
            // its owner's control. Guard-return with no return if it's since left exile some
            // other way (CR 603.10a last-known information).
            Effect::ReturnFlickeredCard { exiled } => {
                let Some(exiled) = exiled else {
                    return Vec::new();
                };
                let exiled = self.current_id(exiled);
                if self.zone_of(exiled) != Zone::Exile {
                    return Vec::new();
                }
                vec![Event::FlickeredToBattlefield {
                    permanent: self.next_object_id(),
                    from: exiled,
                    controller: self.owner_of(exiled),
                }]
            }
            Effect::ReturnToHand { .. } => {
                let object = expect_object_target(target, "bounce");
                let permanent = self
                    .as_permanent(object)
                    .expect("bounce resolves against a battlefield permanent");
                // A token leaving the battlefield ceases to exist rather than changing zones
                // (CR 111.7) — it never reaches the hand.
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                }]
            }
            // The ability's own source — a no-target self-return. Its pool fires from a graveyard
            // death trigger (Angelic Destiny: by the time an `EnchantedCreatureDies` trigger
            // resolves, the Aura is already a graveyard card) or a battlefield activated ability
            // (Flickering Ward). Guard-return if the source has left both zones by resolution —
            // exiled by Nezumi Graverobber mid-trigger, say — per CR 603.6e / 400.7.
            Effect::ReturnThisToHand => {
                let Some(current) =
                    self.return_this_source(source, &[Zone::Graveyard, Zone::Battlefield])
                else {
                    return Vec::new();
                };
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: current,
                }]
            }
            // Nether Traitor: the ability's own source (a graveyard card by now) returns to the
            // battlefield under its owner's control (CR 603.6e). The self-return twin of
            // `ReanimateToBattlefield` — enters via the same ETB path. No-op if it has left the
            // graveyard by resolution (exiled mid-trigger, say). Teacher's Pest activates this
            // from the graveyard directly (CR 112.6) with `tapped = true`.
            Effect::ReturnThisFromGraveyardToBattlefield { tapped } => {
                let Some(current) = self.return_this_source(source, &[Zone::Graveyard]) else {
                    return Vec::new();
                };
                vec![Event::ReanimatedToBattlefield {
                    permanent: self.next_object_id(),
                    from: current,
                    controller,
                    finality: false,
                    tapped,
                }]
            }
            // Mass bounce: every matching permanent returns to its owner's hand (a token ceases to
            // exist). Ids are minted sequentially, matching the order `apply` will push them.
            Effect::ReturnAllToHand { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    if p.token {
                        events.push(Event::TokenCeasedToExist {
                            token: id,
                            controller: p.owner,
                            def: p.def,
                        });
                        continue;
                    }
                    events.push(Event::ReturnedToHand {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            // Raise Dead: send the chosen graveyard creature card to its owner's hand. Reuses
            // the bounce event (both move an object to its owner's hand); the graveyard card
            // isn't on the stack, so that event's stack cleanup is a harmless no-op.
            Effect::ReturnFromGraveyardToHand { .. } => {
                let object = expect_object_target(target, "graveyard recursion");
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: object,
                }]
            }
            // Reanimate: put the chosen graveyard creature card onto the battlefield under the
            // ability's controller's control (enters via the ETB path — see the event's apply arm).
            // Excava, the Risen Past's `becomes` rider follows with a `ReanimatedCreatureBecame` on
            // the just-entered permanent — the "It's a 1/1 Spirit creature with flying" indefinite
            // set (CR 611.2c). A plain reanimation (`becomes == None`) is just the one event.
            Effect::ReanimateToBattlefield {
                finality, becomes, ..
            } => {
                let object = expect_object_target(target, "reanimation");
                let entered = self.reanimate_event(object, controller, finality);
                let Some(becomes) = becomes else {
                    return vec![entered];
                };
                let Event::ReanimatedToBattlefield { permanent, .. } = entered else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                vec![
                    entered,
                    Event::ReanimatedCreatureBecame {
                        object: permanent,
                        add_types: becomes.add_types,
                        add_subtypes: becomes.add_subtypes,
                        base_power: becomes.base_power,
                        base_toughness: becomes.base_toughness,
                        keywords: becomes.keywords,
                    },
                ]
            }
            // Changing Loyalty / Gift of Immortality: reanimate the creature this Aura was
            // enchanting when it died, under either this ability's own controller ("your
            // control") or that card's owner ("its owner's control"). `dying` is the pre-death
            // battlefield id — `current_id` follows its `Moved` lineage into whatever object it
            // is now.
            Effect::ReanimateDyingEnchantedCreature { dying, under_owner } => {
                let Some(dying) = dying else {
                    return Vec::new();
                };
                let card = self.current_id(dying);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let new_controller = if under_owner {
                    self.owner_of(card)
                } else {
                    controller
                };
                vec![self.reanimate_event(card, new_controller, false)]
            }
            // Hofri Ghostforge: "exile it. If you do, create a token that's a copy of that
            // creature, except it's a Spirit in addition to its other types ...". `dead` is the
            // pre-death battlefield id; `current_id` follows its `Moved` lineage into the graveyard
            // card. Guard-return with no mint if it's no longer in a graveyard (exiled/moved in
            // response — the "if you do" fails). Reads the copiable `def` off the card before it
            // exiles, mints the token copy (CR 707.2) under `controller`, then adds `add_subtypes`
            // on the minted token (CR 613.4 subtype layer, indefinite).
            Effect::ExileDeadCreatureCreateCopyWithSubtype {
                dead,
                add_subtypes,
                leaves_returns_exiled,
            } => {
                let Some(dead) = dead else {
                    return Vec::new();
                };
                let card = self.current_id(dead);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let def = self.def_of(card);
                let exiled = self.next_object_id();
                let move_event = self.exile_or_command(card, exiled);
                let token = exiled + 1;
                let mut events = vec![
                    move_event,
                    Event::TokenCreated {
                        token,
                        controller,
                        def,
                        creator: source,
                    },
                ];
                if !add_subtypes.is_empty() {
                    events.push(Event::AddedSubtypes {
                        object: token,
                        subtypes: add_subtypes,
                    });
                }
                // "... and it has 'When this token leaves the battlefield, return the exiled
                // card to its owner's graveyard.'" — link the minted token to the exiled card;
                // `Game::queue_token_return_exiled_trigger` reads this once `token` leaves.
                if leaves_returns_exiled {
                    events.push(Event::TokenGrantedReturnExiledOnLeave { token, exiled });
                }
                events
            }
            // Hofri Ghostforge's minted Spirit token's granted rider: "return the exiled card to
            // its owner's graveyard." `exiled` was baked in at mint time
            // (`Game::queue_token_return_exiled_trigger`). Guard-return with no move if that card
            // is no longer in exile (already reclaimed some other way) — the printed rider only
            // returns a card that's still exiled. `Event::ReturnedExiledCardToGraveyard`, not
            // `MovedToGraveyard` — see that event's doc for why (this isn't a death).
            Effect::ReturnExiledCardToOwnersGraveyard { exiled } => {
                if self.zone_of(exiled) != Zone::Exile {
                    return Vec::new();
                }
                vec![Event::ReturnedExiledCardToGraveyard {
                    card: self.next_object_id(),
                    from: exiled,
                }]
            }
            // Gift of Immortality: the delayed CR 603.7 payoff scheduled by
            // `ScheduleReturnThisAuraAttachedToReanimated`, fired at the next end step. Guard-
            // return with no return if this Aura has since left the graveyard (moved/exiled some
            // other way — CR 603.10a last-known information) or `creature` no longer resolves to
            // a battlefield permanent (destroyed before the delayed trigger fired). Otherwise
            // move the Aura graveyard→battlefield through the same shared reanimate choke
            // `ReanimateDyingEnchantedCreature` above uses, then attach it in the same batch
            // (`Event::AttachedTo`) rather than pausing to choose a host.
            Effect::ReturnThisAuraAttachedTo { creature } => {
                let card = self.current_id(source);
                if self.zone_of(card) != Zone::Graveyard {
                    return Vec::new();
                }
                let Some(creature) = creature else {
                    return Vec::new();
                };
                let creature = self.current_id(creature);
                if self.zone_of(creature) != Zone::Battlefield {
                    return Vec::new();
                }
                let event = self.reanimate_event(card, self.owner_of(card), false);
                let Event::ReanimatedToBattlefield { permanent, .. } = event else {
                    unreachable!("reanimate_event always returns a ReanimatedToBattlefield event")
                };
                vec![
                    event,
                    Event::AttachedTo {
                        object: permanent,
                        host: Some(creature),
                    },
                ]
            }
            // Mistveil Plains: put the chosen graveyard card on the bottom of its owner's
            // library. Mystic Sanctuary sets `to_top` for its "on top of your library" instead.
            Effect::TuckFromGraveyard { to_top, .. } => {
                let object = expect_object_target(target, "graveyard tuck");
                vec![Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top,
                    second_from_top: false,
                }]
            }
            // Temporal Spring ("Put target permanent on top of its owner's library") and
            // Condemn's tuck half ("Put target attacking creature on the bottom of its owner's
            // library"): put a targeted battlefield permanent into its owner's library at a fixed
            // position. No shuffle — unlike its fused sibling `ShuffleTargetPermanentIntoLibraryThenReveal`
            // above, this needs no `&mut self` and stays in the pure event-building path.
            Effect::TuckPermanentIntoLibrary {
                to_top,
                second_from_top,
                ..
            } => {
                let object = expect_object_target(target, "a permanent to tuck");
                let owner = self.owner_of(object);
                // CR 111.7: a token can't exist in a library — it ceases to exist instead.
                if self.permanent(object).token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: owner,
                        def: self.def_of(object),
                    }];
                }
                vec![Event::TuckedToLibrary {
                    card: self.next_object_id(),
                    from: object,
                    to_top,
                    second_from_top,
                }]
            }
            // Gomazoa ("Put this creature and each creature it's blocking on top of their owners'
            // libraries, then those players shuffle."): `source` plus every attacker
            // `attackers_blocked_by` still reports for it (empty if it's blocking nothing — CR:
            // only Gomazoa itself is tucked then). Tuck every one first, then shuffle each
            // distinct owner exactly once — never mid-batch, so a second tuck to an owner already
            // shuffled doesn't land on top of a now-scrambled library while the first tuck was the
            // only one actually randomized in.
            Effect::TuckSelfAndBlockedCreatures => {
                let mut objects = vec![source];
                objects.extend(self.attackers_blocked_by(source));
                let mut events = Vec::new();
                let mut owners_tucked: Vec<PlayerId> = Vec::new();
                // Minted sequentially, matching the order `apply` will push them (same pattern
                // as `MassReturnFromGraveyard`'s mass reanimation) — a plain `next_object_id()`
                // per iteration would repeat the same id, since nothing's applied yet to advance it.
                let mut next_id = self.next_object_id();
                for object in objects {
                    // The source or a blocked attacker may have left the battlefield since this
                    // ability was activated (destroyed in response, CR 608.2b) — skip it.
                    let Some(permanent) = self.as_permanent(object) else {
                        continue;
                    };
                    let owner = permanent.owner;
                    // CR 111.7: a token can't exist in a library — it ceases to exist instead.
                    if permanent.token {
                        events.push(Event::TokenCeasedToExist {
                            token: object,
                            controller: owner,
                            def: permanent.def,
                        });
                        continue;
                    }
                    events.push(Event::TuckedToLibrary {
                        card: next_id,
                        from: object,
                        to_top: true,
                        second_from_top: false,
                    });
                    next_id += 1;
                    if !owners_tucked.contains(&owner) {
                        owners_tucked.push(owner);
                    }
                }
                for owner in owners_tucked {
                    events.push(Event::LibraryShuffled { player: owner });
                }
                events
            }
            // Oblation ("The owner of target nonland permanent shuffles it into their library,
            // then draws two cards."): the no-reveal half of `ShuffleTargetPermanentIntoLibraryThenReveal`
            // (Chaos Warp) — a real shuffle rather than a fixed position, but nothing in *this*
            // effect reads the post-shuffle order (Oblation's draw is a separate `TargetOwnerDraws`
            // step later in the same `Sequence`), so it stays in the pure event-building path too.
            Effect::ShuffleTargetPermanentIntoLibrary { .. } => {
                let object = expect_object_target(target, "a permanent to shuffle-tuck");
                self.shuffle_tuck_events(object)
            }
            // Replenish (Eiganjo Dynastorian's back face): every matching card in the
            // controller's own graveyard returns to the battlefield under their control, with no
            // finality counter. Ids are minted sequentially, matching the order `apply` will push
            // them (same pattern as `ReturnAllToHand`'s mass bounce).
            Effect::MassReturnFromGraveyard {
                filter,
                all_players,
            } => {
                // `all_players = false` (Replenish) scans only the ability controller's own
                // graveyard; `all_players = true` (All Hallow's Eve — "each player returns all
                // creature cards from their graveyard") scans EVERY player's graveyard in APNAP
                // order so id assignment is deterministic. Each player's cards return under that
                // player's own control (`owner`), which for the controller-only scan is just the
                // controller.
                let scan: Vec<PlayerId> = if all_players {
                    self.apnap_order()
                } else {
                    vec![controller]
                };
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for owner in scan {
                    for id in self.live_object_ids() {
                        if self.zone_of(id) != Zone::Graveyard || self.owner_of(id) != owner {
                            continue;
                        }
                        if !filter.matches(self.def_of(id)) {
                            continue;
                        }
                        events.push(Event::ReanimatedToBattlefield {
                            permanent: next,
                            from: id,
                            controller: owner,
                            finality: false,
                            tapped: false,
                        });
                        next += 1;
                    }
                }
                events
            }

            // Cauldron Dance's delayed payloads (never authored directly — see the variant's
            // doc). Return one already-resolved object to its owner's hand, no re-scan — the
            // return-flavored sibling of `SacrificeObject`/`ExileObject`. Guard-return if
            // it's already left the battlefield some other way before the delayed trigger
            // fired: nothing left to return. A token ceases to exist instead of actually
            // changing zones (CR 111.7), mirroring `ReturnToHand`'s own token branch — no pool
            // consumer can hit this (a token never sits in a graveyard to be reanimated, nor
            // does `PutCreatureFromHand` ever deploy one), but the guard costs nothing and keeps
            // the effect faithful if one ever does.
            Effect::ReturnObjectToHand { object } => {
                let id = object.expect("filled in when the delayed return was scheduled");
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let permanent = self.permanent(id);
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: id,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                vec![Event::ReturnedToHand {
                    card: self.next_object_id(),
                    from: id,
                }]
            }
            // Serra Paragon's rider (CR 118.9) — see the variant's doc. Guard-return no-op if
            // the card already left the graveyard before this placed trigger resolved (a
            // response returned/reanimated/re-exiled it in the meantime).
            // ponytail: skips the whole effect (exile + life) when the card is gone. The strict
            // CR 608.2 "do as much as possible" reading gains 2 life even then (the two clauses are
            // independent), but Serra Paragon is a documented rules-corner with no official ruling;
            // split the guard to still gain life if a card ever depends on that line.
            Effect::ExileGraveyardObjectGainLife { object, amount } => {
                let id = object.expect("filled in when the placed trigger was queued");
                if self.zone_of(id) != Zone::Graveyard {
                    return Vec::new();
                }
                vec![
                    self.exile_or_command(id, self.next_object_id()),
                    Event::LifeChanged {
                        player: controller,
                        amount: self.life_gain_after_replacements(controller, amount),
                        source: Some(source),
                    },
                ]
            }
            _ => unreachable!("zones family mint received a non-family effect"),
        }
    }

    /// Shuffle a target permanent into its owner's library (CR 111.7: a token ceases to exist
    /// instead) — the event pair shared by both Chaos Warp's fused reveal
    /// ([`Effect::ShuffleTargetPermanentIntoLibraryThenReveal`], `effects.rs`, which needs
    /// `&mut self` to read the post-shuffle top card) and Oblation's no-reveal sibling
    /// ([`Effect::ShuffleTargetPermanentIntoLibrary`] above, which doesn't).
    pub(crate) fn shuffle_tuck_events(&self, object: ObjectId) -> Vec<Event> {
        let owner = self.owner_of(object);
        if self.permanent(object).token {
            return vec![Event::TokenCeasedToExist {
                token: object,
                controller: owner,
                def: self.def_of(object),
            }];
        }
        vec![
            Event::TuckedToLibrary {
                card: self.next_object_id(),
                from: object,
                to_top: false,
                second_from_top: false,
            },
            Event::LibraryShuffled { player: owner },
        ]
    }
}
