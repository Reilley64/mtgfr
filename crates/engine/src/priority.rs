//! Priority, turn structure, turn-based actions, and cleanup.
//!
//! Turn phases/steps, passing priority, turn-based actions (untap, draw, combat steps
//! advance), cleanup. Also: mana abilities / auto-tap planning (CR 605, ADR 0007).
//! Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// One planned mana-source tap toward a payment ([`Game::plan_auto_taps`]): a land's free
/// base `produces` tap, or a permanent's tap-for-mana ability (free or paid filter/karoo) at the
/// given index. Paid abilities are ordered feed-first so nested [`Game::settle_payment`] inside
/// [`Game::activate_ability`] only spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlannedTap {
    Base(ObjectId),
    Ability(ObjectId, usize),
}

/// A free-tap candidate for [`Game::plan_auto_taps`].
struct FreeTapCandidate {
    tap: PlannedTap,
    credit: ManaPool,
    /// Non-lands sort after lands so Forests are preferred over rocks/dorks.
    nonland: bool,
    pain: bool,
    breadth: u8,
}

/// A paid tap-for-mana candidate (filter land, karoo, signet) for [`Game::plan_auto_taps`].
struct PaidTapCandidate {
    tap: PlannedTap,
    source: ObjectId,
    activation: Cost,
    credit: ManaPool,
    nonland: bool,
    breadth: u8,
}

fn mana_serves(credit: &ManaPool, color: usize) -> bool {
    credit.colored[color] > 0
        || credit.any > 0
        || COLOR_PAIRS
            .iter()
            .zip(credit.either.iter())
            .any(|(&(a, b), &n)| n > 0 && (a.index() == color || b.index() == color))
        || credit
            .of_colors
            .iter()
            .enumerate()
            .any(|(mask, &n)| n > 0 && (mask & (1 << color)) != 0)
}

fn mana_breadth(credit: &ManaPool) -> u8 {
    (0..Color::COUNT)
        .filter(|&c| mana_serves(credit, c))
        .count() as u8
}

impl Game {
    /// The index of `object`'s own *free-tap* mana ability — one that costs nothing but tapping
    /// itself. That's the ability a "tap this for mana" click invokes. An ability with any further
    /// cost (Fetid Heath's `{W/B}, {T}` filter mode; a Treasure's sacrifice) is a real activation
    /// the player must be shown paying, not something a bare click may spend on their behalf.
    pub(crate) fn free_tap_mana_ability(&self, object: ObjectId) -> Option<usize> {
        let Object::Permanent(perm) = &self.objects[object as usize] else {
            return None;
        };
        perm.def.abilities.iter().position(|a| {
            a.effect.is_mana_ability()
                && matches!(a.timing, Timing::Activated(cost)
                    if cost.taps_self
                        && cost.mana == Cost::FREE
                        && cost.sacrifice == SacrificeCost::None
                        && cost.pay_life == Amount::Fixed(0))
        })
    }

    /// Whether tapping `object` produces mana: a land with the `produces` sugar, or any permanent
    /// with a free-tap mana ability (Sol Ring, Arcane Signet, Llanowar Elves). The client shows the
    /// tap-for-mana affordance on exactly these, so a click never fires an intent that must reject.
    pub fn taps_for_mana(&self, object: ObjectId) -> bool {
        let Object::Permanent(perm) = &self.objects[object as usize] else {
            return false;
        };
        if let CardKind::Land {
            produces: Some(_), ..
        } = perm.def.kind
        {
            return true;
        }
        self.free_tap_mana_ability(object).is_some()
    }

    /// Leave the game (CR 104.3a). Cannot fail: a player may always quit, with or without priority,
    /// and conceding twice is a no-op rather than an error — the second one just finds them gone.
    ///
    /// `submit` sweeps state-based actions afterwards (declaring a winner if one is left) and hands
    /// priority on if the conceding player was holding it.
    ///
    /// ponytail: the conceded player's permanents stay on the battlefield, where CR 800.4a would
    /// remove them. That's not new — every elimination in this engine leaves them, and no pool card
    /// yet depends on the difference. Their turns, priority, and combat are all skipped already.
    pub(crate) fn concede(&mut self, player: PlayerId) -> Vec<Event> {
        if self.has_lost(player) {
            return Vec::new();
        }
        // A quitter can't answer the decision the game is parked on, so drop it. Everything the
        // unanswered effect would have done is forfeited — better than deadlocking three seats on
        // one that has closed the tab.
        if self
            .pending_choice
            .as_ref()
            .is_some_and(|c| c.player() == player)
        {
            self.pending_choice = None;
        }
        let events = vec![Event::PlayerLost { player }];
        self.apply_all(&events);
        events
    }

    /// Tap a permanent under `player`'s control for mana. A mana ability: it uses no stack and
    /// doesn't touch priority.
    pub(crate) fn tap_for_mana(
        &mut self,
        player: PlayerId,
        object: ObjectId,
    ) -> Result<Vec<Event>, Reject> {
        let Object::Permanent(perm) = self.objects[object as usize] else {
            return Err(Reject::CannotProduceMana);
        };
        // A land with the `produces` sugar has a free base tap-for-one. Everything else that makes
        // mana does it with a real ability — Sol Ring, Arcane Signet, a mana dork, and a fetch-only
        // land's *non*-mana ability (which finds none, and rejects below). Delegate so the one (CR 605, CR 113)
        // activation path enforces summoning sickness and the rest of the gate.
        let CardKind::Land {
            produces: Some(produces),
            ..
        } = perm.def.kind
        else {
            let Some(index) = self.free_tap_mana_ability(object) else {
                return Err(Reject::CannotProduceMana);
            };
            return self.activate_ability(player, object, index, None, Vec::new(), 0);
        };
        if perm.owner != player || perm.tapped {
            return Err(Reject::CannotProduceMana);
        }

        // "One mana of any color in your commander's color identity" (CR 903.4, Command Tower)
        // and "any color that a land an opponent controls could produce" (Exotic Orchard) both
        // resolve to a real credit here — an empty identity/producible set taps for nothing.
        let mana = match produces {
            LandProduces::Mana(m) => Some(m),
            LandProduces::CommanderIdentity => self.commander_identity_credit(player),
            LandProduces::OpponentColors => self.opponent_producible_colors_credit(player),
        };

        let mut events = vec![Event::Tapped { object }];
        if let Some(mana) = mana {
            events.push(Event::ManaAdded {
                player,
                mana,
                amount: 1,
                persist: false,
            });
        }
        self.apply_all(&events);
        // Fertile Ground / Mirari's Wake fire off the same tap (CR 605.3 — inline, no stack).
        self.land_tapped_for_mana(object, player, &mut events);
        Ok(events)
    }

