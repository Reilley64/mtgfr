use crate::*;

#[derive(Clone)]
pub(crate) struct ReplacementRegistry {
    effects: Vec<ReplacementEffect>,
}

#[derive(Clone)]
enum ReplacementEffect {
    CombatDamagePreventionShield {
        player: PlayerId,
        token: CardId,
    },
    PreventAllCombatDamageThisTurn,
    PreventDamageToSelfRemovingCounter {
        object: ObjectId,
    },
    PreventCombatDamage {
        object: ObjectId,
        to_self: bool,
        by_self: bool,
    },
    PreventNoncombatDamageToOtherCreaturesYouControl {
        source: ObjectId,
        controller: PlayerId,
    },
    CounterReplacement {
        source: ObjectId,
        controller: PlayerId,
        add: i32,
        times: i32,
        other: bool,
        filter: Option<PermanentFilter>,
    },
    CreaturesYouControlEnterWithCounters {
        source: ObjectId,
        controller: PlayerId,
        filter: PermanentFilter,
        count: Amount,
    },
    TokenReplacement {
        controller: PlayerId,
        times: i32,
    },
    LifeGainReplacement {
        controller: PlayerId,
        plus: i32,
    },
}

impl ReplacementRegistry {
    pub(crate) fn new(game: &Game) -> Self {
        let mut effects = Vec::new();
        for &(player, ref token) in &game.combat_extras.combat_damage_prevention_shields {
            effects.push(ReplacementEffect::CombatDamagePreventionShield {
                player,
                token: intern_card_def(token.clone()),
            });
        }
        if game.combat_extras.prevent_all_combat_damage_this_turn {
            effects.push(ReplacementEffect::PreventAllCombatDamageThisTurn);
        }
        for source in game.battlefield() {
            let controller = game.controller_of(source);
            for ability in game.functional_abilities(source).iter().cloned() {
                if ability.timing != Timing::Static {
                    continue;
                }
                match ability.effect {
                    Effect::Static(StaticEffect::PreventDamageToSelfRemovingCounter) => {
                        effects.push(ReplacementEffect::PreventDamageToSelfRemovingCounter {
                            object: source,
                        });
                    }
                    Effect::Static(StaticEffect::PreventCombatDamage { to_self, by_self }) => {
                        effects.push(ReplacementEffect::PreventCombatDamage {
                            object: source,
                            to_self,
                            by_self,
                        });
                    }
                    Effect::Static(
                        StaticEffect::PreventNoncombatDamageToOtherCreaturesYouControl,
                    ) => {
                        effects.push(
                            ReplacementEffect::PreventNoncombatDamageToOtherCreaturesYouControl {
                                source,
                                controller,
                            },
                        );
                    }
                    Effect::Static(StaticEffect::CounterReplacement {
                        add,
                        times,
                        other,
                        filter,
                    }) => {
                        effects.push(ReplacementEffect::CounterReplacement {
                            source,
                            controller,
                            add,
                            times,
                            other,
                            filter,
                        });
                    }
                    Effect::Static(StaticEffect::CreaturesYouControlEnterWithCounters {
                        filter,
                        count,
                    }) => {
                        effects.push(ReplacementEffect::CreaturesYouControlEnterWithCounters {
                            source,
                            controller,
                            filter,
                            count,
                        });
                    }
                    Effect::Static(StaticEffect::TokenReplacement { times }) => {
                        effects.push(ReplacementEffect::TokenReplacement { controller, times });
                    }
                    Effect::Static(StaticEffect::LifeGainReplacement { plus }) => {
                        effects.push(ReplacementEffect::LifeGainReplacement { controller, plus });
                    }
                    _ => {}
                }
            }
        }
        Self { effects }
    }

    pub(crate) fn prevents_all_combat_damage(&self) -> bool {
        self.effects
            .iter()
            .any(|effect| matches!(effect, ReplacementEffect::PreventAllCombatDamageThisTurn))
    }

    pub(crate) fn combat_damage_prevention_token_for_player(
        &self,
        player: PlayerId,
    ) -> Option<CardId> {
        self.effects.iter().find_map(|effect| match effect {
            ReplacementEffect::CombatDamagePreventionShield {
                player: shielded,
                token,
            } if *shielded == player => Some(*token),
            _ => None,
        })
    }

