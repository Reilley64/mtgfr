use crate::*;

/// What a would-be counter placement is aimed at (CR 122.1 — counters sit on permanents and on
/// players). Engine-internal: the key [`ReplacementRegistry::counter_replaced_amount`] answers a
/// CR 614 counter replacement against.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterRecipient {
    Permanent(ObjectId),
    Player(PlayerId),
}

/// Whether a replacement's declared recipient scope covers the recipient at hand.
fn recipients_accept(recipients: CounterRecipients, is_permanent: bool) -> bool {
    match recipients {
        CounterRecipients::Permanents => is_permanent,
        CounterRecipients::Players => !is_permanent,
        CounterRecipients::PermanentsAndPlayers => true,
    }
}

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
        /// Bloatfly Swarm's variant: removes *that many* counters (not exactly one) and hands
        /// every player that many rad counters.
        scales: bool,
        /// Rock Hydra's variant: the prevention is worded **per point** ("for each 1 damage …
        /// remove a +1/+1 counter … and prevent that 1 damage"), so it covers only as many
        /// points as there are counters and the rest of the hit is dealt for real. The other two
        /// prevent the whole event however big it is.
        per_point: bool,
    },
    PreventDamage {
        object: ObjectId,
        to_self: bool,
        by_self: bool,
        combat_only: bool,
        source_filter: Option<PermanentFilter>,
        source_relation: Option<SourceRelation>,
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
        halve: bool,
        other: bool,
        any_kind: bool,
        placer: Option<CounterPlacer>,
        recipients: CounterRecipients,
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
                // "As long as you control another creature, …" (Bronze Horse) — a static ability's
                // `condition` reads as its CR 613 "as long as" gate, re-checked every time the
                // registry is built, so the shield turns off the moment the condition stops
                // holding. `None` on every other replacement static here.
                if let Some(condition) = ability.condition
                    && !game.ability_condition_holds(
                        condition,
                        source,
                        TriggerContext::of(controller),
                    )
                {
                    continue;
                }
                match ability.effect {
                    Effect::Static(StaticEffect::PreventDamageToSelfRemovingCounter) => {
                        effects.push(ReplacementEffect::PreventDamageToSelfRemovingCounter {
                            object: source,
                            scales: false,
                            per_point: false,
                        });
                    }
                    Effect::Static(StaticEffect::PreventDamageToSelfRemovingCountersGivingRad) => {
                        effects.push(ReplacementEffect::PreventDamageToSelfRemovingCounter {
                            object: source,
                            scales: true,
                            per_point: false,
                        });
                    }
                    Effect::Static(StaticEffect::PreventDamageToSelfRemovingCounterPerPoint) => {
                        effects.push(ReplacementEffect::PreventDamageToSelfRemovingCounter {
                            object: source,
                            scales: false,
                            per_point: true,
                        });
                    }
                    Effect::Static(StaticEffect::PreventDamage {
                        to_self,
                        by_self,
                        combat_only,
                        source_filter,
                        source_relation,
                        attached,
                    }) => {
                        // Gaseous Form / Demonic Torment: the shield is worded about "enchanted
                        // creature", so it stands on this Aura's host rather than on the Aura
                        // itself. An unattached Aura shields nothing (CR 303.4 — it would already
                        // be in the graveyard).
                        let shielded = match attached {
                            false => source,
                            true => match game.attached_to(source) {
                                Some(host) => host,
                                None => continue,
                            },
                        };
                        effects.push(ReplacementEffect::PreventDamage {
                            object: shielded,
                            to_self,
                            by_self,
                            combat_only,
                            source_filter,
                            source_relation,
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
                        halve,
                        other,
                        any_kind,
                        placer,
                        recipients,
                        filter,
                    }) => {
                        // A level-gated replacement (Innkeeper's Talent's level 3) functions only
                        // at or above its level (CR 717.5).
                        if ability.min_level > game.as_permanent(source).map_or(0, |p| p.level) {
                            continue;
                        }
                        effects.push(ReplacementEffect::CounterReplacement {
                            source,
                            controller,
                            add,
                            times,
                            halve,
                            other,
                            any_kind,
                            placer,
                            recipients,
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

    /// Whether a shield on `target` prevents a whole damage event however big it is — Phantom
    /// Centaur's and Bloatfly Swarm's. Bloatfly Swarm's only applies while it has a counter to
    /// remove (its prevention is worded off the removal), where Phantom Centaur's prevents
    /// unconditionally and just removes nothing.
    ///
    /// Rock Hydra's per-point variant is deliberately *not* one of these: it covers a point at a
    /// time, so it is spent inside
    /// [`Game::per_point_counter_shield`](crate::Game::per_point_counter_shield) where the
    /// leftover can go on to be dealt, rather than short-circuiting the event here.
    pub(crate) fn phantom_shield_active(&self, game: &Game, target: ObjectId) -> bool {
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventDamageToSelfRemovingCounter {
                object,
                scales,
                per_point,
            } => *object == target && !*per_point && (!*scales || game.plus_counters(target) > 0),
            _ => false,
        })
    }

    /// Whether the shield on `target` is Bloatfly Swarm's "remove that many"/rad variant.
    pub(crate) fn phantom_shield_scales(&self, target: ObjectId) -> bool {
        self.effects.iter().any(|effect| {
            matches!(
                effect,
                ReplacementEffect::PreventDamageToSelfRemovingCounter {
                    object,
                    scales: true,
                    ..
                } if *object == target
            )
        })
    }

    /// Whether the shield on `target` is Rock Hydra's per-point variant, which covers only as
    /// many points as it has counters and lets the rest of the hit through.
    pub(crate) fn phantom_shield_per_point(&self, target: ObjectId) -> bool {
        self.effects.iter().any(|effect| {
            matches!(
                effect,
                ReplacementEffect::PreventDamageToSelfRemovingCounter {
                    object,
                    per_point: true,
                    ..
                } if *object == target
            )
        })
    }

    /// Whether a permanent's own [`StaticEffect::PreventDamage`] shield stands between `source`
    /// and `target` right now (CR 615). `combat` says whether the damage being dealt is combat
    /// damage, which is the only thing a `combat_only` shield stops.
    pub(crate) fn damage_prevented_to_permanent(
        &self,
        game: &Game,
        target: ObjectId,
        source: ObjectId,
        combat: bool,
    ) -> bool {
        self.effects.iter().any(|effect| {
            let ReplacementEffect::PreventDamage {
                object,
                to_self: true,
                combat_only,
                source_filter,
                source_relation,
                ..
            } = effect
            else {
                return false;
            };
            if *object != target || (*combat_only && !combat) {
                return false;
            }
            // "… by enchanted creatures" / "… by Walls": read from the shielded permanent's own
            // controller's perspective, with the shield's permanent as the filter's source.
            if let Some(filter) = source_filter
                && !game.permanent_matches(filter, source, game.controller_of(target), Some(target))
            {
                return false;
            }
            source_relation.is_none_or(|relation| game.source_relates(relation, source, target))
        })
    }

    pub(crate) fn combat_damage_prevented_by_source(&self, source: ObjectId) -> bool {
        self.effects.iter().any(|effect| match effect {
            ReplacementEffect::PreventDamage {
                object,
                by_self: true,
                ..
            } => *object == source,
            _ => false,
        })
    }

    /// Every applicable [`Effect::Static(StaticEffect::CounterReplacement)`] on the battlefield
    /// applies once to a would-be placement of `base` counters.
    ///
    /// Two independent gates decide whether a replacement sees a placement. `placer` — who *would
    /// put* the counters (CR 614.1) — answers Vorinclex's "if **you would put** …" / "if an
    /// **opponent would put** …". The recipient axis (`recipients`, `filter`, and, when the
    /// replacement names no placer, the receiving side itself) answers Winding Constrictor's
    /// passive "if one or more counters **would be put on** an artifact or creature you control".
    /// A card keys off one or the other, never both.
    ///
    /// ponytail: fixed order — additions, then multipliers, then halvings:
    /// `((base + Σadd) × Πtimes) ÷ 2^halvings`. CR 616.1 lets the *affected player* order
    /// simultaneous replacements, and once a halving (Vorinclex's opponent clause) is in the mix
    /// the order genuinely changes the result. Offer a real ordering choice when a board can hold
    /// both a halving and an adder/doubler at once.
    pub(crate) fn counter_replaced_amount(
        &self,
        game: &Game,
        placer: PlayerId,
        recipient: CounterRecipient,
        plus_one: bool,
        base: i32,
    ) -> i32 {
        if base <= 0 {
            return base;
        }
        let (side, object) = match recipient {
            CounterRecipient::Permanent(id) => (game.controller_of(id), Some(id)),
            CounterRecipient::Player(player) => (player, None),
        };
        let mut add = 0;
        let mut times = 1;
        let mut halvings = 0u32;
        for effect in &self.effects {
            let ReplacementEffect::CounterReplacement {
                source,
                controller,
                add: next_add,
                times: next_times,
                halve,
                other,
                any_kind,
                placer: placer_gate,
                recipients,
                filter,
            } = effect
            else {
                continue;
            };
            // Which side of the table the replacement watches. A placer-keyed clause ("if you
            // would put …" / "if an opponent would put …") reads who is putting the counters;
            // everything else reads the recipient's own side ("… you control").
            let watched = match placer_gate {
                Some(CounterPlacer::You) => *controller == placer,
                Some(CounterPlacer::Opponents) => *controller != placer,
                None => *controller == side,
            };
            if !watched {
                continue;
            }
            // "one or more +1/+1 counters" doesn't see a charge or -1/-1 counter.
            if !any_kind && !plus_one {
                continue;
            }
            if !recipients_accept(*recipients, object.is_some()) {
                continue;
            }
            // CR "another creature you control": a replacement that excludes its own source
            // doesn't apply when the permanent receiving the counters IS that source (Benevolent
            // Hydra doesn't double its own counters).
            if *other && object == Some(*source) {
                continue;
            }
            // Ozolith's "an artifact or creature you control": a type gate on the recipient, read
            // from the replacement's own controller's perspective.
            if let (Some(filter), Some(object)) = (filter, object)
                && !game.permanent_matches(filter, object, *controller, Some(*source))
            {
                continue;
            }
            add += *next_add;
            times *= *next_times;
            halvings += u32::from(*halve);
        }
        let mut n = (base + add) * times;
        for _ in 0..halvings {
            n /= 2;
        }
        n
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
        // Lich's "If you would gain life, draw that many cards instead" (CR 614) — the one
        // replacement here that swaps the event out entirely rather than adjusting a number on
        // it, so it short-circuits: no `LifeChanged` is ever applied, and nothing watching life
        // gain sees anything. Routed through `draw_with_dredge` like any other draw, so the
        // replacement cards can themselves be dredged.
        if let Event::LifeChanged { player, amount, .. } = event
            && amount > 0
            && self.life_gain_becomes_draw(player)
        {
            self.draw_with_dredge(player, amount as u32, false, events);
            return;
        }
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
        let n = self.counters_after_replacements(controller, permanent, bonus);
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
