//! Pump-family event mint — pure Event vectors for related [`Effect`] variants.
//!
//! Called only from the private mint path behind [`Game::run`] (card-dsl-and-card-pool spec / explore-all deepen).
//! Apply stays in [`crate::apply`]; this module never mutates the board.

use crate::*;

impl Game {
    pub(crate) fn mint_pump(
        &self,
        effect: PumpEffect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        let source_name = self.source_name_of(source);
        match effect {
            // Pump / destroy / counters target a creature, so the chosen target is an object.
            PumpEffect::PumpUntilEndOfTurn {
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
            PumpEffect::PumpSelfUntilEndOfTurn {
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
            // Mother of Runes: "{T}: Target creature you control gains protection from the color
            // of your choice until end of turn." A preceding `Effect::ChooseColor` step in the
            // same `Sequence` has already stored the chosen color on the ability's own `source`
            // (`Permanent::chosen_color`) by the time this step runs. No P/T change — a
            // keyword-only `TempBoost`, the single-`Keyword` twin of `PumpUntilEndOfTurn`'s
            // `&'static` `keywords` slice: the scope isn't known until resolution, so it's leaked
            // fresh here rather than baked in at TOML-parse time.
            PumpEffect::GrantChosenColorProtectionUntilEndOfTurn { .. } => {
                let object = expect_object_target(target, "a chosen-color protection grant");
                let Some(color) = self.as_permanent(source).and_then(|p| p.chosen_color) else {
                    // The color choice never landed (e.g. the source left before resolution) —
                    // nothing to grant.
                    return Vec::new();
                };
                vec![Event::TempBoost {
                    object,
                    power: 0,
                    toughness: 0,
                    keywords: Box::leak(Box::new([Keyword::ProtectionFrom(
                        ProtectionScope::Color(color),
                    )])),
                    source_name,
                }]
            }
            // Bathe in Light: "Target creature and each other creature that shares a color with
            // it gain protection from the chosen color until end of turn." The batch-capable
            // twin of `GrantChosenColorProtectionUntilEndOfTurn` right above — same preceding
            // `choose_color` step — but the grant lands on every creature in `Game::radiance_batch`
            // of the chosen target (the old "Radiance" keyword action, CR 105.2), not just the
            // target itself. `source` here is the spell itself (an instant), not a permanent, so
            // the choice reads back via `chosen_color_of` (checks `Spell::chosen_color`) rather
            // than `Permanent::chosen_color` directly. One leaked keyword slice is shared across
            // every creature's `TempBoost` (the same protection scope for all).
            PumpEffect::RadianceChosenColorProtectionUntilEndOfTurn { .. } => {
                let chosen = expect_object_target(target, "a radiance protection grant");
                let Some(color) = self.chosen_color_of(source) else {
                    // The color choice never landed (e.g. the source left before resolution) —
                    // nothing to grant.
                    return Vec::new();
                };
                let keywords: &'static [Keyword] = Box::leak(Box::new([Keyword::ProtectionFrom(
                    ProtectionScope::Color(color),
                )]));
                self.radiance_batch(chosen)
                    .into_iter()
                    .map(|object| Event::TempBoost {
                        object,
                        power: 0,
                        toughness: 0,
                        keywords,
                        source_name,
                    })
                    .collect()
            }
            // Mass pump: every creature the controller controls, no target (Selfless Spirit,
            // Moonshaker Cavalry).
            PumpEffect::PumpCreaturesYouControlUntilEndOfTurn {
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
            // Mass pump, every controller: every creature on the battlefield matching `filter`,
            // not just the controller's own (Bladewing the Risen). The board-wide twin of
            // `PumpCreaturesYouControlUntilEndOfTurn` right above — same filter, no
            // `controller_of(id) == controller` gate.
            PumpEffect::PumpEachCreatureUntilEndOfTurn {
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
            PumpEffect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { keywords, filter } => {
                self.battlefield()
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
                    .collect()
            }
            // Mass base-P/T SET: every creature the controller controls has its base P/T set to
            // `power`/`toughness` until end of turn (Biomass Mutation). Same "you control" scan as
            // the mass pump, but a 7b base SET rather than a 7c delta.
            PumpEffect::SetBasePtCreaturesYouControlUntilEndOfTurn {
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
                        ends_at_end_of_combat: false,
                    })
                    .collect()
            }
            // Single-target base-P/T SET: the chosen creature's base P/T is set until end of turn
            // (Quandrix Charm mode 2). The targeted twin of the mass set above.
            PumpEffect::SetBasePtTargetUntilEndOfTurn {
                power, toughness, ..
            } => {
                let object = expect_object_target(target, "a base-P/T set");
                vec![Event::BasePtSetUntilEndOfTurn {
                    object,
                    power: self.resolve_amount(power, controller, source, target, x),
                    toughness: self.resolve_amount(toughness, controller, source, target, x),
                    ends_at_end_of_combat: false,
                }]
            }
            // Indefinite self base-P/T SET (Trench Gorger's "this creature has base power and
            // toughness each equal to the number of cards exiled this way", CR 613.3(7b)): unlike
            // `SetBasePtTargetUntilEndOfTurn` above, this is never cleared at cleanup. Nothing to
            // do if the source has already left (CR 608.2c).
            PumpEffect::SetOwnBasePtFromAmount { amount } => {
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
            PumpEffect::AnimateSelfUntilEndOfTurn {
                add_types,
                add_subtypes,
                base_power,
                base_toughness,
                keywords,
                add_colors,
                ends_at_end_of_combat,
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
                        ends_at_end_of_combat,
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
            PumpEffect::PumpOtherAttackersAttackingYourOpponents { power, toughness } => {
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
            PumpEffect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
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
            // Earthbind: "this Aura gains 'Enchanted creature loses flying.'" The gained ability
            // is modelled as the loss itself, recorded on the Aura — see
            // `Event::AttachedKeywordsLost`. An Aura whose host left in response has nothing to
            // ground (CR 704.5m).
            PumpEffect::EnchantedCreatureLosesKeywords { keywords } => {
                let Some(host) = self.attached_to(source) else {
                    return Vec::new();
                };
                vec![Event::AttachedKeywordsLost {
                    source,
                    object: host,
                    keywords,
                }]
            }
            // Targeted keyword removal (CR 613.1f — the Legends strippers). A target that has left
            // since (CR 608.2b) leaves nothing to strip. `choose_one` never reaches here: the mode
            // pause peels it off first (see `Game::run`).
            PumpEffect::TargetLosesKeywords {
                keywords,
                families,
                until_end_of_turn,
                ..
            } => {
                let object = expect_object_target(target, "a keyword loss");
                if self.as_permanent(object).is_none() {
                    return Vec::new();
                }
                vec![Event::KeywordsStripped {
                    object,
                    keywords,
                    families,
                    until_end_of_turn,
                    cant_have: false,
                }]
            }
            // Mass keyword strip: every creature an opponent of the controller controls loses
            // `keywords` and can't have them until end of turn (arcane_lighthouse).
            PumpEffect::StripKeywordsFromOpponentsCreatures { keywords } => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id) && self.controller_of(id) != controller
                })
                .map(|object| Event::KeywordsStripped {
                    object,
                    keywords,
                    families: &[],
                    until_end_of_turn: true,
                    cant_have: true,
                })
                .collect(),
            // Vesuvan Doppelganger's upkeep: "you may have this creature become a copy of target
            // creature, except it doesn't copy that creature's color and it has this ability."
            // The *source* is what gets rewritten, so a target that has left the battlefield since
            // (CR 608.2b) — or a shapeshifter that has itself left — is simply no re-copy.
            PumpEffect::BecomesCopyOfTarget {
                keeps_own_color,
                keeps_own_abilities,
                ..
            } => {
                let object = expect_object_target(target, "becomes a copy");
                let (Some(keeper), Some(copied)) = (
                    self.as_permanent(source).map(|p| p.def),
                    self.as_permanent(object).map(|p| p.def),
                ) else {
                    return Vec::new();
                };
                vec![Event::BecameCopy {
                    object: source,
                    def: intern_card_def(copy_with_exceptions(
                        (*card_def(copied)).clone(),
                        &card_def(keeper),
                        keeps_own_color,
                        keeps_own_abilities,
                    )),
                    until_eot: false,
                    also_types: TypeSet::NONE,
                }]
            }
            // Vraska, Betrayal's Sting's −2: the target creature becomes a Treasure artifact and
            // loses all other card types and abilities (CR 613.1d/613.1f) — an indefinite def
            // overwrite, so `BecameCopy`'s `until_eot: false` never restores it. A target that has
            // left the battlefield since is skipped (CR 608.2b).
            PumpEffect::TargetBecomesTreasure { .. } => {
                let object = expect_object_target(target, "becomes a Treasure");
                let Some(current) = self.as_permanent(object).map(|p| p.def) else {
                    return Vec::new();
                };
                vec![Event::BecameCopy {
                    object,
                    def: intern_card_def(becomes_treasure((*card_def(current)).clone())),
                    until_eot: false,
                    also_types: TypeSet::NONE,
                }]
            }
            // "…becomes the color of your choice" is peeled to the color picker by `Game::run`
            // before it ever reaches the minter (see `run_choose_pause`), so there is no color to
            // set here.
            PumpEffect::TargetBecomesColor { color: None, .. } => Vec::new(),
            // "Target spell or permanent becomes black" (the lace cycle, no printed duration, so
            // the SET rides the object) and "one or more target creatures become red until end of
            // turn" (the Legends colour-wash cycle, swept at cleanup) — one layer-5 SET, told
            // apart by the duration the card printed. A `color` of `None` ("of your choice",
            // Alchor's Tomb) never reaches here: `Game::run` peels it to the color picker first.
            // Nothing to do if the target already left the stack or the battlefield (CR 608.2b;
            // `target_still_legal` normally fizzles this first).
            PumpEffect::TargetBecomesColor {
                color: Some(color),
                until_end_of_turn,
                ..
            } => {
                let object = expect_object_target(target, "becomes a color");
                if !matches!(
                    self.objects[object as usize],
                    Object::Permanent(_) | Object::Spell(_)
                ) {
                    return Vec::new();
                }
                vec![Event::ColorSet {
                    object,
                    color,
                    until_end_of_turn,
                }]
            }
            // "Target land becomes a Forest until this creature leaves the battlefield" (Gaea's
            // Liege): the whole land-type line is replaced (CR 305.7), and the entry names the
            // source so the read side can check it is still there. A target that has left the
            // battlefield since is skipped (CR 608.2b).
            PumpEffect::TargetBecomesSubtypesWhileSourceRemains { set_subtypes, .. } => {
                let object = expect_object_target(target, "becomes a land type");
                if self.as_permanent(object).is_none() {
                    return Vec::new();
                }
                vec![Event::SubtypesSetWhileSourceRemains {
                    object,
                    subtypes: set_subtypes,
                    source,
                }]
            }
            // Mass weaken: every creature gets -power/-toughness until end of turn (a negative
            // TempBoost, cleared at cleanup). A 0-or-less-toughness creature dies to the next SBA. (CR 704, CR 514)
            PumpEffect::WeakenEachCreature {
                power,
                toughness,
                opponents_only,
            } => {
                // Both amounts are resolved once per affected creature, relative to *that*
                // creature's controller, not the effect's — Phyresis Outbreak's "for each poison
                // counter its controller has" (CR 122.1) gives each opponent's creatures a
                // different -N/-N. `resolve_amount`'s `controller` argument is "the player this
                // amount is relative to"; for a controller-independent amount (`Fixed`, `X`) that
                // distinction is invisible, which is why the flat weakeners are unaffected.
                self.battlefield()
                    .into_iter()
                    .filter(|&id| self.is_creature_on_battlefield(id))
                    .filter(|&id| !opponents_only || self.controller_of(id) != controller)
                    .map(|object| {
                        let relative_to = self.controller_of(object);
                        let power = self.resolve_amount(power, relative_to, source, target, x);
                        let toughness =
                            self.resolve_amount(toughness, relative_to, source, target, x);
                        Event::TempBoost {
                            object,
                            power: -power,
                            toughness: -toughness,
                            keywords: &[],
                            source_name,
                        }
                    })
                    .collect()
            }
        }
    }
}
