//! Destroy, exile, and sacrifice Event mint and resolve choreography.
//!
//! Pure mint stays behind [`Game::execute_effect`]; [`Game::resolve_destroy_all`] /
//! [`Game::resolve_exile_all`] own mint → ResolutionFrame snapshot → apply so
//! [`Game::run`] stays a thin dispatcher (card-dsl-and-card-pool spec deepen).

use crate::*;

impl Game {
    pub(crate) fn mint_destroy(
        &self,
        effect: DestroyEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        let _source_name = self.source_name_of(source);
        match effect {
            DestroyEffect::Target {
                cant_be_regenerated,
                at,
                only_if_it_attacked,
                ..
            } => {
                let object = expect_object_target(target, "destroy");
                // "Destroy that creature at the beginning of the next end step" (Stone Giant): a
                // CR 603.7 delayed triggered ability carrying the id this ability already chose,
                // so it destroys that creature and re-targets nothing when it fires.
                if let Some(fire_at) = at {
                    return vec![Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at,
                        effect: Effect::Destroy(DestroyEffect::ThatCreature {
                            creature: Some(object),
                            only_if_it_attacked,
                            at: None,
                        }),
                    }];
                }
                if self.has_keyword(object, Keyword::Indestructible) {
                    return Vec::new();
                }
                if !cant_be_regenerated && self.regeneration_shield_available(object) {
                    return vec![Event::Regenerated { object }];
                }
                vec![self.graveyard_or_command(object, self.next_object_id())]
            }
            // Mass destruction: every matching permanent goes to the graveyard (a commander
            // diverts; a token ceases to exist). Ids are minted sequentially, matching the order
            // `apply` will push them (as the SBA death sweep does). (CR 704)
            DestroyEffect::All {
                filter,
                cant_be_regenerated,
            } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(ref p) = self.objects[id as usize] else {
                        continue;
                    };
                    if !self.permanent_matches(&filter, id, controller, Some(source)) {
                        continue;
                    }
                    if self.has_keyword(id, Keyword::Indestructible) {
                        continue;
                    }
                    // A regeneration shield replaces this permanent's own destroy with a
                    // regeneration (CR 701.15b), unless "can't be regenerated" turns it off (CR
                    // 701.15d) — same shield-honoring shape as `DestroyTarget`.
                    if !cant_be_regenerated && self.regeneration_shield_available(id) {
                        events.push(Event::Regenerated { object: id });
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
            DestroyEffect::ThatCreature {
                creature,
                only_if_it_attacked,
                at,
            } => {
                let Some(id) = creature else {
                    return Vec::new();
                };
                // Cockatrice's "destroy that creature at end of combat": re-schedule the same
                // payload as a CR 603.7 delayed ability, minus the `at` that got it here.
                if let Some(fire_at) = at {
                    return vec![Event::DelayedTriggerScheduled {
                        controller,
                        source,
                        fire_at,
                        effect: Effect::Destroy(DestroyEffect::ThatCreature {
                            creature,
                            only_if_it_attacked,
                            at: None,
                        }),
                    }];
                }
                // Berserk: the rider only collects on a creature that was declared an attacker
                // this turn (CR 508.1), which is why the check waits until the delayed ability
                // fires rather than happening when it was scheduled.
                if only_if_it_attacked
                    && !self.as_permanent(id).is_some_and(|p| p.attacked_this_turn)
                {
                    return Vec::new();
                }
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                if self.has_keyword(id, Keyword::Indestructible) {
                    return Vec::new();
                }
                if self.regeneration_shield_available(id) {
                    return vec![Event::Regenerated { object: id }];
                }
                vec![self.graveyard_or_command(id, self.next_object_id())]
            }
        }
    }

    pub(crate) fn mint_exile(
        &self,
        effect: ExileEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        match effect {
            ExileEffect::All { filter } => {
                let mut next = self.next_object_id();
                let mut events = Vec::new();
                for id in self.battlefield() {
                    let Object::Permanent(ref p) = self.objects[id as usize] else {
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
            ExileEffect::AllGraveyards => {
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
            ExileEffect::Graveyard => {
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
            ExileEffect::Object { object } => {
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
            ExileEffect::Target { .. } => {
                let object = expect_object_target(target, "exile");
                vec![self.exile_or_command(object, self.next_object_id())]
            }
            ExileEffect::TargetMintingIllusionOnLeave { .. } => {
                let object = expect_object_target(target, "exile-minting-illusion-on-leave");
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
            ExileEffect::UntilSourceLeaves { .. } => {
                let object = expect_object_target(target, "exile-until-source-leaves");
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
        }
    }

    pub(crate) fn mint_sacrifice(
        &self,
        effect: SacrificeEffect,
        controller: PlayerId,
        source: ObjectId,
        _target: Option<Target>,
        _x: u32,
    ) -> Vec<Event> {
        match effect {
            SacrificeEffect::EnchantedCreature { creature } => {
                let Some(id) = creature else {
                    return Vec::new();
                };
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_id_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: self.controller_of(id),
                        def,
                    },
                ]
            }
            SacrificeEffect::Object { object } => {
                let id = object.expect("filled in when the delayed sacrifice was scheduled");
                if self.zone_of(id) != Zone::Battlefield {
                    return Vec::new();
                }
                let def = self.def_id_of(id);
                vec![
                    self.sacrifice_event(id),
                    Event::Sacrificed {
                        object: id,
                        by: controller,
                        def,
                    },
                ]
            }
            SacrificeEffect::Source => {
                let def = self.def_id_of(source);
                vec![
                    self.sacrifice_event(source),
                    Event::Sacrificed {
                        object: source,
                        by: controller,
                        def,
                    },
                ]
            }
        }
    }

    pub(crate) fn resolve_destroy_all(
        &mut self,
        effect: DestroyEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        events: &mut Vec<Event>,
    ) {
        debug_assert!(matches!(effect, DestroyEffect::All { .. }));
        let evs = self.execute_effect(Effect::Destroy(effect), controller, source, target, x);
        self.resolution_frame.destroyed_this_way.clear();
        for e in &evs {
            match e.clone() {
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

    pub(crate) fn resolve_exile_all(
        &mut self,
        effect: ExileEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
        events: &mut Vec<Event>,
    ) {
        debug_assert!(matches!(effect, ExileEffect::All { .. }));
        let evs = self.execute_effect(Effect::Exile(effect), controller, source, target, x);
        self.resolution_frame.power_exiled_this_way.clear();
        for e in &evs {
            match e.clone() {
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