    /// The CR "whenever [a land] is tapped for mana" watch: each matching static
    /// [`Effect::TappedForManaBonus`] on the battlefield adds a bonus credit into the tap's own
    /// pool batch. Mana abilities don't stack (CR 605.3), so the bonus resolves inline — no stack,
    /// no priority. Called at both land-tap-for-mana chokes ([`Self::tap_for_mana`]'s `produces`
    /// sugar and an `add_mana` activation on a land). `land` is the just-tapped land, `player` its
    /// controller, `events` the tap's already-applied events (its [`Event::ManaAdded`]s, from which
    /// the produced type for a `Produced` bonus is read). Inline bonuses are `push_apply`ed onto
    /// `events`; an `AnyColor` bonus instead raises a [`PendingChoice::ChooseManaColor`] the caller
    /// returns on.
    pub(crate) fn land_tapped_for_mana(
        &mut self,
        land: ObjectId,
        player: PlayerId,
        events: &mut Vec<Event>,
    ) {
        // Only a *land* tapped for mana is watched (Mirari's Wake: "tap a **land**"; Fertile
        // Ground enchants a land) — a mana rock (Sol Ring) tapping fires nothing. Read the source
        // as a live permanent: a mana ability that sacrifices its own source as a cost (a Treasure)
        // has already removed it by now, and it's no land either way.
        let Some(perm) = self.as_permanent(land) else {
            return;
        };
        if !matches!(perm.def.kind, CardKind::Land { .. }) {
            return;
        }
        // "Tapped for mana" means it produced mana (CR 106.11) — the type this tap made, read back
        // from its own event. A tap that added nothing (empty commander identity) fires no watch.
        let Some(produced) = events.iter().find_map(|e| match e {
            Event::ManaAdded { mana, .. } => Some(*mana),
            _ => None,
        }) else {
            return;
        };

        // Scan the battlefield for matching watchers. `scope` says which taps a watch reacts to
        // (Mirari's Wake's controller — "whenever **you** tap a land", the tapper being the land's
        // controller; Fertile Ground's enchanted host — an Aura on the tapped land); `bonus_color`
        // says what mana it adds (a `Produced` credit inline, or an `AnyColor` credit the
        // controller names via a pause).
        let mut produced_bonuses = 0usize;
        let mut any_color_source: Option<ObjectId> = None;
        for id in self.battlefield() {
            for ability in self.def_of(id).abilities {
                let (Timing::Static, Effect::TappedForManaBonus { scope, bonus_color }) =
                    (ability.timing, ability.effect)
                else {
                    continue;
                };
                let watches = match scope {
                    LandTapScope::Controller => self.controller_of(id) == player,
                    LandTapScope::EnchantedHost => self.attached_to(id) == Some(land),
                };
                if !watches {
                    continue;
                }
                match bonus_color {
                    LandTapBonusColor::Produced => produced_bonuses += 1,
                    // ponytail: only the FIRST any-color watch raises its pause — a second on the
                    // same tap is dropped (the `ChooseManaColor` answer path doesn't re-enter this
                    // watch to queue another). No pool board stacks two. Queue them if one ever does.
                    LandTapBonusColor::AnyColor => {
                        any_color_source.get_or_insert(id);
                    }
                }
            }
        }

        for _ in 0..produced_bonuses {
            self.push_apply(
                events,
                Event::ManaAdded {
                    player,
                    mana: produced,
                    amount: 1,
                    persist: false,
                },
            );
        }
        if let Some(source) = any_color_source {
            pending::raise(
                self,
                pending::ChoiceRequest::ChooseManaColor {
                    player,
                    source,
                    amount: 1,
                },
            );
        }
    }

    /// Pay 1 life to add {C} under Yavimaya Bloomsage's Channel grant (a CR 605 mana ability —
    /// doesn't use the stack). Legal only while
    /// [`Player::channel_colorless_mana_this_turn`] holds and the player can afford the life
    /// payment (CR 119.4).
    /// ponytail: no source permanent to hang this off of (Channel is spent from hand — see
    /// [`Effect::GrantChannelColorlessManaThisTurn`]'s doc), so it's a standalone `Intent` rather
    /// than a `Game::ability_at`-addressed granted ability; offered whenever the flag holds, with
    /// no independent "any time you could activate a mana ability" timing gate.
    pub(crate) fn channel_colorless_mana(
        &mut self,
        player: PlayerId,
    ) -> Result<Vec<Event>, Reject> {
        if !self.players[player.0 as usize].channel_colorless_mana_this_turn {
            return Err(Reject::CannotProduceMana);
        }
        if self.life(player) < 1 {
            return Err(Reject::CannotProduceMana);
        }
        let events = vec![
            Event::LifeChanged {
                player,
                amount: -1,
                source: None,
            },
            Event::ManaAdded {
                player,
                mana: Mana::Colorless,
                amount: 1,
                persist: false,
            },
        ];
        self.apply_all(&events);
        Ok(events)
    }

    /// Whether `player` may take a sorcery-speed action right now: it's their turn, a
    /// main phase, and the stack is empty.
    pub(crate) fn can_take_sorcery_speed_action(&self, player: PlayerId) -> bool {
        player == self.active_player
            && matches!(self.step, Step::Main1 | Step::Main2)
            && self.stack.is_empty()
    }

    /// Whether the chosen `target` is legal for `controller` casting `def`: nothing chosen iff
    /// the card takes no target, otherwise the choice must be in the card's legal-target set
    /// ([`Game::legal_targets_for`] — the same set the client highlights and auto-pass reads).
    /// `x` is the caster's chosen `{X}` (CR 601.2b — chosen before targets), read by a
    /// [`PermanentFilter::mv_eq_x`] target filter; 0 for a non-`{X}` cast.
    pub(crate) fn targets_are_legal(
        &self,
        object: ObjectId,
        def: CardDef,
        target: Option<Target>,
        controller: PlayerId,
        mode: Option<usize>,
        x: u32,
    ) -> bool {
        let spec = self.required_target(def, mode);
        match target {
            None => spec == TargetSpec::None,
            Some(t) => self
                .legal_targets_for(spec, object, controller, color_identity(def), x)
                .contains(&t),
        }
    }

    /// The target a card requires when cast. For a non-modal card this is its first spell-timed
    /// effect that needs one; for a modal spell it's the *chosen* mode's effect (CR 601.2c) — so
    /// a creature-targeting mode requires a creature and a non-targeting mode requires none. A
    /// mode-less query on a modal card (snapshot / auto-pass, which don't know the pick) reports
    /// no requirement.
    pub(crate) fn required_target(&self, def: CardDef, mode: Option<usize>) -> TargetSpec {
        // An Aura is cast targeting the creature it will enchant (CR 303.4a), even though its
        // grant is a static ability, not a spell effect. An "Enchant creature you control"-style
        // restriction narrows that to `def.enchant`'s filter; an unrestricted "Enchant creature"
        // falls back to any creature.
        if matches!(def.kind, CardKind::Aura) {
            return TargetSpec::Permanent(
                def.enchant
                    .unwrap_or(PermanentFilter::of(TypeSet::CREATURE)),
            );
        }
        // Animate Dead (CR 303.4a's "enchant creature card in a graveyard"): the pool's one Aura
        // whose enchant subject is a graveyard card, not a battlefield permanent — kind stays
        // `Enchantment` (see `CardDef::enchant_graveyard`'s doc), so it needs its own cast-target
        // branch alongside the `CardKind::Aura` one above.
        if def.enchant_graveyard {
            return TargetSpec::CreatureCardInAnyGraveyard;
        }
        if def.modal {
            return mode
                .and_then(|m| nth_mode(def, m))
                .map_or(TargetSpec::None, |a| a.effect.target());
        }
        for ability in def.abilities {
            if matches!(ability.timing, Timing::Spell)
                && ability.effect.target() != TargetSpec::None
            {
                return ability.effect.target();
            }
        }
        TargetSpec::None
    }

    /// Whether `object` is a creature currently on the battlefield. A phased-out creature (CR
    /// 702.26e) doesn't count — it's not a legal target and not a combat participant.
    pub(crate) fn is_creature_on_battlefield(&self, object: ObjectId) -> bool {
        let Some(p) = self.as_permanent(object) else {
            return false;
        };
        // CR 613.4 type layer, not the printed kind: a manland animated into a creature (Restless
        // Spire) counts, via `effective_types`.
        !p.phased_out && self.effective_types(object).intersects(TypeSet::CREATURE)
    }