    pub(crate) fn noncombat_damage_prevented_to_creature(
        &self,
        game: &Game,
        target: ObjectId,
    ) -> bool {
        let controller = game.controller_of(target);
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventNoncombatDamageToOtherCreaturesYouControl {
                source,
                controller: shield_controller,
            } => *shield_controller == controller && *source != target,
            _ => false,
        })
    }

    pub(crate) fn phantom_shield_active(&self, target: ObjectId) -> bool {
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventDamageToSelfRemovingCounter { object } => *object == target,
            _ => false,
        })
    }

    pub(crate) fn combat_damage_prevented_to_creature(&self, target: ObjectId) -> bool {
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventCombatDamage {
                object,
                to_self: true,
                ..
            } => *object == target,
            _ => false,
        })
    }

    pub(crate) fn combat_damage_prevented_by_source(&self, source: ObjectId) -> bool {
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventCombatDamage {
                object,
                by_self: true,
                ..
            } => *object == source,
            _ => false,
        })
    }

    pub(crate) fn counter_replaced_amount(&self, game: &Game, object: ObjectId, base: i32) -> i32 {
        if base <= 0 {
            return base;
        }
        let controller = game.controller_of(object);
        let mut add = 0;
        let mut times = 1;
        for effect in &self.effects {
            let ReplacementEffect::CounterReplacement {
                source,
                controller: replacement_controller,
                add: next_add,
                times: next_times,
                other,
                filter,
            } = effect
            else {
                continue;
            };
            if *replacement_controller != controller {
                continue;
            }
            if *other && *source == object {
                continue;
            }
            if filter.is_some_and(|f| {
                !game.permanent_matches(&f, object, *replacement_controller, Some(*source))
            }) {
                continue;
            }
            add += *next_add;
            times *= *next_times;
        }
        (base + add) * times
    }

    pub(crate) fn additional_enter_counters(
        &self,
        game: &Game,
        entered: ObjectId,
        controller: PlayerId,
    ) -> i32 {
        let mut total = 0;
        for effect in &self.effects {
            let ReplacementEffect::CreaturesYouControlEnterWithCounters {
                source,
                controller: effect_controller,
                filter,
                count,
            } = effect
            else {
                continue;
            };
            if *source == entered || *effect_controller != controller {
                continue;
            }
            if !game.permanent_matches(filter, entered, controller, Some(*source)) {
                continue;
            }
            total += game.resolve_count(*count, controller, *source, None, 0) as i32;
        }
        total
    }

    pub(crate) fn token_replaced_amount(&self, recipient: PlayerId, base: u32) -> u32 {
        if base == 0 {
            return 0;
        }
        let mut product = 1_u32;
        for effect in &self.effects {
            let ReplacementEffect::TokenReplacement { controller, times } = effect else {
                continue;
            };
            if *controller != recipient {
                continue;
            }
            product *= (*times).max(0) as u32;
        }
        base * product
    }

    pub(crate) fn life_gain_replaced_amount(&self, recipient: PlayerId, base: i32) -> i32 {
        if base <= 0 {
            return base;
        }
        let mut total = 0;
        for effect in &self.effects {
            let ReplacementEffect::LifeGainReplacement { controller, plus } = effect else {
                continue;
            };
            if *controller != recipient {
                continue;
            }
            total += *plus;
        }
        base + total
    }
}

impl Game {
    pub(crate) fn replacement_registry(&self) -> ReplacementRegistry {
        ReplacementRegistry::new(self)
    }

    pub(crate) fn push_apply_effect_event(&mut self, events: &mut Vec<Event>, event: Event) {
        let entry = match &event {
            Event::ReanimatedToBattlefield {
                permanent,
                controller,
                ..
            }
            | Event::ReturnedFromLinkedExile {
                permanent,
                controller,
                ..
            }
            | Event::FlickeredToBattlefield {
                permanent,
                controller,
                ..
            }
            | Event::SearchedToBattlefield {
                permanent,
                controller,
                ..
            }
            | Event::PutOntoBattlefieldFromHand {
                permanent,
                controller,
                ..
            } => Some((*permanent, *controller)),
            _ => None,
        };
        self.push_apply(events, event);
        if let Some((permanent, controller)) = entry {
            self.push_nonspell_entry_counter_replacements(permanent, controller, events);
        }
    }

    pub(crate) fn apply_effect_events_with_replacements(
        &mut self,
        effect_events: Vec<Event>,
        events: &mut Vec<Event>,
    ) {
        for event in effect_events {
            self.push_apply_effect_event(events, event);
        }
    }

    fn push_nonspell_entry_counter_replacements(
        &mut self,
        permanent: ObjectId,
        controller: PlayerId,
        events: &mut Vec<Event>,
    ) {
        let printed = self.def_of(permanent);
        self.push_enters_with_counters(&printed, permanent, controller, None, 0, events);
        let bonus = self.additional_enter_counters(permanent, controller);
        let n = self.counters_after_replacements(permanent, bonus);
        if n <= 0 {
            return;
        }
        self.push_apply(
            events,
            Event::CountersPlaced {
                object: permanent,
                count: n,
                source_name: printed.name,
            },
        );
    }
}
