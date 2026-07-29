//! Priority, turn structure, turn-based actions, and cleanup.
//!
//! Turn phases/steps, passing priority, turn-based actions (untap, draw, combat steps
//! advance), cleanup. Also: mana abilities / auto-tap planning (CR 605, turn-priority-and-stack spec).
//! Deferred / gaps: per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;
use std::collections::BTreeSet;

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
        let printed = card_def(perm.def);
        printed.abilities.iter().position(|a| {
            a.effect.clone().is_mana_ability()
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
        // CR 708.2: a face-down permanent has no abilities — a hidden land's `produces` sugar
        // (Zoetic Cavern) is unavailable until it's turned face up.
        if perm.face_down {
            return false;
        }
        let printed = card_def(perm.def);
        if let CardKind::Land {
            produces: Some(_), ..
        } = &printed.kind
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
    /// The CR 800.4a elimination sweep — remove everything the leaver owns (even permanents others
    /// control) and end every control effect that gave *them* control (stolen permanents return to
    /// their owners) — runs in the [`Event::PlayerLost`] apply arm, so it fires for a lethal-damage
    /// elimination just as it does for this concede.
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
        let Object::Permanent(ref perm) = self.objects[object as usize] else {
            return Err(Reject::CannotProduceMana);
        };
        // CR 708.2: a face-down permanent has no abilities — a hidden land's `produces` sugar
        // (Zoetic Cavern) is unavailable until it's turned face up.
        if perm.face_down {
            return Err(Reject::CannotProduceMana);
        }
        let printed = card_def(perm.def);
        // A land with the `produces` sugar has a free base tap-for-one. Everything else that makes
        // mana does it with a real ability — Sol Ring, Arcane Signet, a mana dork, and a fetch-only
        // land's *non*-mana ability (which finds none, and rejects below). Delegate so the one (CR 605, CR 113)
        // activation path enforces summoning sickness and the rest of the gate.
        let CardKind::Land {
            produces: Some(_), ..
        } = &printed.kind
        else {
            let Some(index) = self.free_tap_mana_ability(object) else {
                return Err(Reject::CannotProduceMana);
            };
            return self.activate_ability(player, object, index, None, Vec::new(), Vec::new(), 0);
        };
        // CR 602.2/605.3: tapping a permanent for mana is its *controller*'s action — a stolen
        // land taps for its thief, not its owner.
        if self.controller_of(object) != player || perm.tapped {
            return Err(Reject::CannotProduceMana);
        }

        // "One mana of any color in your commander's color identity" (CR 903.4, Command Tower)
        // and "any color that a land an opponent controls could produce" (Exotic Orchard) both
        // resolve to a real credit here — an empty identity/producible set taps for nothing.
        let mana = self.land_mana_credit(object, player);

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
    /// [`Effect::Static(StaticEffect::TappedForManaBonus)`] on the battlefield adds a bonus credit into the tap's own
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
        let printed = card_def(perm.def);
        if !matches!(&printed.kind, CardKind::Land { .. }) {
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
        let mut fixed_bonuses: Vec<Color> = Vec::new();
        let mut any_color_source: Option<ObjectId> = None;
        for id in self.battlefield() {
            for ability in self.def_of(id).abilities.iter().cloned() {
                let (
                    Timing::Static,
                    Effect::Static(StaticEffect::TappedForManaBonus { scope, bonus_color }),
                ) = (ability.timing, ability.effect.clone())
                else {
                    continue;
                };
                let watches = match scope {
                    LandTapScope::Controller => self.controller_of(id) == player,
                    LandTapScope::EnchantedHost => self.attached_to(id) == Some(land),
                    // "Whenever a player taps a land for mana" / "whenever a Mountain is tapped
                    // for mana" — the tapped land itself is what's filtered, not the tapper, so
                    // `you` is the watcher's own controller and goes unread by both filters.
                    LandTapScope::AnyLand(filter) => {
                        self.permanent_matches(&filter, land, self.controller_of(id), Some(id))
                    }
                };
                if !watches {
                    continue;
                }
                match bonus_color {
                    LandTapBonusColor::Produced => produced_bonuses += 1,
                    // A named color (Wild Growth's "an additional {G}") is nothing to choose and
                    // nothing to copy — credit it straight away.
                    LandTapBonusColor::Fixed(color) => fixed_bonuses.push(color),
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
        for color in fixed_bonuses {
            self.push_apply(
                events,
                Event::ManaAdded {
                    player,
                    mana: Mana::Color(color),
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
        // Manabarbs' "whenever a player taps a land for mana" is a real triggered ability, not an
        // inline bonus — it uses the stack. It hangs here rather than off `Event::Tapped` because
        // this is the only choke that knows the tap produced mana (CR 106.11); the every-tap
        // watches fire from that event instead, so the two never double-fire.
        self.queue_becomes_tapped_triggers(land, true);
    }

    /// Pay 1 life to add {C} under Yavimaya Bloomsage's Channel grant (a CR 605 mana ability —
    /// doesn't use the stack). Legal only while
    /// [`Player::channel_colorless_mana_this_turn`] holds and the player can afford the life
    /// payment (CR 119.4).
    /// ponytail: no source permanent to hang this off of (Channel is spent from hand — see
    /// [`Effect::Misc(MiscEffect::GrantChannelColorlessManaThisTurn)`]'s doc), so it's a standalone `Intent` rather
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
        def: &CardDef,
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
    pub(crate) fn required_target(&self, def: &CardDef, mode: Option<usize>) -> TargetSpec {
        // Animate Dead (CR 303.4a's "enchant creature card in a graveyard"): the pool's one Aura
        // whose enchant subject is a graveyard card, not a battlefield permanent — checked ahead
        // of the ordinary `CardKind::Aura` battlefield-permanent case below (see
        // `CardDef::enchant_graveyard`'s doc).
        if def.enchant_graveyard {
            return TargetSpec::CreatureCardInAnyGraveyard;
        }
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
        if def.modal {
            return mode
                .and_then(|m| nth_mode(def, m))
                .map_or(TargetSpec::None, |a| a.effect.target());
        }
        for ability in def.abilities.iter().cloned() {
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

    /// Whether `object` is an artifact currently on the battlefield (CR 301). A phased-out
    /// permanent doesn't count, mirroring [`Self::is_creature_on_battlefield`]. Used by Copy
    /// Artifact's `enter_as_copy` (`of = "artifact"`, CR 706/707.2) to enumerate its candidates.
    pub(crate) fn is_artifact_on_battlefield(&self, object: ObjectId) -> bool {
        let Some(p) = self.as_permanent(object) else {
            return false;
        };
        !p.phased_out && self.effective_types(object).intersects(TypeSet::ARTIFACT)
    }

    /// The mana `player` could produce right now: their pool plus free taps, then a fixed-point
    /// over paid tap-for-mana abilities (filter lands, karoos, signets) — each unused permanent's
    /// paid ability is included only when the running estimate can pay its activation cost, via
    /// spend-then-merge (never gross-only). Used by [`has_meaningful_action`] so an untapped
    /// board counts as castable mana.
    /// A painland's two free modes are both summed, over-counting a single land's output.
    /// Callers that gate *listable* casts / activates must not use this alone — use
    /// [`Game::plan_auto_taps`] (as [`Game::cast_affordable_list`] and
    /// [`Game::activation_mana_payable`] do) so exclusive modes cannot invent a second pip.
    /// Optimistic over-count remains acceptable for upper bounds such as [`Game::max_payable_x`].
    /// (CR 605, CR 108.3, CR 113)
    pub(crate) fn available_mana(&self, player: PlayerId) -> ManaPool {
        let mut mana = self.players[player.0 as usize].mana_pool;
        let mut used = vec![false; self.objects.len()];
        let mut paid: Vec<(ObjectId, Cost, ManaPool)> = Vec::new();

        for (idx, o) in self.objects.iter().enumerate() {
            let Object::Permanent(p) = o else {
                continue;
            };
            let id = idx as ObjectId;
            // CR 602.2/605.3: a player's available mana counts the permanents they *control*, not
            // merely own — a stolen land contributes to its thief's pool, mirroring `tap_for_mana`.
            // CR 708.2: a face-down permanent (Zoetic Cavern morphed down) has no abilities.
            if self.controller_of(id) != player || p.tapped || p.face_down {
                continue;
            }
            let printed = card_def(p.def);
            // Permanents with a paid tap-for-mana ability (Fetid Heath filter, Study Hall any)
            // are counted only via the fixed-point below — adding their free mode here would
            // mark them used and hide the paid mode when duals are required.
            let has_paid_mana = printed.abilities.iter().any(|a| {
                let Timing::Activated(cost) = a.timing else {
                    return false;
                };
                let Effect::Mana(ManaEffect::Add { single_color, .. }) = a.effect else {
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
                    produces: Some(_), ..
                } = &printed.kind
                && let Some(credit) = self.land_mana_credit(id, player)
            {
                mana.add(credit, 1);
                contributed_free = true;
            }
            for (i, a) in printed.abilities.iter().enumerate() {
                let Timing::Activated(cost) = a.timing else {
                    continue;
                };
                let Effect::Mana(ManaEffect::Add {
                    mana: batch,
                    identity,
                    opponent_colors,
                    restriction,
                    single_color,
                    ..
                }) = a.effect
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
                for (cost, batch, single_color) in self.granted_mana_abilities(id) {
                    // A `single_color` grant (Goldspan's "two mana of any one color") can't be
                    // auto-tapped for the estimate: its color is a manual choice, so its batch
                    // of "any" credits would overstate reachable colors — skip it like an own
                    // `single_color` ability.
                    if cost.taps_self && cost.mana == Cost::FREE && !single_color {
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
            // CR 708.2: a face-down permanent has no abilities.
            if p.owner != player || p.tapped || p.face_down {
                continue;
            }
            let printed = card_def(p.def);
            if let CardKind::Land {
                produces: Some(_), ..
            } = &printed.kind
                && let Some(credit) = self.land_mana_credit(idx as ObjectId, player)
            {
                mana.add(credit, 1);
                used[idx] = true;
                continue;
            }
            for a in printed.abilities.iter().cloned() {
                let Timing::Activated(cost) = a.timing else {
                    continue;
                };
                let Effect::Mana(ManaEffect::Add {
                    mana: batch,
                    identity,
                    opponent_colors,
                    restriction,
                    single_color,
                    ..
                }) = a.effect
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
        // Widen last, so the estimate offers the same colors the payment planners will accept
        // (Sunglasses of Urza) — the merges above compare colors and want the printed ones.
        mana.substituted(&self.mana_substitutions(player))
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

    /// A card's printed non-mana ceiling on {X} (CR 601.2b — Open the Way's player-count bound),
    /// or `None` when X is bounded only by affordability. Both the cast gate
    /// ([`Game::validate_cast`]) and the snapshot's count-picker consult this so the offered
    /// ceiling and the accepted ceiling can never diverge.
    pub fn cast_x_ceiling(&self, def: &CardDef) -> Option<u32> {
        match def.cast_x_max? {
            CastXMax::PlayerCount => Some(u32::from(self.living_player_count())),
        }
    }

    /// Largest X such that `available_mana` can pay `cost_at(x)`, or `0` when even X=0 fails.
    ///
    /// A free cast has no mana-derived upper bound, so it returns zero. A cost whose X is paid
    /// with life instead returns the player's current life total.
    pub fn max_payable_x(
        &self,
        player: PlayerId,
        spell: Option<SpellCharacteristics>,
        mut cost_at: impl FnMut(u32) -> Cost,
    ) -> u32 {
        let available = self.available_mana(player);
        let at_zero = cost_at(0);
        let at_one = cost_at(1);
        let life = self.players[player.0 as usize].life.max(0) as u32;

        if at_zero == at_one
            && at_zero.generic == 0
            && at_zero.colored == [0; Color::COUNT]
            && at_zero.colorless == 0
            && at_zero.hybrid.is_empty()
        {
            return if at_zero.additional.pay_life_x {
                life
            } else {
                0
            };
        }
        if !Self::affordable_from(available, at_zero, spell) {
            return 0;
        }

        let cap = if at_zero.additional.pay_life_x {
            life.min(255)
        } else {
            255
        };
        let mut upper = 1.min(cap);
        while upper < cap && Self::affordable_from(available, cost_at(upper), spell) {
            upper = upper.saturating_mul(2).min(cap);
        }
        if Self::affordable_from(available, cost_at(upper), spell) {
            return upper;
        }

        let mut lower = 0;
        let mut best = 0;
        while lower <= upper {
            let middle = lower + (upper - lower) / 2;
            if Self::affordable_from(available, cost_at(middle), spell) {
                best = middle;
                lower = middle + 1;
            } else if middle == 0 {
                break;
            } else {
                upper = middle - 1;
            }
        }
        best
    }

    /// Plan how to pay `cost` from `player`'s pool. Returns the exact multiset to remove, or
    /// `None` if the pool can't cover it. Pure — the caller applies the [`Event::ManaSpent`].
    pub(crate) fn plan_payment(
        &self,
        player: PlayerId,
        cost: Cost,
        spell: Option<SpellCharacteristics>,
    ) -> Option<ManaPool> {
        let subs = self.mana_substitutions(player);
        let pool = self.players[player.0 as usize].mana_pool;
        let spend = pool.substituted(&subs).spend_plan(&cost, spell)?;
        Some(pool.unsubstitute(&subs, spend))
    }

    /// Can `player` still pay `cost`? The same planner [`Game::settle_payment`] runs, so a `false`
    /// here is exactly the payment the pay path would reject — offer the payment only when this
    /// says yes.
    /// ponytail: sacrifice-cost sources (cracking a Treasure) aren't planned, so a board that can
    /// only pay that way reads `false` until the player floats the mana by hand; teach
    /// [`Game::plan_auto_taps`] sacrifice costs if that ever gates a real prompt.
    pub fn can_pay_cost(&self, player: PlayerId, cost: Cost) -> bool {
        if cost.additional.discard as usize > self.hand_of(player).len() {
            return false;
        }
        self.plan_auto_taps(player, cost, None, None).is_some()
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
        // Every credit — floating and yet to be tapped for — is widened by whatever "spend {W} as
        // though it were {R}" statics the player controls, so the whole greedy search plans in
        // that widened space; the plan is a list of taps, and the real spend is re-planned (and
        // mapped back to real credits) by [`Game::plan_payment`].
        let subs = self.mana_substitutions(player);
        let mut pool = self.players[player.0 as usize].mana_pool.substituted(&subs);
        if pool.can_pay(&cost, spell) {
            return Some(Vec::new());
        }

        let (mut free, mut paid) = self.auto_tap_candidates(player, exclude);
        for candidate in &mut free {
            candidate.credit = candidate.credit.substituted(&subs);
        }
        for candidate in &mut paid {
            candidate.credit = candidate.credit.substituted(&subs);
        }
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
            // CR 708.2: a face-down permanent has no abilities.
            if p.owner != player || p.tapped || p.face_down || Some(id) == exclude {
                continue;
            }
            let printed = card_def(p.def);
            let nonland = !matches!(&printed.kind, CardKind::Land { .. });
            if let Some(m) = self.land_mana_credit(id, player) {
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
            for (i, a) in printed.abilities.iter().enumerate() {
                let Timing::Activated(acost) = a.timing else {
                    continue;
                };
                let Effect::Mana(ManaEffect::Add {
                    mana: batch,
                    identity,
                    opponent_colors,
                    single_color,
                    restriction,
                    ..
                }) = a.effect
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
            let own_len = printed.abilities.len();
            for (gi, (acost, batch, single_color)) in
                self.granted_mana_abilities(id).into_iter().enumerate()
            {
                let index = own_len + gi;
                if !acost.taps_self
                    || acost.mana != Cost::FREE
                    || acost.pay_life != Amount::Fixed(0)
                    || !matches!(acost.sacrifice, SacrificeCost::None)
                    || single_color
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
            MeaningfulAction::KeepHand
            | MeaningfulAction::Mulligan
            | MeaningfulAction::PlayLand { .. }
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
                        player,
                        card,
                        def.clone(),
                        None,
                        0,
                        zone,
                        delve,
                        false,
                        false,
                        false,
                        0,
                        0,
                        0,
                        false,
                    );
                    if let Some(plan) =
                        self.plan_auto_taps(player, cost, None, Some(def.spell_characteristics()))
                    {
                        return object_ids(plan);
                    }
                }
                return Vec::new();
            }
            MeaningfulAction::CastSplitHalf { card, half } => {
                let Some(&face_id) = self.def_of(card).halves.get(half as usize) else {
                    return Vec::new();
                };
                let face = card_def(face_id);
                (
                    self.cast_cost(
                        player,
                        card,
                        face.as_ref().clone(),
                        None,
                        0,
                        Zone::Hand,
                        0,
                        false,
                        false,
                        false,
                        0,
                        0,
                        0,
                        false,
                    ),
                    None,
                    Some(face.spell_characteristics()),
                )
            }
            MeaningfulAction::CastPrepared { source } => {
                let Some(back) = card_def(self.permanent(source).def).back else {
                    return Vec::new();
                };
                let back = card_def(back);
                (
                    self.cast_cost(
                        player,
                        source,
                        back.as_ref().clone(),
                        None,
                        0,
                        Zone::Battlefield,
                        0,
                        false,
                        false,
                        false,
                        0,
                        0,
                        0,
                        false,
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
            MeaningfulAction::ActivateHandAbility { card, index } => {
                let def = self.def_of(card);
                let Some(ability) = def
                    .hand_ability
                    .get(index)
                    .cloned()
                    .or(def.forecast.clone())
                else {
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
            // A standing prevention offer's cost is written on the offer itself (Guardian Angel's
            // {1}) — there is no object to read it off.
            MeaningfulAction::PayStandingPrevention { index } => {
                let Some(offer) = self.standing_preventions.get(index) else {
                    return Vec::new();
                };
                (offer.cost, None, None)
            }
        };
        self.plan_auto_taps(player, cost, exclude, spell)
            .map(object_ids)
            .unwrap_or_default()
    }

    /// Take one of Guardian Angel's standing "you may pay {1} … prevent the next 1 damage that
    /// would be dealt to that permanent or player" offers (CR 615): pay the cost and arm an
    /// ordinary shield on the offer's remembered target. The offer itself survives the payment —
    /// the card says "any time you could cast an instant", not "once" — so it can be bought again
    /// until the turn ends and [`Game::standing_preventions`] is cleared.
    ///
    /// Shaped like [`Game::turn_face_up`]: no stack, no priority pass, an unpayable cost rejects
    /// with nothing tapped.
    pub(crate) fn pay_standing_prevention(
        &mut self,
        player: PlayerId,
        index: usize,
    ) -> Result<Vec<Event>, Reject> {
        if player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        let Some(&offer) = self.standing_preventions.get(index) else {
            return Err(Reject::UnknownAction);
        };
        if offer.player != player {
            return Err(Reject::CannotActivate);
        }
        let mut events = Vec::new();
        self.settle_payment(player, offer.cost, None, None, &mut events)
            .map_err(|_| Reject::CannotPayCost)?;
        self.damage_prevention_shields
            .push(crate::state::PreventionShield {
                target: offer.target,
                amount: Some(offer.amount),
                keep: None,
                from_color: crate::ColorFilter::Any,
                from_source: None,
                combat_only: false,
                gain_life: false,
                redirect_to: None,
            });
        // Paying is a game action taken with priority, not a pass — the payer keeps priority
        // (CR 117.3c), exactly as a special action does.
        self.consecutive_passes = 0;
        self.priority = player;
        Ok(events)
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
                    self.activate_ability(player, source, index, None, Vec::new(), Vec::new(), 0)?
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

            // Whoever is owed an extra turn takes it before the rotation moves on (CR 505.6a).
            // Popped straight off the queue rather than through an event: the `StepBegan` below
            // already carries the new active player, so a pure event replay lands on the same
            // turn order either way (same bookkeeping shape as `skip_starting_players_first_draw`).
            let next_active = if leaving_cleanup {
                match self.extra_turns.pop() {
                    Some(owed) if !self.has_lost(owed) => owed,
                    _ => self.next_player(self.active_player),
                }
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
                        to: None,
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
            // Time Vault (CR 614): "if you would begin your turn while this artifact is tapped,
            // you may skip that turn instead". The pause stands here, after the `StepBegan` that
            // names the new active player but before a single turn-based action has run — so a
            // skipped turn untaps nothing, draws nothing and never opens a priority window.
            // `answer_may` resumes from exactly this point either way.
            // ponytail: the skipped turn's `StepBegan` is emitted before the offer is answered, so
            // a replay sees an untap step for a turn that CR says never happened. Nothing observes
            // it — the per-turn tallies it resets are reset again by the turn that does happen, and
            // no trigger or priority window sits in an untap step — so the alternative (raising the
            // pause before `StepBegan` and re-pushing the whole step preamble in the "no" handler)
            // buys nothing.
            if next == Step::Untap
                && let Some(source) = self.may_skip_turn_offer(next_active)
            {
                crate::pending::raise_choice(
                    self,
                    PendingChoice::MayYesNo {
                        player: next_active,
                        source,
                        effect: Effect::Static(StaticEffect::MaySkipTurnWhileTapped),
                        resume: MayYesNoResume::SkipTurnWhileSourceTapped,
                    },
                );
                return events;
            }
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

    /// The *tapped* battlefield permanent `player` controls that offers Time Vault's
    /// [`StaticEffect::MaySkipTurnWhileTapped`] turn replacement. `None` when they control none or
    /// when every one they control is untapped — the ordinary case, which takes the turn as usual,
    /// since the offer only exists "while this artifact is tapped".
    /// ponytail: first offer wins if a player somehow controls two tapped ones; the pool has a
    /// single such card, and a skipped turn untaps only the one that bought it, so a second would
    /// just re-offer next turn.
    fn may_skip_turn_offer(&self, player: PlayerId) -> Option<ObjectId> {
        self.controlled_battlefield(player).into_iter().find(|&id| {
            self.permanent(id).tapped
                && self.functional_abilities(id).iter().any(|ability| {
                    ability.timing == Timing::Static
                        && matches!(
                            ability.effect,
                            Effect::Static(StaticEffect::MaySkipTurnWhileTapped)
                        )
                })
        })
    }

    /// The battlefield permanent `player` controls that offers Island Sanctuary's
    /// [`StaticEffect::MaySkipDrawForCantBeAttackedBy`] draw-step replacement, with the shield it
    /// buys. `None` when they control none — the ordinary case, which draws as usual.
    /// ponytail: first offer wins if a player somehow controls two; the pool has one such card,
    /// and stacking them buys nothing a single one doesn't (CR 614.5 lets the player order
    /// replacements, but both orders end at the same shield).
    fn may_skip_draw_offer(&self, player: PlayerId) -> Option<(ObjectId, PermanentFilter)> {
        self.controlled_battlefield(player)
            .into_iter()
            .find_map(|id| {
                self.def_of(id).abilities.iter().find_map(|ability| {
                    match (ability.timing, ability.effect.clone()) {
                        (
                            Timing::Static,
                            Effect::Static(StaticEffect::MaySkipDrawForCantBeAttackedBy { filter }),
                        ) => Some((id, filter)),
                        _ => None,
                    }
                })
            })
    }

    /// The draw step's own draw, dredge replacement and all (CR 702.52). Returns after raising
    /// [`PendingChoice::ChooseDredge`] when a dredger is eligible — `answer_choose_dredge` then
    /// performs the draw or the mill+return and resumes the step loop. Shared with
    /// [`Game::answer_may`], which lands here when Island Sanctuary's skip is declined.
    pub(crate) fn draw_step_draw(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        let dredgers = self.dredge_options(player);
        if !dredgers.is_empty() {
            crate::pending::raise_choice(
                self,
                PendingChoice::ChooseDredge {
                    player,
                    eligible: dredgers,
                    remaining: 1,
                    from_draw_step: true,
                },
            );
            return;
        }
        let drawn = self.draw_card(player);
        events.extend(drawn);
    }

    /// The automatic actions performed as a step begins (untap, draw, cleanup).
    pub(crate) fn perform_turn_based_actions(
        &mut self,
        step: Step,
        active: PlayerId,
        events: &mut Vec<Event>,
    ) {
        // CR 800.4e: if the active player leaves the game mid-turn, that turn continues to its
        // completion *without* an active player — nobody untaps, draws, mills for rad, or (below)
        // discards to hand size. Their zones were emptied by the CR 800.4a sweep, so a draw here
        // would reach removed library objects. Every other turn-based action still runs: ending
        // combat (CR 511.3) and cleanup's damage/boost/control housekeeping (CR 514.2) belong to
        // the whole board, not to the active player.
        if self.has_lost(active) && matches!(step, Step::Untap | Step::Draw | Step::Main1) {
            return;
        }
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
                // Stasis's "Players skip their untap steps" (CR 703.4a): the step's two turn-based
                // actions — phasing in and untapping — simply don't happen. Everything else this
                // arm does is a per-*turn* duration that merely gets bookkept here (goad expiry,
                // summoning sickness, loyalty), so it runs whether or not the step is skipped.
                let untap_step_happens = !self.players_skip_untap_steps();
                // Phase in the active player's phased-out permanents (CR 702.26f) — as a turn-based
                // action at the start of the untap step, *before* untapping. Emit one `PhasedIn`
                // per directly-phased permanent (`attached_to.is_none()`); its handler cascades to
                // its indirectly-phased attachments, which phase in together (CR 702.26g).
                // ponytail: keyed on the phased permanent's live controller (`controller_of`),
                // which stands in for CR's "its controller's next turn"; a phased permanent whose
                // control changed while phased is an unmodeled edge no pool card reaches.
                let to_phase_in: Vec<ObjectId> = self
                    .permanent_ids(|p| p.phased_out && p.attached_to.is_none())
                    .filter(|&id| untap_step_happens && self.controller_of(id) == active)
                    .collect();
                for id in to_phase_in {
                    self.push_apply(events, Event::PhasedIn { object: id });
                }
                // "You may choose not to untap this" (CR 502.2 — Rubinia Soulsinger): a tapped
                // permanent carrying the flag isn't untapped here; instead it's offered below in a
                // yes/no pause, and only untapped once the active player declines to keep it tapped.
                let mut optional_untap: Vec<ObjectId> = Vec::new();
                // Smoke / Winter Orb (CR 502.2): a cap on how many permanents of a class may
                // untap. Resolved to concrete groups *before* anything untaps, so the state each
                // cap reads is the one the step started in. A group of one isn't a choice — the
                // lone candidate untaps as it always would — so only groups the cap actually bites
                // are kept, and their members go into the pause below instead of untapping here.
                let capped: Vec<Vec<ObjectId>> = match untap_step_happens {
                    false => Vec::new(),
                    true => self
                        .untap_at_most_one_filters()
                        .into_iter()
                        .map(|(source, filter)| {
                            let controller = self.controller_of(source);
                            self.controlled_battlefield(active)
                                .into_iter()
                                .filter(|&id| {
                                    self.permanent(id).tapped
                                        && !self.skip_next_untap.contains(&id)
                                        && !self.doesnt_untap(id)
                                        && self.permanent_matches(
                                            &filter,
                                            id,
                                            controller,
                                            Some(source),
                                        )
                                })
                                .collect::<Vec<ObjectId>>()
                        })
                        .filter(|group| group.len() > 1)
                        .collect(),
                };
                for id in self.controlled_battlefield(active) {
                    // Pollen Lullaby's win rider (CR): a permanent marked to skip its controller's
                    // next untap step doesn't untap now — the mark is consumed here (whether or not
                    // it was tapped), so it untaps normally on every later untap step.
                    if !untap_step_happens {
                        // A skipped step consumes nothing: a permanent marked to miss its
                        // controller's next untap step is still owed that miss once Stasis goes.
                    } else if self.skip_next_untap.contains(&id) {
                        self.push_apply(events, Event::NextUntapSkipConsumed { object: id });
                    } else if self.permanent(id).tapped && !self.doesnt_untap(id) {
                        if self.def_of(id).may_choose_not_to_untap
                            || capped.iter().any(|group| group.contains(&id))
                        {
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
                            at_most_one: capped,
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
                    if count != 1 {
                        continue;
                    }
                    // All Hallow's Eve (CR 702.62-flavored scream counter): a self-exiled card
                    // carrying an `on_expiry` payload resolves that payload when its last counter
                    // is removed — it goes to its owner's graveyard, then the effects run — instead
                    // of becoming free-castable from exile. A plain suspend card (empty payload)
                    // keeps the free-cast permission below.
                    let payload = self.suspend_expiry_payload(card);
                    if payload.is_empty() {
                        self.push_apply(
                            events,
                            Event::CastFromExileFreePermissionGranted {
                                card,
                                player: active,
                            },
                        );
                        continue;
                    }
                    let owner = self.owner_of(card);
                    let moved = self.next_object_id();
                    self.push_apply(
                        events,
                        Event::MovedToGraveyard {
                            card: moved,
                            from: card,
                        },
                    );
                    let ctx = crate::resolution::ResolveCtx {
                        controller: owner,
                        source: moved,
                        target: None,
                        targets_second: crate::TargetList::default(),
                        x: 0,
                        spent_mana: [0; 6],
                    };
                    self.run_sequence(payload, ctx, events);
                }
                // Vanishing (CR 702.63b — Deadwood Treefolk): at the beginning of its controller's
                // upkeep, *if it has a time counter on it*, remove one from each vanishing
                // permanent they control (the intervening-if is the `left == 0` skip below). When
                // the last one comes off, its controller sacrifices it (CR 702.63c).
                // ponytail: the removal happens directly here rather than through a triggered
                // ability of its own, the same shortcut suspend's tick above takes — nothing in
                // the pool can respond to the removal itself. The sacrifice *is* a real trigger, so
                // responses still get their window, but it is only fired from this tick — emptying
                // the last time counter any other way (a "remove all counters" effect) wouldn't
                // fire it. Its controller is read as the permanent's *owner* (see
                // `queue_self_sacrifice_trigger`), so a stolen vanishing permanent would be
                // sacrificed by the wrong player; nothing in the pool steals one.
                for id in self.controlled_battlefield(active) {
                    if self.def_of(id).vanishing.is_none() {
                        continue;
                    }
                    let left = self.counters_of_kind(id, CounterKind::Time);
                    if left == 0 {
                        continue;
                    }
                    self.push_apply(
                        events,
                        Event::KindCountersPlaced {
                            object: id,
                            kind: CounterKind::Time,
                            count: -1,
                        },
                    );
                    if left == 1 {
                        self.queue_self_sacrifice_trigger(id);
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
                // Island Sanctuary (CR 614): "you may skip that draw" is offered before dredge's
                // own replacement, and declining falls through to it — so a player holding both
                // still gets the dredge choice.
                if let Some((source, filter)) = self.may_skip_draw_offer(active) {
                    crate::pending::raise_choice(
                        self,
                        PendingChoice::MayYesNo {
                            player: active,
                            source,
                            effect: Effect::Static(StaticEffect::MaySkipDrawForCantBeAttackedBy {
                                filter,
                            }),
                            resume: MayYesNoResume::SkipDrawStepDraw,
                        },
                    );
                    return;
                }
                self.draw_step_draw(active, events);
            }
            // Rad counters (CR 122.1, Fallout): "At the beginning of each player's precombat main
            // phase, if that player has any rad counters, they mill that many cards. For each
            // nonland card milled this way, that player loses 1 life and removes one rad counter."
            // A turn-based action, not a triggered ability — no stack object, no priority window,
            // nothing to respond to. `Step::Main1` is the *precombat* main phase (`Main2` is
            // postcombat), and "each player's" resolves in that player's own turn, so only the
            // active player's counters fire here.
            Step::Main1 => self.perform_rad_counter_mill(active, events),
            // The two combat damage steps deal their own batch (CR 510.5). The between-steps
            // SBA sweep and death triggers are handled by `submit` after this step, and a (CR 704, CR 603, CR 104.3)
            // priority window opens between them. (CR 117)
            Step::FirstStrikeCombatDamage => self.combat_damage_substep(true, events),
            Step::CombatDamage => self.combat_damage_substep(false, events),
            Step::EndCombat => {
                // Clockwork Beast's "At end of combat, if this creature attacked or blocked this
                // combat" (CR 511.1) — queued *before* the clear below, which is what the
                // intervening-if reads.
                self.queue_end_of_combat_triggers();
                // Clear combat if attackers were declared this turn (so the declared-flags reset,
                // even after a zero-attacker declaration). No attackers ⇒ nothing to clear.
                if self.combat.attackers_declared {
                    self.push_apply(events, Event::CombatCleared);
                }
                // Jade Statue's "becomes a 3/6 Golem artifact creature until end of combat" — the
                // only duration in the pool shorter than a turn, swept here instead of at cleanup.
                // ponytail: `TempBoostsEnded` ends *every* until-EOT effect on the Statue, so a
                // pump cast on it mid-combat ends early too. Narrow enough to live with; split the
                // event if a second end-of-combat card ever lands.
                let animated: BTreeSet<ObjectId> = self
                    .modifier_provenance
                    .modifiers
                    .iter()
                    .filter(|m| m.duration == ModifierDuration::EndOfCombat)
                    .map(|m| m.host)
                    .collect();
                for id in animated {
                    self.push_apply(events, Event::TempBoostsEnded { object: id });
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

                // Every duration-scoped continuous effect lives in the modifier registry, so the
                // sweep is one scan of it. `BTreeSet` because a permanent pumped twice has two
                // entries, and event order has to be deterministic.
                let boosted: BTreeSet<ObjectId> = self
                    .modifier_provenance
                    .modifiers
                    .iter()
                    .filter(|m| m.duration != ModifierDuration::Indefinite)
                    .map(|m| m.host)
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
                // Nobody is left to discard for a seat that left the game (CR 800.4e).
                if self.has_lost(active) {
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

    /// The rad-counter turn-based action for `player`'s precombat main phase (CR 122.1, Fallout):
    /// mill one card per rad counter, then lose 1 life and remove one rad counter for each
    /// *nonland* card actually milled. A short library mills only what's there, so the life loss
    /// and the removal follow the real nonland count — never the counter total.
    fn perform_rad_counter_mill(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        let rad = self.player_counters(player, PlayerCounterKind::Rad);
        if rad == 0 {
            return;
        }

        let milled = self.mill_events(player, rad as u32);
        let nonland = milled
            .iter()
            .filter(|event| match event {
                Event::Milled { from, .. } => {
                    !matches!(self.def_of(*from).kind, CardKind::Land { .. })
                }
                _ => false,
            })
            .count() as i32;
        self.apply_all(&milled);
        events.extend(milled);
        if nonland == 0 {
            return;
        }

        self.push_apply(
            events,
            Event::LifeChanged {
                player,
                amount: -nonland,
                source: None,
            },
        );
        self.push_apply(
            events,
            Event::PlayerCountersPlaced {
                player,
                kind: PlayerCounterKind::Rad,
                count: -nonland,
            },
        );
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
            snow: false,
            world: false,
            uncounterable: false,
            enchant: None,
            enchant_graveyard: false,
            modal: false,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: empty_slice(),
            conditional_keywords: empty_slice(),
            abilities: empty_slice(),
            identity_pips: empty_slice(),
            colors: empty_slice(),
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            enters_tapped_unless_you_pay_life: None,
            free_cast_if: None,
            alternative_cost: None,
            cast_only_during_combat: false,
            cast_only_before_attackers: false,
            cast_only_before_blockers: false,
            cast_only_during_opponents_turn: false,
            cast_only_before_combat_damage: false,
            cast_only_during_declare_blockers: false,
            cast_only_during_declare_attackers: false,
            approximates: None,
            oracle: None,
            sets: empty_slice(),
            subtypes: empty_slice(),
            otags: empty_slice(),
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
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
            halves: empty_slice(),
            suspend: None,
            vanishing: None,
            cast_x_max: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: empty_slice(),
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
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

    // CR 800.4e: a turn whose active player has left the game runs to completion without an
    // active player — no untap, no draw, no cleanup discard for the seat that is gone. Their
    // library objects were removed by the CR 800.4a sweep, so drawing from it would panic.
    #[test]
    fn an_eliminated_active_player_skips_their_turn_based_actions() {
        let mut game = Game::with_players(4, 0);
        game.spawn_in_library(P0, forest());
        game.apply(&Event::PlayerLost { player: P0 });

        let mut events = Vec::new();
        game.perform_turn_based_actions(Step::Draw, P0, &mut events);

        assert!(events.is_empty());
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