    /// Whether `object` is an enchantment currently on the battlefield (CR 303 — includes an
    /// Aura, CR 303.2). A phased-out permanent doesn't count, mirroring
    /// [`Self::is_creature_on_battlefield`]. Used by Copy Enchantment's `enter_as_copy` (`of =
    /// "enchantment"`, CR 706/707.2) to enumerate its copyable candidates.
    pub(crate) fn is_enchantment_on_battlefield(&self, object: ObjectId) -> bool {
        let Some(p) = self.as_permanent(object) else {
            return false;
        };
        !p.phased_out
            && self
                .effective_types(object)
                .intersects(TypeSet::ENCHANTMENT)
    }

    /// The mana `player` could produce right now: their pool plus free taps, then a fixed-point
    /// over paid tap-for-mana abilities (filter lands, karoos, signets) — each unused permanent's
    /// paid ability is included only when the running estimate can pay its activation cost, via
    /// spend-then-merge (never gross-only). Used by [`has_meaningful_action`] so an untapped
    /// board counts as castable mana.
    /// A painland's two free modes are both summed, over-counting a single land's output, but
    /// over-counting only makes auto-pass stop *more* often (never wrongly skip), the safe
    /// direction (ADR 0007). (CR 605, CR 108.3, CR 113)
    pub(crate) fn available_mana(&self, player: PlayerId) -> ManaPool {
        let mut mana = self.players[player.0 as usize].mana_pool;
        let mut used = vec![false; self.objects.len()];
        let mut paid: Vec<(ObjectId, Cost, ManaPool)> = Vec::new();

        for (idx, o) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = o else {
                continue;
            };
            if p.owner != player || p.tapped {
                continue;
            }
            let id = idx as ObjectId;
            // Permanents with a paid tap-for-mana ability (Fetid Heath filter, Study Hall any)
            // are counted only via the fixed-point below — adding their free mode here would
            // mark them used and hide the paid mode when duals are required.
            let has_paid_mana = p.def.abilities.iter().any(|a| {
                let Timing::Activated(cost) = a.timing else {
                    return false;
                };
                let Effect::AddMana { single_color, .. } = a.effect else {
                    return false;
                };
                cost.taps_self
                    && cost.mana != Cost::FREE
                    && cost.pay_life == Amount::Fixed(0)
                    && matches!(cost.sacrifice, SacrificeCost::None)
                    && !single_color
            });

            let mut contributed_free = false;
            if !has_paid_mana
                && let CardKind::Land {
                    produces: Some(produces),
                    ..
                } = p.def.kind
            {
                let credit = match produces {
                    LandProduces::Mana(m) => Some(m),
                    LandProduces::CommanderIdentity => self.commander_identity_credit(player),
                    LandProduces::OpponentColors => self.opponent_producible_colors_credit(player),
                };
                if let Some(credit) = credit {
                    mana.add(credit, 1);
                    contributed_free = true;
                }
            }
            for (i, a) in p.def.abilities.iter().enumerate() {
                let Timing::Activated(cost) = a.timing else {
                    continue;
                };
                let Effect::AddMana {
                    mana: batch,
                    identity,
                    opponent_colors,
                    restriction,
                    single_color,
                    ..
                } = a.effect
                else {
                    continue;
                };
                if !cost.taps_self
                    || cost.pay_life != Amount::Fixed(0)
                    || !matches!(cost.sacrifice, SacrificeCost::None)
                    || single_color
                {
                    continue;
                }
                if cost.mana == Cost::FREE {
                    if has_paid_mana {
                        continue;
                    }
                    mana.merge(&batch.restricted_by(restriction));
                    if identity > 0
                        && let Some(credit) = self.commander_identity_credit(player)
                    {
                        mana.add(credit, 1);
                    }
                    if opponent_colors > 0
                        && let Some(credit) = self.opponent_producible_colors_credit(player)
                    {
                        mana.add(credit, 1);
                    }
                    contributed_free = true;
                    continue;
                }
                if self.ability_activation_gate(player, id, i).is_err() {
                    continue;
                }
                let mut credit = batch.restricted_by(restriction);
                if identity > 0
                    && let Some(m) = self.commander_identity_credit(player)
                {
                    credit.add(m, 1);
                }
                if opponent_colors > 0
                    && let Some(m) = self.opponent_producible_colors_credit(player)
                {
                    credit.add(m, 1);
                }
                // Net-zero paid modes (Study Hall's {{1}},{{T}}: any) are color conversion — skip
                // them here so the free {{C}} still counts; filter/karoo/signet net-positive taps stay.
                let activation_pips = cost.mana.generic as u32
                    + cost.mana.colorless as u32
                    + cost.mana.colored.iter().map(|&n| n as u32).sum::<u32>()
                    + cost.mana.hybrid.len() as u32;
                if credit.total() <= activation_pips {
                    continue;
                }
                paid.push((id, cost.mana, credit));
            }
            if !has_paid_mana {
                for (cost, batch) in self.granted_mana_abilities(id) {
                    if cost.taps_self && cost.mana == Cost::FREE {
                        mana.merge(&batch);
                        contributed_free = true;
                    }
                }
            }
            if contributed_free {
                used[idx] = true;
            }
        }

        let mut progress = true;
        while progress {
            progress = false;
            for (id, activation, credit) in &paid {
                let idx = *id as usize;
                if used[idx] || !mana.can_pay(activation, None) {
                    continue;
                }
                let Some(spend) = mana.spend_plan(activation, None) else {
                    continue;
                };
                let mut after = mana;
                after.subtract(&spend);
                after.merge(credit);
                // Only take a paid outlet when it does not drop coverage of any color the
                // pre-activation pool could pay (Ferrous must not burn a lone {{W}} into {{U}}{{R}}).
                let preserves = (0..Color::COUNT).all(|c| {
                    let before_cov = mana.colored[c]
                        + mana.any
                        + COLOR_PAIRS
                            .iter()
                            .zip(mana.either.iter())
                            .filter(|((a, b), _)| a.index() == c || b.index() == c)
                            .map(|(_, &n)| n)
                            .sum::<u8>();
                    let after_cov = after.colored[c]
                        + after.any
                        + COLOR_PAIRS
                            .iter()
                            .zip(after.either.iter())
                            .filter(|((a, b), _)| a.index() == c || b.index() == c)
                            .map(|(_, &n)| n)
                            .sum::<u8>();
                    after_cov >= before_cov || before_cov == 0
                });
                // Net-positive or color-preserving conversion with more total mana.
                if !preserves || after.total() < mana.total() {
                    continue;
                }
                if after.total() == mana.total() && after == mana {
                    continue;
                }
                mana = after;
                used[idx] = true;
                progress = true;
            }
        }

