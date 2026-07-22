//! Pump-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_pump_family(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // Pump / destroy / counters target a creature, so the chosen target is an object.
            Effect::PumpUntilEndOfTurn {
                power,
                toughness,
                keywords,
                ..
            } => {
                let object = expect_object_target(target, "a pump");
                vec![Event::TempBoost {
                    object,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                    keywords,
                    source_name,
                }]
            }
            // Self-pump: the ability's own source, no target (prowess). The source is already
            // known at resolution, so there's nothing to choose.
            Effect::PumpSelfUntilEndOfTurn {
                power,
                toughness,
                keywords,
            } => {
                // CR 608.2c: nothing to boost if the source has already left the battlefield —
                // e.g. it paid its own "Sacrifice a creature" cost (Fallen Ideal's granted
                // ability, where the host may sacrifice itself).
                if self.as_permanent(source).is_none() {
                    return Vec::new();
                }
                vec![Event::TempBoost {
                    object: source,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                    keywords,
                    source_name,
                }]
            }
            // Mass pump: every creature the controller controls, no target (Selfless Spirit,
            // Moonshaker Cavalry).
            Effect::PumpCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                keywords,
                filter,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        self.is_creature_on_battlefield(id)
                            && self.controller_of(id) == controller
                            && self.permanent_matches(&filter, id, controller, Some(source))
                    })
                    .map(|object| Event::TempBoost {
                        object,
                        power,
                        toughness,
                        keywords,
                        source_name,
                    })
                    .collect()
            }
            // Keyword-only mass grant to every permanent (creature or not) the controller
            // controls matching `filter`, no P/T (Silkguard's Auras/Equipment clause). The
            // noncreature-permanent twin of the mass pump above — same "you control" scan, no
            // creature gate.
            Effect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { keywords, filter } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(source))
                })
                .map(|object| Event::TempBoost {
                    object,
                    power: 0,
                    toughness: 0,
                    keywords,
                    source_name,
                })
                .collect(),
            // Mass base-P/T SET: every creature the controller controls has its base P/T set to
            // `power`/`toughness` until end of turn (Biomass Mutation). Same "you control" scan as
            // the mass pump, but a 7b base SET rather than a 7c delta.
            Effect::SetBasePtCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                other,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| {
                        (!other || id != source)
                            && self.is_creature_on_battlefield(id)
                            && self.controller_of(id) == controller
                    })
                    .map(|object| Event::BasePtSetUntilEndOfTurn {
                        object,
                        power,
                        toughness,
                    })
                    .collect()
            }
            // Single-target base-P/T SET: the chosen creature's base P/T is set until end of turn
            // (Quandrix Charm mode 2). The targeted twin of the mass set above.
            Effect::SetBasePtTargetUntilEndOfTurn {
                power, toughness, ..
            } => {
                let object = expect_object_target(target, "a base-P/T set");
                vec![Event::BasePtSetUntilEndOfTurn {
                    object,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                }]
            }
            // Indefinite self base-P/T SET (Trench Gorger's "this creature has base power and
            // toughness each equal to the number of cards exiled this way", CR 613.3(7b)): unlike
            // `SetBasePtTargetUntilEndOfTurn` above, this is never cleared at cleanup. Nothing to
            // do if the source has already left (CR 608.2c).
            Effect::SetOwnBasePtFromAmount { amount } => {
                if self.as_permanent(source).is_none() {
                    return Vec::new();
                }
                let value = self.resolve_amount(amount, controller, source, target, x);
                vec![Event::BasePtSetIndefinite {
                    object: source,
                    power: value,
                    toughness: value,
                }]
            }
            // Manland self-animation (Restless Spire): the source land becomes a creature until end
            // of turn — an added type/subtype (613.4), a base-P/T SET (613.3(7b)), and granted
            // keywords, all on the source. Nothing to do if the source has left (CR 608.2c).
            Effect::AnimateSelfUntilEndOfTurn {
                add_types,
                add_subtypes,
                base_power,
                base_toughness,
                keywords,
                add_colors,
            } => {
                if self.as_permanent(source).is_none() {
                    return Vec::new();
                }
                let mut events = vec![
                    Event::TypesAddedUntilEndOfTurn {
                        object: source,
                        types: add_types,
                        subtypes: add_subtypes,
                        colors: add_colors,
                    },
                    Event::BasePtSetUntilEndOfTurn {
                        object: source,
                        power: base_power,
                        toughness: base_toughness,
                    },
                ];
                if !keywords.is_empty() {
                    events.push(Event::TempBoost {
                        object: source,
                        power: 0,
                        toughness: 0,
                        keywords,
                        source_name,
                    });
                }
                events
            }
            // "each other creature that's attacking one of your opponents gets +1/+1 until end
            // of turn." Fired by the enchanted creature's own attack trigger; `source` is the
            // Aura, so its host is the "other"-excluded creature.
            Effect::PumpOtherAttackersAttackingYourOpponents { power, toughness } => {
                let Some(host) = self.attached_to(source) else {
                    return Vec::new();
                };
                self.combat
                    .attackers
                    .iter()
                    .copied()
                    .filter(|&a| a != host)
                    .filter(|&a| self.is_creature_on_battlefield(a))
                    .filter(|&a| self.defending_player_of(a).is_some_and(|d| d != controller))
                    .map(|object| Event::TempBoost {
                        object,
                        power,
                        toughness,
                        keywords: &[],
                        source_name,
                    })
                    .collect()
            }
            // Contract (Scriv, the Obligator): "Whenever enchanted creature attacks, it gets
            // +2/+0 until end of turn if it's attacking one of your opponents. Otherwise, its
            // controller loses 2 life." `source` is the Aura, `controller` its own controller;
            // the host is `source`'s attachment, "one of your opponents" is the host's declared
            // defender being someone other than the Aura's controller. An unattached Aura (mid-SBA) (CR 704, CR 303.4, CR 108.3)
            // has no host (guard-return).
            Effect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
                power,
                toughness,
                life,
            } => {
                let Some(host) = self.attached_to(source) else {
                    return Vec::new();
                };
                let attacking_your_opponent = self
                    .defending_player_of(host)
                    .is_some_and(|d| d != controller);
                if attacking_your_opponent {
                    return vec![Event::TempBoost {
                        object: host,
                        power,
                        toughness,
                        keywords: &[],
                        source_name,
                    }];
                }
                vec![Event::LifeChanged {
                    player: self.controller_of(host),
                    amount: -(life as i32),
                    source: Some(source),
                }]
            }
            // Mass keyword strip: every creature an opponent of the controller controls loses
            // `keywords` and can't have them until end of turn (arcane_lighthouse).
            Effect::StripKeywordsFromOpponentsCreatures { keywords } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id) && self.controller_of(id) != controller
                })
                .map(|object| Event::KeywordsStripped { object, keywords })
                .collect(),
            // Mass weaken: every creature gets -power/-toughness until end of turn (a negative
            // TempBoost, cleared at cleanup). A 0-or-less-toughness creature dies to the next SBA. (CR 704, CR 514)
            Effect::WeakenEachCreature {
                power,
                toughness,
                opponents_only,
            } => {
                let power = self.resolve_amount(power, controller, source, target, x);
                let toughness = self.resolve_amount(toughness, controller, source, target, x);
                self.battlefield()
                    .into_iter()
                    .filter(|&id| self.is_creature_on_battlefield(id))
                    .filter(|&id| !opponents_only || self.controller_of(id) != controller)
                    .map(|object| Event::TempBoost {
                        object,
                        power: -power,
                        toughness: -toughness,
                        keywords: &[],
                        source_name,
                    })
                    .collect()
            }

            _ => unreachable!("pump family mint received a non-family effect"),
        }
    }
}
