//! Destroy-family Event mint and resolve choreography for related [`Effect`] variants.
//!
//! Pure mint stays behind [`Game::execute_effect`]; [`Game::resolve_destroy_all`] /
//! [`Game::resolve_exile_all`] own mint → ResolutionFrame snapshot → apply so
//! [`Game::run`] stays a thin dispatcher (ADR 0002 deepen).

use crate::*;

impl Game {
    pub(crate) fn mint_destroy_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            Effect::DestroyTarget {
                cant_be_regenerated,
                ..
            } => {
                let object = expect_object_target(target, "destroy");
                // Indestructible ignores "destroy" (CR 702.12b).
                if self.has_keyword(object, Keyword::Indestructible) {
                    return Vec::new();
                }
                // A regeneration shield replaces the next "destroy" this turn with a regeneration
                // (CR 701.15b), unless "can't be regenerated" turns it off (CR 701.15d).
                // ponytail: only this effect-driven destroy consults the shield; the CR 704.5g
                // lethal-marked-damage state-based destroy (also a "destroy" a shield should
                // replace) does not — unobserved, since no pool card grants a shield. Upgrade:
                // consult the shield in `apply`'s SBA death sweep for the lethal-damage case too. (CR 704, CR 120.3)
                if !cant_be_regenerated && self.permanent(object).regeneration_shields > 0 {
                    return vec![Event::Regenerated { object }];
                }
                // A destroyed commander may divert to the command zone.
                vec![self.graveyard_or_command(object, self.next_object_id())]
            }
            // Mass destruction: every matching permanent goes to the graveyard (a commander
            // diverts; a token ceases to exist). Ids are minted sequentially, matching the order
            // `apply` will push them (as the SBA death sweep does). (CR 704)
            Effect::DestroyAll { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    // Indestructible survives a board wipe's "destroy" (CR 702.12b).
                    if self.has_keyword(id, Keyword::Indestructible) {
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
                    events.push(self.graveyard_or_command(id, next));
                    next += 1;
                }
                events
            }
            // Mass exile: every matching permanent goes to exile (a commander diverts; a token
            // ceases to exist). Unlike `DestroyAll`, there's no indestructible guard — exile (CR 702.12, CR 111.7, CR 406.5)
            // isn't "destroy" (CR 701.18a vs CR 702.12b) — and no graveyard branch, just the
            // exile-or-command-zone choke point `ExileTarget` already uses (CR 903.9b).
            Effect::ExileAll { filter } => {
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
                    events.push(self.exile_or_command(id, next));
                    next += 1;
                }
                events
            }
            Effect::ExileTarget { .. } => {
                let object = expect_object_target(target, "exile");
                vec![self.exile_or_command(object, self.next_object_id())]
            }
            // The O-Ring pattern (CR 603.6e): exile the target, linking it to this ability's own
            // `source` (the Aura) so `Game::check_linked_exile_returns` can send it back once the
            // Aura leaves.
            Effect::ExileUntilSourceLeaves { .. } => {
                let object = expect_object_target(target, "exile-until-source-leaves");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — it's never actually placed in exile, so there's nothing to
                // link back to this source.
                let permanent = self
                    .as_permanent(object)
                    .expect("exile-until-source-leaves resolves against a battlefield permanent");
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledUntilSourceLeaves {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Skyclave Apparition's linked exile (a sibling of `ExileUntilSourceLeaves`, not a
            // fork of its list): exile the target, linking it to this ability's own `source` so
            // `Game::check_leaves_battlefield_illusions` can mint its owner an Illusion once
            // `source` leaves. Unlike the O-Ring pattern, the card is never returned.
            Effect::ExileTargetMintingIllusionOnLeave { .. } => {
                let object = expect_object_target(target, "exile-minting-illusion-on-leave");
                // CR 111.7: a token that leaves the battlefield ceases to exist rather than
                // changing zones — nothing to link back to this source.
                let permanent = self.as_permanent(object).expect(
                    "exile-minting-illusion-on-leave resolves against a battlefield permanent",
                );
                if permanent.token {
                    return vec![Event::TokenCeasedToExist {
                        token: object,
                        controller: permanent.owner,
                        def: permanent.def,
                    }];
                }
                let exiled = self.next_object_id();
                vec![
                    self.exile_or_command(object, exiled),
                    Event::ExiledUntilSourceLeavesMintingIllusion {
                        source,
                        object: exiled,
                    },
                ]
            }
            // Bojuka Bog / Remorseful Cleric: exile every card in the target player's graveyard.
            // Ids are minted sequentially, matching the order `apply` will push them (same
            // pattern as ReturnAllToHand's mass bounce).
            Effect::ExileGraveyard => {
                let Some(Target::Player(player)) = target else {
                    panic!("exile-graveyard resolves with a chosen player target");
                };
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.live_object_ids() {
                    if self.zone_of(id) != Zone::Graveyard || self.owner_of(id) != player {
                        continue;
                    }
                    events.push(Event::MovedToExile {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            // Final Act's "Exile all graveyards" mode: every player's graveyard, no target — the
            // mass twin of `ExileGraveyard` above, minus the `owner_of(id) != player` filter.
            Effect::ExileAllGraveyards => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.live_object_ids() {
                    if self.zone_of(id) != Zone::Graveyard {
                        continue;
                    }
                    events.push(Event::MovedToExile {
                        card: next,
                        from: id,
                    });
                    next += 1;
                }
                events
            }
            // Sacrifice one already-resolved object (never authored directly — see the variant's
            // doc). Guard-return if it's already left the battlefield (destroyed/exiled some
            // other way before the delayed trigger fired): nothing left to sacrifice.
            Effect::SacrificeObject { object } => {
                let id = object.expect("filled in when the delayed sacrifice was scheduled");
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: controller,
                        def,
                    },
                ]
            }
            // Sacrifice the ability's own source (CR 701.16) — Court Hussar's "sacrifice it",
            // authorable directly (unlike `SacrificeObject` above). No zone guard needed: this
            // only ever runs synchronously off the source's own ETB, which can't have already
            // left the battlefield.
            Effect::SacrificeSource => {
                let def = self.def_of(source);
                vec![
                    self.sacrifice_event(source),
                    Event::Sacrificed {
                        object: source,
                        by: controller,
                        def,
                    },
                ]
            }
            // A `ThisPermanentLeavesBattlefield` look-back payoff (Animate Dead): "that creature's
            // controller sacrifices it" (CR 603.10a last-known information). Guard-return if the
            // triggering context never filled a host, or if that creature no longer sits on the
            // battlefield (it died first and the Aura fell off its own CR 704.5m SBA, or it was
            // bounced/exiled in response — the "that creature" reference fizzles). `by` reads the
            // creature's own current controller, not this ability's — CR "that creature's
            // controller", not "you".
            Effect::SacrificeEnchantedCreature { creature } => {
                let Some(id) = creature else {
                    return Vec::new();
                };
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: self.controller_of(id),
                        def,
                    },
                ]
            }
            // Exile one already-resolved object (never authored directly — see the variant's
            // doc). Guard-return if it's already left the battlefield (destroyed/exiled/bounced
            // some other way before the delayed trigger fired): nothing left to exile. A token
            // ceases to exist instead of actually changing zones (CR 111.7) — the same
            // exile-or-command-zone choke point `ExileAll`/`ExileTarget` already use (CR 903.9b).
            Effect::ExileObject { object } => {
                let id = object.expect("filled in when the delayed exile was scheduled");
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
                vec![self.exile_or_command(id, self.next_object_id())]
            }

            // A `DealsCombatDamageToCreature` payoff (Stinkweed Imp): "destroy that creature" (CR
            // 603.10a last-known information). Guard-return if the triggering context never
            // filled a target, or if that creature no longer sits on the battlefield (it died
            // first, or was bounced/exiled in response — the "that creature" reference fizzles).
            // An ordinary destroy otherwise, same shield-honoring shape as `DestroyTarget`:
            // indestructible ignores it (CR 702.12b), and a regeneration shield replaces it (CR
            // 701.15b).
            Effect::DestroyTriggeringDamagedCreature { creature } => {
                let Some(id) = creature else {
                    return Vec::new();
                };
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                if self.has_keyword(id, Keyword::Indestructible) {
                    return Vec::new();
                }
                if self.permanent(id).regeneration_shields > 0 {
                    return vec![Event::Regenerated { object: id }];
                }
                vec![self.graveyard_or_command(id, self.next_object_id())]
            }
            _ => unreachable!("destroy family mint received a non-family effect"),
        }
    }

    /// Resolve [`Effect::DestroyAll`]: mint → snapshot into [`ResolutionFrame`] → apply.
    /// Owns the "destroyed this way" choreography so [`Game::run`] stays a thin dispatcher.
    pub(crate) fn resolve_destroy_all(
        &mut self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        events: &mut Vec<Event>,
    ) {
        debug_assert!(matches!(effect, Effect::DestroyAll { .. }));
        let evs = self.execute_effect(effect, controller, source, target, x);
        self.resolution_frame.destroyed_this_way.clear();
        for e in &evs {
            match *e {
                Event::TokenCeasedToExist {
                    controller: died_controller,
                    def,
                    ..
                } => {
                    self.resolution_frame
                        .destroyed_this_way
                        .push(state::DestroyedThisWay {
                            def,
                            controller: died_controller,
                            token: true,
                        });
                }
                Event::MovedToGraveyard { from, .. } | Event::MovedToCommandZone { from, .. } => {
                    if let Some(p) = self.as_permanent(from) {
                        self.resolution_frame
                            .destroyed_this_way
                            .push(state::DestroyedThisWay {
                                def: p.def,
                                controller: self.controller_of(from),
                                token: false,
                            });
                    }
                }
                _ => {}
            }
        }
        self.apply_all(&evs);
        events.extend(evs);
    }

    /// Resolve [`Effect::ExileAll`]: mint → snapshot power-exiled-this-way → apply.
    pub(crate) fn resolve_exile_all(
        &mut self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        events: &mut Vec<Event>,
    ) {
        debug_assert!(matches!(effect, Effect::ExileAll { .. }));
        let evs = self.execute_effect(effect, controller, source, target, x);
        self.resolution_frame.power_exiled_this_way.clear();
        for e in &evs {
            match *e {
                Event::TokenCeasedToExist {
                    token,
                    controller: died_controller,
                    ..
                } => {
                    self.resolution_frame
                        .power_exiled_this_way
                        .push(state::PowerExiledThisWay {
                            controller: died_controller,
                            power: self.power(token),
                        });
                }
                Event::MovedToExile { from, .. } | Event::MovedToCommandZone { from, .. } => {
                    self.resolution_frame
                        .power_exiled_this_way
                        .push(state::PowerExiledThisWay {
                            controller: self.controller_of(from),
                            power: self.power(from),
                        });
                }
                _ => {}
            }
        }
        self.apply_all(&evs);
        events.extend(evs);
    }
}