        // Free modes on paid-capable permanents (Fetid Heath's {{C}}) still count when the
        // paid mode was not used — otherwise Plains+Swamp+Heath undercounts generic.
        for (idx, o) in self.objects.iter().enumerate() {
            if used[idx] {
                continue;
            }
            let Object::Permanent(p) = o else {
                continue;
            };
            if p.owner != player || p.tapped {
                continue;
            }
            if let CardKind::Land {
                produces: Some(produces),
                ..
            } = p.def.kind
            {
                let credit = match produces {
                    LandProduces::Mana(m) => Some(m),
                    LandProduces::CommanderIdentity => self.commander_identity_credit(player),
                    LandProduces::OpponentColors => self.opponent_producible_colors_credit(player),
                };
                if let Some(credit) = credit {
                    mana.add(credit, 1);
                    used[idx] = true;
                    continue;
                }
            }
            for a in p.def.abilities {
                let Timing::Activated(cost) = a.timing else {
                    continue;
                };
                let Effect::AddMana {
                    mana: batch,
                    identity,
                    opponent_colors,
                    restriction,
                    single_color,
                    ..
                } = a.effect
                else {
                    continue;
                };
                if !cost.taps_self
                    || cost.mana != Cost::FREE
                    || cost.pay_life != Amount::Fixed(0)
                    || !matches!(cost.sacrifice, SacrificeCost::None)
                    || single_color
                {
                    continue;
                }
                mana.merge(&batch.restricted_by(restriction));
                if identity > 0
                    && let Some(credit) = self.commander_identity_credit(player)
                {
                    mana.add(credit, 1);
                }
                if opponent_colors > 0
                    && let Some(credit) = self.opponent_producible_colors_credit(player)
                {
                    mana.add(credit, 1);
                }
                used[idx] = true;
            }
        }
        mana
    }

    /// Whether `cost` can be paid from `available` mana — `spell` is the spell being cast
    /// (`None` for an ability activation), read by [`ManaPool::spend_plan`] against any
    /// spend-restricted credit in `available`.
    pub(crate) fn affordable_from(
        available: ManaPool,
        cost: Cost,
        spell: Option<SpellCharacteristics>,
    ) -> bool {
        available.can_pay(&cost, spell)
    }

    /// Plan how to pay `cost` from `player`'s pool. Returns the exact multiset to remove, or
    /// `None` if the pool can't cover it. Pure — the caller applies the [`Event::ManaSpent`].
    pub(crate) fn plan_payment(
        &self,
        player: PlayerId,
        cost: Cost,
        spell: Option<SpellCharacteristics>,
    ) -> Option<ManaPool> {
        self.players[player.0 as usize]
            .mana_pool
            .spend_plan(&cost, spell)
    }

    /// Plan which untapped mana sources to tap so `player` can pay `cost`: empty when the pool
    /// alone covers it, `None` when even tapping everything can't (nothing is ever tapped for a
    /// cost that won't be met). Free taps first (lands over non-lands, painless, narrow), then
    /// paid tap-for-mana abilities (filter lands, karoos, signets) with a **feed-first** free
    /// subplan so nested [`Game::settle_payment`] inside activation only spends. Pure —
    /// [`Game::settle_payment`] executes the plan.
    /// ponytail: greedy (most-constrained unmet pip first, lands first, painless first, narrowest
    /// first) with the loop gated on an exact `can_pay` — it can miss a payable plan over lands
    /// with overlapping dual pairs (then it rejects without tapping, never mis-taps); make it
    /// exhaustive if a mixed-pair manabase ever needs it.
    pub(crate) fn plan_auto_taps(
        &self,
        player: PlayerId,
        cost: Cost,
        exclude: Option<ObjectId>,
        spell: Option<SpellCharacteristics>,
    ) -> Option<Vec<PlannedTap>> {
        let mut pool = self.players[player.0 as usize].mana_pool;
        if pool.can_pay(&cost, spell) {
            return Some(Vec::new());
        }

        let (mut free, mut paid) = self.auto_tap_candidates(player, exclude);
        let mut taps = Vec::new();

        while !pool.can_pay(&cost, spell) {
            if let Some(i) =
                Self::pick_free_tap(&free, &pool, &cost, /*completing_only*/ false)
            {
                let chosen = free.swap_remove(i);
                Self::commit_free_tap(&mut pool, &mut free, &mut paid, &mut taps, chosen);
                continue;
            }

            // A free tap that alone completes payment (e.g. Fetid Heath's {C} for leftover generic)
            // before we spend a permanent on its paid filter mode.
            if let Some(i) = Self::pick_free_tap(&free, &pool, &cost, /*completing_only*/ true) {
                let mut trial = pool;
                trial.merge(&free[i].credit);
                if trial.can_pay(&cost, spell) {
                    let chosen = free.swap_remove(i);
                    Self::commit_free_tap(&mut pool, &mut free, &mut paid, &mut taps, chosen);
                    continue;
                }
            }

            // Generic free filler before paid taps — otherwise a filter/signet can burn a colored
            // pip still required by the spell (Plains+Mountain+Island+Ferrous for {{2}}{{W}}).
            // Skip a free mode whose permanent still has a paid sibling when a colored shortfall
            // remains that this free credit cannot serve (keep Fetid Heath free for filter use).
            let unmet_color =
                (0..Color::COUNT).any(|c| !Self::pool_covers_color(&pool, c, cost.colored[c]));
            if let Some(i) = free
                .iter()
                .enumerate()
                .filter(|(_, k)| {
                    let source = match k.tap {
                        PlannedTap::Base(l) | PlannedTap::Ability(l, _) => l,
                    };
                    let has_paid_sibling = paid.iter().any(|p| p.source == source);
                    if has_paid_sibling && unmet_color {
                        // Only take this free credit if it itself serves an unmet color.
                        return (0..Color::COUNT).any(|c| {
                            !Self::pool_covers_color(&pool, c, cost.colored[c])
                                && mana_serves(&k.credit, c)
                        });
                    }
                    true
                })
                .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                .map(|(i, _)| i)
            {
                let chosen = free.swap_remove(i);
                Self::commit_free_tap(&mut pool, &mut free, &mut paid, &mut taps, chosen);
                continue;
            }

            if let Some((feed, paid_i)) = Self::pick_paid_tap(&free, &paid, &pool, &cost, spell) {
                for f in feed {
                    let pos = free.iter().position(|c| c.tap == f)?;
                    let chosen = free.swap_remove(pos);
                    Self::commit_free_tap(&mut pool, &mut free, &mut paid, &mut taps, chosen);
                }
                let chosen = paid.swap_remove(paid_i);
                let spend = pool.spend_plan(&chosen.activation, None)?;
                pool.subtract(&spend);
                pool.merge(&chosen.credit);
                free.retain(|k| match k.tap {
                    PlannedTap::Base(l) | PlannedTap::Ability(l, _) => l != chosen.source,
                });
                paid.retain(|k| k.source != chosen.source);
                taps.push(chosen.tap);
                continue;
            }

            return None;
        }
        Some(taps)
    }

    /// Whether `pool` can cover `need` pips of `color` from mono, either, any, or of_colors.
    fn pool_covers_color(pool: &ManaPool, color: usize, need: u8) -> bool {
        if need == 0 {
            return true;
        }
        let mut have = pool.colored[color];
        have = have.saturating_add(pool.any);
        for (&(a, b), &n) in COLOR_PAIRS.iter().zip(pool.either.iter()) {
            if a.index() == color || b.index() == color {
                have = have.saturating_add(n);
            }
        }
        for (mask, &n) in pool.of_colors.iter().enumerate() {
            if n > 0 && (mask & (1 << color)) != 0 {
                have = have.saturating_add(n);
            }
        }
        have >= need
    }

    fn commit_free_tap(
        pool: &mut ManaPool,
        free: &mut Vec<FreeTapCandidate>,
        paid: &mut Vec<PaidTapCandidate>,
        taps: &mut Vec<PlannedTap>,
        chosen: FreeTapCandidate,
    ) {
        pool.merge(&chosen.credit);
        let source = match chosen.tap {
            PlannedTap::Base(l) | PlannedTap::Ability(l, _) => l,
        };
        free.retain(|k| match k.tap {
            PlannedTap::Base(l) | PlannedTap::Ability(l, _) => l != source,
        });
        paid.retain(|k| k.source != source);
        taps.push(chosen.tap);
    }

    /// Pick a free candidate: scarce colored pip, else colorless shortfall. When
    /// `completing_only`, only consider a candidate whose credit alone would make `cost` payable
    /// from `pool` (lookahead for preferring free {C} over a filter mode).
    fn pick_free_tap(
        free: &[FreeTapCandidate],
        pool: &ManaPool,
        cost: &Cost,
        completing_only: bool,
    ) -> Option<usize> {
        let best = |pred: &dyn Fn(&FreeTapCandidate) -> bool| {
            free.iter()
                .enumerate()
                .filter(|(_, k)| {
                    if !pred(k) {
                        return false;
                    }
                    if !completing_only {
                        return true;
                    }
                    let mut trial = *pool;
                    trial.merge(&k.credit);
                    trial.can_pay(cost, None)
                })
                .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                .map(|(i, _)| i)
        };
        let scarcest = (0..Color::COUNT)
            .filter(|&c| cost.colored[c] > pool.colored[c])
            .filter(|&c| free.iter().any(|k| mana_serves(&k.credit, c)))
            .min_by_key(|&c| free.iter().filter(|k| mana_serves(&k.credit, c)).count());
        if let Some(c) = scarcest {
            return best(&|k: &FreeTapCandidate| mana_serves(&k.credit, c));
        }
        if cost.colorless > pool.colorless && free.iter().any(|k| k.credit.colorless > 0) {
            return best(&|k: &FreeTapCandidate| k.credit.colorless > 0);
        }
        if completing_only {
            return best(&|_| true);
        }
        None
    }

    /// Choose a paid ability plus the free feed taps needed so the activation cost is covered
    /// before the ability runs. Only accepted when the post-activation pool completes the outer
    /// cost, or strictly covers an unmet colored pip without dropping a previously covered one.
    fn pick_paid_tap(
        free: &[FreeTapCandidate],
        paid: &[PaidTapCandidate],
        pool: &ManaPool,
        cost: &Cost,
        spell: Option<SpellCharacteristics>,
    ) -> Option<(Vec<PlannedTap>, usize)> {
        let mut best: Option<(Vec<PlannedTap>, usize, bool, usize, bool, u8)> = None;
        for (pi, p) in paid.iter().enumerate() {
            let Some((feed, after)) = Self::simulate_paid_activation(free, pool, p, p.source)
            else {
                continue;
            };
            let completes = after.can_pay(cost, spell);
            let preserves_colors = (0..Color::COUNT).all(|c| {
                Self::pool_covers_color(&after, c, cost.colored[c])
                    || !Self::pool_covers_color(pool, c, cost.colored[c])
            });
            let helps_color = preserves_colors
                && (0..Color::COUNT).any(|c| {
                    !Self::pool_covers_color(pool, c, cost.colored[c])
                        && Self::pool_covers_color(&after, c, cost.colored[c])
                });
            if !completes && !helps_color {
                continue;
            }
            let feed_len = feed.len();
            let candidate = (feed, pi, completes, feed_len, p.nonland, p.breadth);
            let take = match &best {
                None => true,
                Some((_, _, best_done, best_feed, best_nl, best_br)) => {
                    match (completes, *best_done) {
                        (true, false) => true,
                        (false, true) => false,
                        _ => (feed_len, p.nonland, p.breadth) < (*best_feed, *best_nl, *best_br),
                    }
                }
            };
            if take {
                best = Some(candidate);
            }
        }
        best.map(|(feed, pi, _, _, _, _)| (feed, pi))
    }

    fn simulate_paid_activation(
        free: &[FreeTapCandidate],
        pool: &ManaPool,
        paid: &PaidTapCandidate,
        exclude_source: ObjectId,
    ) -> Option<(Vec<PlannedTap>, ManaPool)> {
        let mut sim = *pool;
        let mut feed = Vec::new();
        let mut remaining: Vec<&FreeTapCandidate> = free
            .iter()
            .filter(|k| match k.tap {
                PlannedTap::Base(l) | PlannedTap::Ability(l, _) => l != exclude_source,
            })
            .collect();

        while !sim.can_pay(&paid.activation, None) {
            let scarcest = (0..Color::COUNT)
                .filter(|&c| paid.activation.colored[c] > sim.colored[c])
                .filter(|&c| remaining.iter().any(|k| mana_serves(&k.credit, c)))
                .min_by_key(|&c| {
                    remaining
                        .iter()
                        .filter(|k| mana_serves(&k.credit, c))
                        .count()
                });
            let pick = if let Some(c) = scarcest {
                remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| mana_serves(&k.credit, c))
                    .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                    .map(|(i, _)| i)
            } else if !paid.activation.hybrid.is_empty()
                && remaining.iter().any(|k| {
                    paid.activation.hybrid.iter().any(|&(a, b)| {
                        mana_serves(&k.credit, a.index()) || mana_serves(&k.credit, b.index())
                    })
                })
            {
                remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| {
                        paid.activation.hybrid.iter().any(|&(a, b)| {
                            mana_serves(&k.credit, a.index()) || mana_serves(&k.credit, b.index())
                        })
                    })
                    .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                    .map(|(i, _)| i)
            } else if paid.activation.colorless > sim.colorless
                && remaining.iter().any(|k| k.credit.colorless > 0)
            {
                remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| k.credit.colorless > 0)
                    .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                    .map(|(i, _)| i)
            } else {
                remaining
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, k)| (k.nonland, k.pain, k.breadth))
                    .map(|(i, _)| i)
            };
            let i = pick?;
            let chosen = remaining.swap_remove(i);
            sim.merge(&chosen.credit);
            feed.push(chosen.tap);
        }
        let spend = sim.spend_plan(&paid.activation, None)?;
        sim.subtract(&spend);
        sim.merge(&paid.credit);
        Some((feed, sim))
    }

    fn auto_tap_candidates(
        &self,
        player: PlayerId,
        exclude: Option<ObjectId>,
    ) -> (Vec<FreeTapCandidate>, Vec<PaidTapCandidate>) {
        let mut free = Vec::new();
        let mut paid = Vec::new();
        for (id, o) in self.objects.iter().enumerate() {
            let id = id as ObjectId;
            let Object::Permanent(p) = o else {
                continue;
            };
            if p.owner != player || p.tapped || Some(id) == exclude {
                continue;
            }
            let nonland = !matches!(p.def.kind, CardKind::Land { .. });
            if let CardKind::Land { produces, .. } = p.def.kind {
                let base_credit = match produces {
                    Some(LandProduces::Mana(m)) => Some(m),
                    Some(LandProduces::CommanderIdentity) => self.commander_identity_credit(player),
                    Some(LandProduces::OpponentColors) => {
                        self.opponent_producible_colors_credit(player)
                    }
                    None => None,
                };
                if let Some(m) = base_credit {
                    let mut credit = ManaPool::default();
                    credit.add(m, 1);
                    free.push(FreeTapCandidate {
                        tap: PlannedTap::Base(id),
                        breadth: mana_breadth(&credit),
                        credit,
                        nonland: false,
                        pain: false,
                    });
                }
            }
            for (i, a) in p.def.abilities.iter().enumerate() {
                let Timing::Activated(acost) = a.timing else {
                    continue;
                };
                let Effect::AddMana {
                    mana: batch,
                    identity,
                    opponent_colors,
                    single_color,
                    restriction,
                    ..
                } = a.effect
                else {
                    continue;
                };
                if !acost.taps_self
                    || acost.pay_life != Amount::Fixed(0)
                    || !matches!(acost.sacrifice, SacrificeCost::None)
                    || single_color
                    || self.ability_activation_gate(player, id, i).is_err()
                {
                    continue;
                }
                let mut credit = batch.restricted_by(restriction);
                if identity > 0
                    && let Some(m) = self.commander_identity_credit(player)
                {
                    credit.add(m, 1);
                }
                if opponent_colors > 0
                    && let Some(m) = self.opponent_producible_colors_credit(player)
                {
                    credit.add(m, 1);
                }
                if acost.mana == Cost::FREE {
                    free.push(FreeTapCandidate {
                        tap: PlannedTap::Ability(id, i),
                        breadth: mana_breadth(&credit),
                        credit,
                        nonland,
                        pain: acost.self_damage > 0,
                    });
                } else {
                    let activation_pips = acost.mana.generic as u32
                        + acost.mana.colorless as u32
                        + acost.mana.colored.iter().map(|&n| n as u32).sum::<u32>()
                        + acost.mana.hybrid.len() as u32;
                    // Net-zero converters (Study Hall {{1}},{{T}}: any) stay manual on the radial.
                    if credit.total() <= activation_pips {
                        continue;
                    }
                    paid.push(PaidTapCandidate {
                        tap: PlannedTap::Ability(id, i),
                        source: id,
                        activation: acost.mana,
                        breadth: mana_breadth(&credit),
                        credit,
                        nonland,
                    });
                }
            }
            let own_len = p.def.abilities.len();
            for (gi, (acost, batch)) in self.granted_mana_abilities(id).into_iter().enumerate() {
                let index = own_len + gi;
                if !acost.taps_self
                    || acost.mana != Cost::FREE
                    || acost.pay_life != Amount::Fixed(0)
                    || !matches!(acost.sacrifice, SacrificeCost::None)
                    || self.ability_activation_gate(player, id, index).is_err()
                {
                    continue;
                }
                free.push(FreeTapCandidate {
                    tap: PlannedTap::Ability(id, index),
                    breadth: mana_breadth(&batch),
                    credit: batch,
                    nonland,
                    pain: acost.self_damage > 0,
                });
            }
        }
        (free, paid)
    }

    /// Object ids [`Game::plan_auto_taps`] would tap to pay `action`'s mana (empty when the pool
    /// covers it, the action has no mana cost, or the plan is somehow unaffordable). Same planner
    /// settle uses — preview must match payment.
    ///
    /// Delve casts try `delve = max…0` (matching list affordability) so a listed Cruise still
    /// gets a preview; the chosen count is the largest that yields a payable plan (fewest taps).
    pub fn auto_tap_objects(&self, action: &LegalAction) -> Vec<ObjectId> {
        let player = action.player;
        let object_ids = |plan: Vec<PlannedTap>| {
            plan.into_iter()
                .map(|tap| match tap {
                    PlannedTap::Base(id) | PlannedTap::Ability(id, _) => id,
                })
                .collect()
        };
        let (cost, exclude, spell) = match action.kind {
            MeaningfulAction::PlayLand { .. }
            | MeaningfulAction::DeclareAttackers
            | MeaningfulAction::DeclareBlockers => return Vec::new(),
            MeaningfulAction::Cast { card, zone } => {
                let def = self.def_of(card);
                let max_delve = if def.delve {
                    self.graveyard_of(player).len().min(u8::MAX as usize) as u8
                } else {
                    0
                };
                for delve in (0..=max_delve).rev() {
                    let cost = self.cast_cost(
                        player, card, def, None, 0, zone, delve, false, false, false, 0, 0,
                    );
                    if let Some(plan) =
                        self.plan_auto_taps(player, cost, None, Some(def.spell_characteristics()))
                    {
                        return object_ids(plan);
                    }
                }
                return Vec::new();
            }
            MeaningfulAction::CastPrepared { source } => {
                let Some(back) = self.def_of(source).back else {
                    return Vec::new();
                };
                let back = *back;
                (
                    self.cast_cost(
                        player,
                        source,
                        back,
                        None,
                        0,
                        Zone::Battlefield,
                        0,
                        false,
                        false,
                        false,
                        0,
                        0,
                    ),
                    None,
                    Some(back.spell_characteristics()),
                )
            }
            MeaningfulAction::Cycle { card } => {
                let Some(cost) = self.def_of(card).cycling else {
                    return Vec::new();
                };
                (cost, None, None)
            }
            MeaningfulAction::ActivateHandAbility { card } => {
                let Some(ability) = self.def_of(card).hand_ability else {
                    return Vec::new();
                };
                (ability.cost, None, None)
            }
            MeaningfulAction::Suspend { card } => {
                let Some(suspend) = self.def_of(card).suspend else {
                    return Vec::new();
                };
                (*suspend.cost, None, None)
            }
            MeaningfulAction::Encore { card } => {
                let Some(cost) = self.def_of(card).encore else {
                    return Vec::new();
                };
                (*cost, None, None)
            }
            // Turning face up pays a morph card's morph cost (CR 702.37c), else a manifest's
            // hidden printed cost (CR 701.34e) — the same fork as `Game::turn_face_up`.
            MeaningfulAction::TurnFaceUp { permanent } => {
                let def = self.def_of(permanent);
                (def.morph.unwrap_or(def.cost), None, None)
            }
            // A face-down morph cast pays a flat generic {3} (CR 702.37b).
            MeaningfulAction::CastFaceDown { .. } => (
                Cost {
                    generic: 3,
                    ..Cost::FREE
                },
                None,
                None,
            ),
            MeaningfulAction::Activate { source, ability } => {
                let Ok((_, cost)) = self.ability_activation_gate(player, source, ability) else {
                    return Vec::new();
                };
                (cost.mana, cost.taps_self.then_some(source), None)
            }
        };
        self.plan_auto_taps(player, cost, exclude, spell)
            .map(object_ids)
            .unwrap_or_default()
    }

    /// Pay `cost` for `player` — from the pool, auto-tapping mana sources for any shortfall
    /// (free taps first, then paid tap-for-mana abilities via a feed-first plan) — appending the
    /// tap events and the [`Event::ManaSpent`]. Call only after the action is otherwise fully
    /// validated: an unpayable cost rejects with *nothing* tapped, and a successful plan applies
    /// whole. `exclude` keeps an ability's own source out of the plan (it's already being tapped
    /// as the activation cost). `spell` is the spell `cost` is casting (`None` for an ability
    /// activation — see [`ManaPool::spend_plan`]).
    pub(crate) fn settle_payment(
        &mut self,
        player: PlayerId,
        cost: Cost,
        exclude: Option<ObjectId>,
        spell: Option<SpellCharacteristics>,
        events: &mut Vec<Event>,
    ) -> Result<(), Reject> {
        let plan = self
            .plan_auto_taps(player, cost, exclude, spell)
            .ok_or(Reject::CannotPayCost)?;
        for tap in plan {
            let produced = match tap {
                PlannedTap::Base(source) => self.tap_for_mana(player, source)?,
                PlannedTap::Ability(source, index) => {
                    // A mana-payment plan only taps mana abilities, none of which carry `{X}`.
                    self.activate_ability(player, source, index, None, Vec::new(), 0)?
                }
            };
            events.extend(produced);
        }
        let spend = self
            .plan_payment(player, cost, spell)
            .ok_or(Reject::CannotPayCost)?; // unreachable: the plan's pool math matches
        self.push_apply(
            events,
            Event::ManaSpent {
                player,
                mana: spend,
            },
        );
        Ok(())
    }

    /// Whether the *next* pass of priority would complete the round and resolve the top of the
    /// stack (CR 608.1: all players passing in succession with a non-empty stack). The server
    /// reads this to pause before submitting that final auto-pass, so an uncontested spell
    /// visibly sits on the stack instead of resolving in the same broadcast frame.
    pub fn next_pass_resolves_stack(&self) -> bool {
        !self.stack.is_empty() && self.consecutive_passes + 1 >= self.living_player_count()
    }

    pub(crate) fn pass_priority(&mut self, player: PlayerId) -> Result<Vec<Event>, Reject> {
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }

        let mut events = vec![Event::PriorityPassed { player }];
        self.apply_all(&events);
        self.consecutive_passes += 1;
        self.priority = self.next_player(player);

        // When every living player passes in succession, either the top of the stack
        // resolves or — if the stack is empty — the current step ends. (SBAs are swept
        // by `submit`.) Eliminated seats never hold priority, so they don't count. (CR 117)
        if self.consecutive_passes >= self.living_player_count() {
            crate::pipeline::PostIntentPipeline::complete_priority_round(self, &mut events);
        }
        Ok(events)
    }

    /// Apply `event`, recording it into `events`. Used where turn-based actions must
    /// see the effect of the previous event (e.g. untap reads the just-entered step).
    pub(crate) fn push_apply(&mut self, events: &mut Vec<Event>, event: Event) {
        self.apply(&event);
        events.push(event);
    }

    /// End the current step and roll forward, performing each new step's turn-based
    /// actions, until reaching a step that grants priority. Steps without a priority
    /// window (Untap, Cleanup) are processed and passed straight through.
    pub(crate) fn advance_step(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        loop {
            let leaving_cleanup = self.step == Step::Cleanup;
            let next = self.step.next();

            // Skip the first-strike combat damage step unless a first/double striker is in (CR 510, CR 120.3, CR 506)
            // combat (CR 510.5): advance the step marker without a StepBegan or a priority
            // window, so with no first strikers there's exactly one combat damage step. (CR 510, CR 120.3, CR 506)
            if next == Step::FirstStrikeCombatDamage && !self.any_first_strike_in_combat() {
                self.step = next;
                continue;
            }

            let next_active = if leaving_cleanup {
                self.next_player(self.active_player)
            } else {
                self.active_player
            };

            // A step or phase ending empties every player's mana pool (rule 500.4), except
            // "until end of turn" persistent mana, which survives until the turn actually ends.
            for i in 0..self.players.len() as u8 {
                self.push_apply(
                    &mut events,
                    Event::ManaEmptied {
                        player: PlayerId(i),
                        end_of_turn: leaving_cleanup,
                    },
                );
            }
            self.push_apply(
                &mut events,
                Event::StepBegan {
                    step: next,
                    active_player: next_active,
                },
            );
            self.perform_turn_based_actions(next, next_active, &mut events);

            // A turn-based action may raise a choice (cleanup's discard-to-hand-size). Stop the
            // step loop and hand back to the caller; answering the choice resumes it (via
            // `answer_discard` → `advance_step`), so it isn't silently skipped.
            if self.pending_choice.is_some() {
                return events;
            }

            if next.has_priority_window() {
                self.priority = next_active;
                self.consecutive_passes = 0;
                return events;
            }
        }
    }

    /// The automatic actions performed as a step begins (untap, draw, cleanup).
    pub(crate) fn perform_turn_based_actions(
        &mut self,
        step: Step,
        active: PlayerId,
        events: &mut Vec<Event>,
    ) {
        match step {
            Step::Untap => {
                // Goad ends "until your next turn" (CR 701.38b): the active player's turn
                // beginning clears every goad they applied. (CR 701.38)
                if self
                    .combat_extras
                    .goaded
                    .iter()
                    .any(|&(_, by, _)| by == active)
                {
                    self.push_apply(events, Event::GoadCleared { by: active });
                }
                // An extended impulse-draw permission (Atsushi's `until_next_turn`) arms the same
                // way: the shield only lifts once its own controller's next turn begins.
                let to_arm: Vec<ObjectId> = self
                    .play_permissions
                    .play_from_exile
                    .iter()
                    .filter(|&&(_, player, extended)| player == active && extended)
                    .map(|&(card, _, _)| card)
                    .collect();
                for card in to_arm {
                    self.push_apply(events, Event::PlayFromExilePermissionArmed { card });
                }
                // Phase in the active player's phased-out permanents (CR 702.26f) — as a turn-based
                // action at the start of the untap step, *before* untapping. Emit one `PhasedIn`
                // per directly-phased permanent (`attached_to.is_none()`); its handler cascades to
                // its indirectly-phased attachments, which phase in together (CR 702.26g).
                // ponytail: keyed on the phased permanent's live controller (`controller_of`),
                // which stands in for CR's "its controller's next turn"; a phased permanent whose
                // control changed while phased is an unmodeled edge no pool card reaches.
                let to_phase_in: Vec<ObjectId> = self
                    .permanent_ids(|p| p.phased_out && p.attached_to.is_none())
                    .filter(|&id| self.controller_of(id) == active)
                    .collect();
                for id in to_phase_in {
                    self.push_apply(events, Event::PhasedIn { object: id });
                }
                // "You may choose not to untap this" (CR 502.2 — Rubinia Soulsinger): a tapped
                // permanent carrying the flag isn't untapped here; instead it's offered below in a
                // yes/no pause, and only untapped once the active player declines to keep it tapped.
                let mut optional_untap: Vec<ObjectId> = Vec::new();
                for id in self.controlled_battlefield(active) {
                    if self.permanent(id).tapped {
                        if self.def_of(id).may_choose_not_to_untap {
                            optional_untap.push(id);
                        } else {
                            self.push_apply(events, Event::Untapped { object: id });
                        }
                    }
                    if self.permanent(id).summoning_sick {
                        self.push_apply(events, Event::LostSummoningSickness { object: id });
                    }
                    // A new turn frees each planeswalker to activate a loyalty ability again (CR 606.3).
                    if self.permanent(id).loyalty_activated {
                        self.push_apply(
                            events,
                            Event::LoyaltyActivated {
                                object: id,
                                active: false,
                            },
                        );
                    }
                }
                // Pause on the optional-untap decision (CR 502.2). `advance_step` returns on this so
                // the step loop doesn't skip past it; `answer_decline_untap` untaps the ones the
                // player didn't keep tapped and resumes the loop.
                if !optional_untap.is_empty() {
                    crate::pending::raise_choice(
                        self,
                        PendingChoice::DeclineUntap {
                            player: active,
                            permanents: optional_untap,
                        },
                    );
                }
            }
            Step::Upkeep => {
                // Suspend (CR 702.62d): at the start of its owner's upkeep, remove one time
                // counter from each of that player's suspended cards. When the last is removed
                // the owner may cast it from exile without paying its mana cost (CR 702.62e) —
                // modeled by granting the #86 free-cast permission (which lasts until this turn's
                // cleanup, so the owner gets their main phases to cast it).
                // ponytail: real suspend casts the card via a *triggered* ability the instant the
                // last counter is removed, and the card gains haste (CR 702.62e/f). Modeled here
                // as a "may cast free from exile this turn" permission instead — for Rousing
                // Refrain (a sorcery with no haste-relevant body) the two are indistinguishable.
                let ticking: Vec<(ObjectId, u32)> = self
                    .exile_time_counters
                    .iter()
                    .filter(|&&(card, count)| count > 0 && self.owner_of(card) == active)
                    .copied()
                    .collect();
                for (card, count) in ticking {
                    self.push_apply(events, Event::TimeCountersRemoved { card });
                    if count == 1 {
                        self.push_apply(
                            events,
                            Event::CastFromExileFreePermissionGranted {
                                card,
                                player: active,
                            },
                        );
                    }
                }
            }
            Step::Draw => {
                // The starting player skips their first draw step in a two-player game (CR 103.8a);
                // in multiplayer no one skips (CR 103.8c). `begin_first_turn` arms the flag from the
                // seat count; spend it here so only that first draw is skipped.
                if std::mem::take(&mut self.skip_starting_players_first_draw) {
                    return;
                }
                let drawn = self.draw_card(active);
                events.extend(drawn);
            }
            // The two combat damage steps deal their own batch (CR 510.5). The between-steps
            // SBA sweep and death triggers are handled by `submit` after this step, and a (CR 704, CR 603, CR 104.3)
            // priority window opens between them. (CR 117)
            Step::FirstStrikeCombatDamage => self.combat_damage_substep(true, events),
            Step::CombatDamage => self.combat_damage_substep(false, events),
            Step::EndCombat => {
                // Clear combat if attackers were declared this turn (so the declared-flags reset,
                // even after a zero-attacker declaration). No attackers ⇒ nothing to clear.
                if self.combat.attackers_declared {
                    self.push_apply(events, Event::CombatCleared);
                }
            }
            Step::Cleanup => {
                // Remove all marked damage and until-end-of-turn boosts from every permanent.
                let damaged: Vec<ObjectId> = self
                    .permanent_ids(|p| p.marked_damage > 0 || p.deathtouched)
                    .collect();
                for id in damaged {
                    self.push_apply(events, Event::DamageCleared { object: id });
                }

                let boosted: Vec<ObjectId> = self
                    .permanent_ids(|p| {
                        p.temp_power != 0
                            || p.temp_toughness != 0
                            || p.base_pt_set_eot.is_some()
                            || p.added_types_eot != TypeSet::NONE
                            || !p.added_subtypes_eot.is_empty()
                            || !p.temp_keywords.is_empty()
                            || !p.temp_lost_keywords.is_empty()
                            || p.reverts_to_def_eot.is_some()
                    })
                    .collect();
                for id in boosted {
                    self.push_apply(events, Event::TempBoostsEnded { object: id });
                }

                // Regeneration shields last only "this turn" (CR 701.15b) — any unused one expires.
                let shielded: Vec<ObjectId> =
                    self.permanent_ids(|p| p.regeneration_shields > 0).collect();
                for id in shielded {
                    self.push_apply(events, Event::RegenerationShieldsExpired { object: id });
                }

                // A one-shot until-end-of-turn control change (CR 720) ends in the cleanup
                // step (CR 514.2); control reverts to the owner (or a still-attached
                // ControlAttached Aura).
                let stolen: Vec<ObjectId> = self
                    .play_permissions
                    .control_overrides
                    .iter()
                    .map(|&(id, ..)| id)
                    .collect();
                for id in stolen {
                    self.push_apply(events, Event::ControlEndedUntilEndOfTurn { object: id });
                }

                // Backup / "gains the following abilities until end of turn" (CR 702.166 / 514.2)
                // grants end here — the targets lose the granted abilities and keywords.
                if !self.abilities_granted_until_eot.is_empty() {
                    self.push_apply(events, Event::GrantedAbilitiesEnded);
                }

                // Impulse-draw permissions last only until end of turn (CR 118.6) — an `extended`
                // entry (Atsushi's `until_next_turn`, not yet armed) survives this cleanup.
                if self
                    .play_permissions
                    .play_from_exile
                    .iter()
                    .any(|&(_, _, extended)| !extended)
                {
                    self.push_apply(events, Event::PlayFromExileEnded);
                }

                // Quintorius's free-cast permission lasts only until end of turn (CR 118.5),
                // same "this turn" duration as impulse draw's plain (non-`extended`) entries
                // above — every entry here clears at once.
                if !self.play_permissions.cast_from_exile_free.is_empty() {
                    self.push_apply(events, Event::CastFromExileFreeEnded);
                }

                // A controlled "no maximum hand size" static (CR 402.2, e.g. Reliquary Tower)
                // lifts the limit entirely — that player never discards here.
                if self.has_no_max_hand_size(active) {
                    return;
                }
                // Discard down to the hand-size limit (CR 514.3): the player chooses which cards.
                let hand = self.hand_of(active);
                let over = hand.len().saturating_sub(HAND_SIZE);
                if over > 0 {
                    // Pause; `advance_step` returns on this so the step loop doesn't skip past
                    // the discard. `answer_discard` moves the chosen cards and resumes the loop.
                    crate::pending::raise_choice(
                        self,
                        PendingChoice::DiscardToHandSize {
                            player: active,
                            hand,
                            count: over,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    /// Ids of the permanents `player` controls on the battlefield.
    pub(crate) fn controlled_battlefield(&self, player: PlayerId) -> Vec<ObjectId> {
        // Controller, not owner — a permanent stolen by a control-changing Aura untaps, sheds
        // summoning sickness, and meets goad requirements under its new controller (CR 720).
        self.battlefield()
            .into_iter()
            .filter(|&id| self.controller_of(id) == player)
            .collect()
    }

    /// Ids of the live permanents whose state matches `pred`.
    pub(crate) fn permanent_ids<'a>(
        &'a self,
        pred: impl Fn(&Permanent) -> bool + 'a,
    ) -> impl Iterator<Item = ObjectId> + 'a {
        self.objects.iter().enumerate().filter_map(move |(id, o)| {
            matches!(o, Object::Permanent(p) if pred(p)).then_some(id as ObjectId)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    const P0: PlayerId = PlayerId(0);

    fn forest() -> CardDef {
        CardDef {
            name: "Forest",
            id: "",
            default_print: "",
            cost: Cost::FREE,
            kind: CardKind::Land {
                produces: Some(LandProduces::Mana(Mana::Color(Color::Green))),
                subtypes: &["Forest"],
                basic: true,
            },
            legendary: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: &[],
            identity_pips: &[],
            colors: &[],
            enters_tapped: false,
            enters_tapped_unless: None,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            flashback: None,
            echo: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
            may_choose_not_to_untap: false,
        }
    }

    #[test]
    fn taps_for_mana_on_an_untapped_forest() {
        let mut game = Game::new();
        let forest = game.spawn_on_battlefield(P0, forest());
        assert!(game.taps_for_mana(forest));
    }

    #[test]
    fn available_mana_counts_an_untapped_land_producer() {
        let mut game = Game::new();
        game.spawn_on_battlefield(P0, forest());
        let mana = game.available_mana(P0);
        assert_eq!(mana.colored[Color::Green.index()], 1);
        assert_eq!(mana.total(), 1);
    }

    #[test]
    fn tap_for_mana_adds_to_the_players_pool() {
        let mut game = Game::new();
        let forest = game.spawn_on_battlefield(P0, forest());
        game.tap_for_mana(P0, forest).unwrap();
        assert_eq!(game.mana_in_pool(P0, Color::Green), 1);
        assert!(game.is_tapped(forest));
    }
}
