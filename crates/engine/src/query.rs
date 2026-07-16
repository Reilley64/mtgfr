//! Meaningful / legal actions and auto-pass predicates.
//!
//! Actions worth stopping priority for (ADR 0007). Also: CR 605 mana-ability carve-outs
//! so tapping for mana does not block auto-pass. Deferred / gaps: see
//! `docs/FIDELITY_BACKLOG.md`.

use crate::*;

impl Game {
    /// Whether `player` has any *meaningful* action available right now — a play worth stopping
    /// priority for. Drives auto-pass: a player with no meaningful action can be safely skipped
    /// (passing is their only real option anyway). Exactly "[`Game::meaningful_actions`] is
    /// non-empty" — see it for what counts and the deliberate exclusions.
    pub fn has_meaningful_action(&self, player: PlayerId) -> bool {
        !self.meaningful_actions(player).is_empty()
    }

    /// Every *meaningful action* `player` may take right now — the plays worth stopping
    /// priority for (ADR 0007): a land drop, a castable spell, an activatable non-mana
    /// ability, or a combat declaration. Auto-pass ([`Game::has_meaningful_action`]) and
    /// intent validation read the same per-action predicates, so they cannot drift apart.
    /// Where validation needs a *chosen* input the query can't know (an ability's target or
    /// sacrifice pick), the query errs toward listing the action: it may stop for a play
    /// whose inputs turn out unsatisfiable (equip with no creature), but never skips a
    /// player who had a legal one.
    ///
    /// Deliberate scoping (ADR 0007): bare mana production (tap-for-mana, mana abilities)
    /// never counts, since it's almost always legal but pointless on its own; on an *empty*
    /// stack, casts and land drops count at sorcery speed only — holding an instant does NOT
    /// stop the flow in combat or on an opponent's turn, otherwise the game halts constantly
    /// for anyone carrying removal (you can still cast it while stopped for another reason).
    /// Once something is ON the stack, an instant-speed cast (an instant, or flash — CR
    /// CR 702.8a) counts too: that reaction window is the whole point of the stack, and the
    /// per-player "don't care" yield is the smooth-flow relief valve. Affordability is
    /// checked against the mana untapped lands *could* produce ([`Game::available_mana`]),
    /// not just the current pool — at the start of a turn the pool is empty and the lands
    /// untapped, but a spell is still castable (auto-tap pays).
    /// ponytail: proactive instant-speed play on an empty stack (holding up mana in the end
    /// step) still has no affordance; add "hold priority" if it's ever wanted. (CR 702.8, CR 117, CR 301.5)
    pub fn meaningful_actions(&self, player: PlayerId) -> Vec<MeaningfulAction> {
        let mut actions = Vec::new();
        let sorcery_ok = self.can_take_sorcery_speed_action(player);
        let available = self.available_mana(player);
        let land_drop_unused = self.players[player.0 as usize].lands_played < 1;

        for (id, o) in self.objects.iter().enumerate() {
            let id = id as ObjectId;
            match o {
                // A land to play or a spell to cast — from hand, the command zone (the
                // player's commander), or impulse-exiled with permission (CR 118.6).
                Object::Card(c) => {
                    // CR 112.6/603.6e: a `functions_in_graveyard` card's activated ability
                    // works from the graveyard itself (Teacher's Pest's "{B}{G}: Return this
                    // card ... to the battlefield tapped") — the graveyard twin of the
                    // `Object::Permanent` arm below.
                    if c.zone == Zone::Graveyard && c.def.functions_in_graveyard {
                        self.push_activatable_abilities(
                            &mut actions,
                            player,
                            id,
                            c.def.abilities,
                            available,
                        );
                    }
                    // Encore (CR 702.140) — a keyword activated ability that functions from the
                    // graveyard, offered right here beside the `functions_in_graveyard` scan.
                    if self.encore_listable(player, id, available) {
                        actions.push(MeaningfulAction::Encore { card: id });
                    }
                    let Some(zone) = self.playable_zone(id, player) else {
                        // Still offer cycling from hand even when the card isn't otherwise playable (CR 702.28, CR 402.5)
                        // (e.g. a land after the land drop is used).
                        if self.cycle_listable(player, id, available) {
                            actions.push(MeaningfulAction::Cycle { card: id });
                        }
                        if self.hand_ability_listable(player, id, available) {
                            actions.push(MeaningfulAction::ActivateHandAbility { card: id });
                        }
                        continue;
                    };
                    // Lands are *played* (a land drop), not cast — their `Cost::FREE` would
                    // otherwise always read as "affordable," so a spare land after the land
                    // drop would falsely count as a castable spell and stop auto-pass.
                    if matches!(c.def.kind, CardKind::Land { .. }) {
                        if sorcery_ok
                            && land_drop_unused
                            && matches!(zone, Zone::Hand | Zone::Exile)
                        {
                            actions.push(MeaningfulAction::PlayLand { card: id, zone });
                        }
                        if self.cycle_listable(player, id, available) {
                            actions.push(MeaningfulAction::Cycle { card: id });
                        }
                        if self.hand_ability_listable(player, id, available) {
                            actions.push(MeaningfulAction::ActivateHandAbility { card: id });
                        }
                        continue;
                    }
                    if let Some(zone) = self.cast_listable(player, id) {
                        actions.push(MeaningfulAction::Cast { card: id, zone });
                    }
                    if self.cycle_listable(player, id, available) {
                        actions.push(MeaningfulAction::Cycle { card: id });
                    }
                    if self.hand_ability_listable(player, id, available) {
                        actions.push(MeaningfulAction::ActivateHandAbility { card: id });
                    }
                    if self.suspend_listable(player, id, available) {
                        actions.push(MeaningfulAction::Suspend { card: id });
                    }
                }
                // A non-mana activated ability the player can afford, or a prepared back-face cast. (CR 602, CR 601, CR 113)
                Object::Permanent(p) => {
                    self.push_activatable_abilities(
                        &mut actions,
                        player,
                        id,
                        p.def.abilities,
                        available,
                    );
                    if self.cast_prepared_listable(player, id) {
                        actions.push(MeaningfulAction::CastPrepared { source: id });
                    }
                    if self.turn_face_up_listable(player, id, available) {
                        actions.push(MeaningfulAction::TurnFaceUp { permanent: id });
                    }
                }
                _ => {}
            }
        }

        // A combat declaration to make (only if this player hasn't already declared).
        let can_attack = player == self.active_player
            && self.step == Step::DeclareAttackers
            && !self.combat.attackers_declared
            && self
                .controlled_battlefield(player)
                .into_iter()
                .any(|id| self.can_attack(id));
        if can_attack {
            actions.push(MeaningfulAction::DeclareAttackers);
        }
        let can_block = self.is_attacked_player(player)
            && self.step == Step::DeclareBlockers
            && !self.combat.blocked_by.contains(&player)
            // Can any of this player's creatures legally block at least one attacker?
            && self.battlefield().into_iter().any(|bid| {
                self.combat
                    .attackers
                    .iter()
                    .any(|&atk| self.can_block(player, bid, atk))
            });
        if can_block {
            actions.push(MeaningfulAction::DeclareBlockers);
        }

        actions
    }

    /// Whether `card` may be offered as a Cycle action: priority holder, in hand with cycling,
    /// and the cycling cost is affordable.
    fn cycle_listable(&self, player: PlayerId, card: ObjectId, available: ManaPool) -> bool {
        if player != self.priority {
            return false;
        }
        let Object::Card(c) = &self.objects[card as usize] else {
            return false;
        };
        if c.zone != Zone::Hand || c.owner != player {
            return false;
        }
        let Some(cost) = c.def.cycling else {
            return false;
        };
        Self::affordable_from(available, cost, None)
    }

    /// Whether `card` may be offered as an ActivateHandAbility action (CR 113.6/602.5e):
    /// priority holder, in hand with a [`CardDef::hand_ability`], and its cost is affordable.
    /// The `hand_ability` sibling of [`Self::cycle_listable`].
    fn hand_ability_listable(&self, player: PlayerId, card: ObjectId, available: ManaPool) -> bool {
        if player != self.priority {
            return false;
        }
        let Object::Card(c) = &self.objects[card as usize] else {
            return false;
        };
        if c.zone != Zone::Hand || c.owner != player {
            return false;
        }
        let Some(ability) = c.def.hand_ability else {
            return false;
        };
        Self::affordable_from(available, ability.cost, None)
    }

    /// Whether `card` may be offered as a Suspend action (CR 702.62): priority holder, in hand
    /// with a suspend cost the player can afford, at a time the card could be cast (CR 702.62b).
    fn suspend_listable(&self, player: PlayerId, card: ObjectId, available: ManaPool) -> bool {
        if player != self.priority {
            return false;
        }
        let Object::Card(c) = &self.objects[card as usize] else {
            return false;
        };
        if c.zone != Zone::Hand || c.owner != player {
            return false;
        }
        let Some(suspend) = c.def.suspend else {
            return false;
        };
        if !self.cast_timing_ok(player, card, c.def, playable::CastPlayKind::List) {
            return false;
        }
        Self::affordable_from(available, *suspend.cost, None)
    }

    /// Whether `card` may be offered as an Encore action (CR 702.140): priority holder, in the
    /// owner's graveyard with an affordable encore cost, at sorcery speed (CR 702.140b — active
    /// player, a main phase, empty stack).
    fn encore_listable(&self, player: PlayerId, card: ObjectId, available: ManaPool) -> bool {
        if player != self.priority {
            return false;
        }
        let Object::Card(c) = &self.objects[card as usize] else {
            return false;
        };
        if c.zone != Zone::Graveyard || c.owner != player {
            return false;
        }
        let Some(cost) = c.def.encore else {
            return false;
        };
        if !self.can_take_sorcery_speed_action(player) {
            return false;
        }
        Self::affordable_from(available, *cost, None)
    }

    /// Whether the face-down manifest `permanent` may be offered the turn-face-up action (CR
    /// 701.34e): priority holder, its controller (owner), the hidden card is a creature card, and
    /// its mana cost is affordable. A noncreature manifest is never turnable (it stays a 2/2).
    fn turn_face_up_listable(
        &self,
        player: PlayerId,
        permanent: ObjectId,
        available: ManaPool,
    ) -> bool {
        if player != self.priority {
            return false;
        }
        let Some(perm) = self.as_permanent(permanent) else {
            return false;
        };
        if !perm.face_down || perm.owner != player {
            return false;
        }
        // CR 701.34e: only a creature card may be turned face up.
        if !matches!(perm.def.kind, CardKind::Creature { .. }) {
            return false;
        }
        Self::affordable_from(available, perm.def.cost, None)
    }

    /// Whether `source` may offer a prepared back-face cast in the action list (timing +
    /// affordability; mode-blind to a concrete target when one is needed).
    fn cast_prepared_listable(&self, player: PlayerId, source: ObjectId) -> bool {
        let Some(perm) = self.as_permanent(source) else {
            return false;
        };
        if perm.owner != player || !perm.prepared {
            return false;
        }
        let Some(back) = perm.def.back else {
            return false;
        };
        let back = *back;
        // Match CastPlayKind::List timing (ADR 0007): instants only in a reaction window or at
        // sorcery speed; sorceries need sorcery speed.
        if back.is_instant_speed() {
            if !self.can_take_sorcery_speed_action(player) && self.stack.is_empty() {
                return false;
            }
        } else if !self.can_take_sorcery_speed_action(player) {
            return false;
        }
        let available = self.available_mana(player);
        let spell = Some(back.spell_characteristics());
        let spec = self.required_target(back, None);
        if spec == TargetSpec::None {
            let cost = self.cast_cost(
                player,
                source,
                back,
                None,
                0,
                Zone::Battlefield,
                0,
                false,
                0,
                0,
            );
            return Self::affordable_from(available, cost, spell);
        }
        self.legal_targets_for(spec, source, player, color_identity(back), 0)
            .into_iter()
            .any(|t| {
                let cost = self.cast_cost(
                    player,
                    source,
                    back,
                    Some(t),
                    0,
                    Zone::Battlefield,
                    0,
                    false,
                    0,
                    0,
                );
                Self::affordable_from(available, cost, spell)
            })
    }

    /// Push every non-mana activated ability of `abilities` (a permanent's or a
    /// graveyard-functional card's own `def.abilities`) that `player` can both legally activate
    /// ([`Game::ability_activation_gate`]) and afford, as a [`MeaningfulAction::Activate`] on
    /// `source`. Shared by [`Game::meaningful_actions`]'s battlefield and graveyard scans so they
    /// can't drift apart.
    fn push_activatable_abilities(
        &self,
        actions: &mut Vec<MeaningfulAction>,
        player: PlayerId,
        source: ObjectId,
        abilities: &'static [Ability],
        available: ManaPool,
    ) {
        for (i, a) in abilities.iter().enumerate() {
            if a.effect.is_mana_ability() {
                continue;
            }
            let Ok((_, cost)) = self.ability_activation_gate(player, source, i) else {
                continue;
            };
            if Self::affordable_from(available, cost.mana, None) {
                actions.push(MeaningfulAction::Activate { source, ability: i });
            }
        }
        // An activated ability granted by an Aura attached to `source` (Fallen Ideal's "Sacrifice (CR 602, CR 303.4, CR 113)
        // a creature: +2/+1"), addressed past the source's own abilities and its granted mana
        // abilities — see `Game::ability_at` for the index order. (Granted *mana* abilities are
        // skipped, like own ones, by the `is_mana_ability` guard above's counterpart in the gate.)
        let base = abilities.len() + self.granted_mana_abilities(source).len();
        for offset in 0..self.granted_attachment_abilities(source).len() {
            let index = base + offset;
            let Ok((_, cost)) = self.ability_activation_gate(player, source, index) else {
                continue;
            };
            if Self::affordable_from(available, cost.mana, None) {
                actions.push(MeaningfulAction::Activate {
                    source,
                    ability: index,
                });
            }
        }
    }

    /// Paid tap-for-mana activates (filter lands, karoos, signets) for the wire radial — **not**
    /// part of [`Game::meaningful_actions`], so they never stop auto-pass (ADR 0007). Appended onto
    /// [`Game::actions`] by [`Game::refresh_actions`] so the client can show them.
    pub(crate) fn paid_mana_activates(&self, player: PlayerId) -> Vec<MeaningfulAction> {
        if player != self.priority {
            return Vec::new();
        }
        let mut actions = Vec::new();
        for (id, o) in self.objects.iter().enumerate() {
            let id = id as ObjectId;
            let Object::Permanent(p) = o else {
                continue;
            };
            if p.owner != player || p.tapped {
                continue;
            }
            for (i, a) in p.def.abilities.iter().enumerate() {
                if !a.effect.is_mana_ability() {
                    continue;
                }
                let Timing::Activated(cost) = a.timing else {
                    continue;
                };
                let Effect::AddMana { single_color, .. } = a.effect else {
                    continue;
                };
                if !cost.taps_self
                    || cost.mana == Cost::FREE
                    || cost.pay_life != Amount::Fixed(0)
                    || !matches!(cost.sacrifice, SacrificeCost::None)
                    || single_color
                {
                    continue;
                }
                let Ok((_, acost)) = self.ability_activation_gate(player, id, i) else {
                    continue;
                };
                // Exclude this source so its own free {{C}} cannot false-list the paid mode.
                if self
                    .plan_auto_taps(player, acost.mana, Some(id), None)
                    .is_none()
                {
                    continue;
                }
                actions.push(MeaningfulAction::Activate {
                    source: id,
                    ability: i,
                });
            }
        }
        actions
    }

    /// The permanents that may pay `object`'s ability-at-`index`'s [`SacrificeCost::Creature`]
    ///
    /// `None` when the ability has no such cost — including "Sacrifice this", which names its own
    /// source and needs no choice. `Some(candidates)` otherwise, and `Some(vec![])` when the
    /// controller has nothing to pay with, which is a different thing from having no cost at all.
    ///
    /// A sacrifice cost is chosen as it's paid (CR 118.9) and rides the activating intent, so an
    /// ability with one is rejected outright until a client offers this list. The set matches
    /// `activate_ability`'s own check exactly — narrowed by the cost's [`PermanentFilter`], so
    /// "sacrifice ANOTHER creature" (`other = true`) excludes the source itself.
    pub fn sacrifice_candidates(&self, object: ObjectId, index: usize) -> Option<Vec<ObjectId>> {
        let Timing::Activated(cost) = self.ability_at(object, index)?.timing else {
            return None;
        };
        let SacrificeCost::Creature { filter, .. } = cost.sacrifice else {
            return None;
        };
        let controller = self.controller_of(object);
        Some(
            self.battlefield()
                .into_iter()
                .filter(|&id| {
                    self.controller_of(id) == controller
                        && self.permanent_matches(&filter, id, controller, Some(object))
                })
                .collect(),
        )
    }

    /// Hand cards that may pay `object`'s additional discard cost when casting it.
    ///
    /// `None` when the spell has no discard cost. `Some(candidates)` otherwise (excluding the
    /// spell itself); `Some([])` when the hand can't pay the required count.
    pub fn discard_cost_candidates(&self, object: ObjectId) -> Option<(Vec<ObjectId>, u8)> {
        let n = self.def_of(object).cost.additional.discard;
        if n == 0 {
            return None;
        }
        let owner = self.owner_of(object);
        let choices: Vec<ObjectId> = self
            .hand_of(owner)
            .into_iter()
            .filter(|&id| id != object)
            .collect();
        if choices.len() < n as usize {
            Some((Vec::new(), n))
        } else {
            Some((choices, n))
        }
    }

    /// Graveyard exile picks for casting `object` (delve or escape).
    ///
    /// `None` when neither keyword applies. Otherwise `(choices, min, max)` — escape uses
    /// exact `exile` (excluding the spell); delve uses `0..=gy_len` over the whole graveyard.
    /// `Some(([], n, n))` when escape needs N and the graveyard can't pay.
    pub fn graveyard_exile_cost(
        &self,
        object: ObjectId,
        zone: Zone,
    ) -> Option<(Vec<ObjectId>, u8, u8)> {
        let def = self.def_of(object);
        let owner = self.owner_of(object);
        let gy = self.graveyard_of(owner);
        if zone == Zone::Graveyard
            && let Some(escape) = def.escape
        {
            let n = escape.exile;
            let choices: Vec<ObjectId> = gy.into_iter().filter(|&id| id != object).collect();
            if choices.len() < n as usize {
                return Some((Vec::new(), n, n));
            }
            return Some((choices, n, n));
        }
        if def.delve {
            // Delve may exile any number from the GY (CR 702.66); cap only at GY size so the
            // client prompt never auto-skips when listability needed a positive delve count. (CR 702.66)
            let max = gy.len().min(u8::MAX as usize) as u8;
            return Some((gy, 0, max));
        }
        None
    }

    /// The printed modes of a modal spell, in printed order, as `(label, legal targets)`. Empty for
    /// a non-modal card — read [`Game::legal_targets`] for those instead.
    ///
    /// A modal spell's targets travel *per mode* (CR 700.2), never in the top-level `target`, so a
    /// client that only knows the card's own target spec can't aim one: `required_target` answers
    /// `None` for a modal card until a mode is picked. This is the enumeration the mode picker reads,
    /// and it is the same `legal_targets_for` the cast gate validates against.
    pub fn modes_of(&self, object: ObjectId) -> Vec<ModeInfo> {
        let def = self.def_of(object);
        if !def.modal {
            return Vec::new();
        }
        let controller = self.controller_of(object);
        let colors = color_identity(def);
        (0..MAX_MODES)
            .map_while(|m| nth_mode(def, m))
            .map(|a| {
                let spec = a.effect.target();
                ModeInfo {
                    label: a.effect.label(),
                    needs_target: spec != TargetSpec::None,
                    // ponytail: mode-blind enumeration, ahead of any {X} choice — x = 0, same
                    // "unknown yet" gap as `legal_targets` below. No pool card is modal *and*
                    // has an X-gated mode target.
                    targets: self.legal_targets_for(spec, object, controller, colors, 0),
                }
            })
            .collect()
    }

    /// The legal upper bound on modes `caster` may choose for `def`'s modal spell (CR 700.2 / CR
    /// 700.2d's "choose one or more"): the plain `modal_choose_max` range, or — when
    /// [`CardDef::modal_choose_max_if_commander`] gates it (Nexus Mentality: "if you control a
    /// commander as you cast this spell, you may choose both instead") — only while `caster`
    /// controls a commander ([`Game::controls_a_commander`]); otherwise the count collapses to the
    /// unconditional `modal_choose`. The single choke [`Game::validate_modes`] (cast legality) and
    /// the wire projection's mode-picker prompt both read this.
    pub fn modal_choose_max(&self, def: CardDef, caster: PlayerId) -> u8 {
        let unconditional_max = def.modal_choose_max.unwrap_or(def.modal_choose);
        if !def.modal_choose_max_if_commander || self.controls_a_commander(caster) {
            return unconditional_max;
        }
        def.modal_choose
    }

    /// Every living player, as a `TargetSpec::Player` legal-target list (CR 111.4 — any player,
    /// no restriction). Used by the wire projection for a modal *triggered* ability's per-mode
    /// Player-target enumeration ([`PendingChoice::ChooseTriggerModes`] — Shadrix Silverquill),
    /// whose modes aren't reachable through [`Game::legal_targets`]'s `object`/`ability_index`
    /// addressing (a trigger's modes have no ability index of their own).
    pub fn legal_player_targets(&self) -> Vec<Target> {
        self.living_players().map(Target::Player).collect()
    }

    /// The legal targets for casting `object` (`ability_index` = `None`) or activating one of
    /// its abilities (`Some(index)`). Empty if that action takes no target. One source of
    /// truth: the cast gate ([`Game::cast`]), auto-pass ([`Game::meaningful_actions`]), and
    /// the client's highlight set all read this same enumeration.
    pub fn legal_targets(&self, object: ObjectId, ability_index: Option<usize>) -> Vec<Target> {
        // ponytail: a targeted *ability* passes no source colors, so protection never filters
        // its targets — no pool ability needs it; wire the source's colors if one does.
        let (spec, source_colors) = match ability_index {
            None => (
                self.required_target(self.def_of(object), None),
                color_identity(self.def_of(object)),
            ),
            Some(i) => (
                self.ability_at(object, i)
                    .map_or(TargetSpec::None, |a| a.effect.target()),
                [false; Color::COUNT],
            ),
        };
        // ponytail: this is the pre-cast highlight enumeration, before the caster has chosen an
        // {X} — x = 0, so a `mv_eq_x` filter (Entrancing Melody) under-highlights until the
        // client asks for X first (CR 601.2b already requires that ordering).
        self.legal_targets_for(spec, object, self.controller_of(object), source_colors, 0)
    }

    /// A triggered/activated ability's source's own entered `{X}` (Kinetic Ooze's ETB, Fractal
    /// Harness's ETB), read from [`Permanent::entered_with_x`] — an ability carries no cast `{X}`
    /// of its own (see `run`'s "abilities carry no X"), but a
    /// [`PermanentFilter::mv_max_x`] target filter needs *some* X, and the entering permanent's
    /// own locked-in cast X is it (CR 601.2b/107.3i). Shared by [`Game::place_targeted_ability`]
    /// (placement-time legality) and [`Game::resolve_top`] (resolution-time re-check), so they
    /// can't read a different X and disagree. 0 for a non-permanent source.
    pub(crate) fn ability_source_x(&self, source: ObjectId) -> u32 {
        match self.objects[source as usize] {
            Object::Permanent(p) => p.entered_with_x,
            _ => 0,
        }
    }

    /// The targets that satisfy a [`TargetSpec`] right now (empty for `None`), for an action
    /// `controller` is taking. Creatures are permanents on the battlefield; players are the living
    /// seats; graveyard scopes enumerate matching creature cards (of `controller`'s graveyard, or
    /// any graveyard). `controller` matters only to the "your graveyard" scope and to protection;
    /// `source_colors` is the acting spell's colors, tested against protection (CR 702.16b).
    /// `source` is the spell/ability object this enumeration is for — read only by
    /// [`TargetSpec::ThisPermanent`]/[`TargetSpec::EnchantedCreature`], which resolve to a fixed
    /// reference to (or attached to) that object rather than offering a real choice. `x` is the
    /// casting spell's chosen `{X}` (CR 601.2b — chosen before targets), read by a
    /// [`PermanentFilter::mv_eq_x`] axis (Entrancing Melody); pass 0 where no `{X}` applies.
    pub(crate) fn legal_targets_for(
        &self,
        spec: TargetSpec,
        source: ObjectId,
        controller: PlayerId,
        source_colors: [bool; Color::COUNT],
        x: u32,
    ) -> Vec<Target> {
        let creatures = || {
            self.live_object_ids()
                .into_iter()
                .filter(|&id| self.is_creature_on_battlefield(id))
                .map(Target::Object)
        };
        let players = || self.living_players().map(Target::Player);
        let planeswalkers = || {
            self.battlefield()
                .into_iter()
                .filter(|&id| {
                    self.def_of(id)
                        .kind
                        .types()
                        .intersects(TypeSet::PLANESWALKER)
                })
                .map(Target::Object)
        };
        // Creature cards in graveyards; `owner` scopes to one player's graveyard when `Some`.
        let graveyard_creatures = |owner: Option<PlayerId>| {
            self.live_object_ids()
                .into_iter()
                .filter(move |&id| {
                    self.zone_of(id) == Zone::Graveyard
                        && matches!(self.def_of(id).kind, CardKind::Creature { .. })
                        && owner.is_none_or(|o| self.owner_of(id) == o)
                })
                .map(Target::Object)
        };
        let mut targets = match spec {
            TargetSpec::None => Vec::new(),
            TargetSpec::Creature => creatures().collect(),
            // "Target creature you control" (Twinflame): a battlefield creature under `controller`.
            TargetSpec::CreatureYouControl => self
                .live_object_ids()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id) && self.controller_of(id) == controller
                })
                .map(Target::Object)
                .collect(),
            TargetSpec::Player => players().collect(),
            // "Target opponent" (CR): a living player other than the choosing player.
            TargetSpec::OpponentPlayer => self
                .living_players()
                .filter(|&p| p != controller)
                .map(Target::Player)
                .collect(),
            // Populate: creature tokens the choosing player controls (CR 701.32).
            TargetSpec::CreatureTokenYouControl => self
                .live_object_ids()
                .into_iter()
                .filter(|&id| {
                    self.is_creature_on_battlefield(id)
                        && self.controller_of(id) == controller
                        && self.as_permanent(id).is_some_and(|p| p.token)
                })
                .map(Target::Object)
                .collect(),
            TargetSpec::AnyTarget => creatures()
                .chain(players())
                .chain(planeswalkers())
                .collect(),
            // Burn that also reaches planeswalkers (Rip Apart, CR 120.3c).
            TargetSpec::CreatureOrPlaneswalker => creatures().chain(planeswalkers()).collect(),
            // Balefire Liege's "target player or planeswalker."
            TargetSpec::PlayerOrPlaneswalker => players().chain(planeswalkers()).collect(),
            // Noncreature permanent removal (Fracture): artifacts, enchantments (incl. Auras),
            // and planeswalkers on the battlefield. Reads the full type set, so an Artifact
            // Creature counts as an artifact here (#19).
            TargetSpec::ArtifactEnchantmentOrPlaneswalker => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.def_of(id).kind.types().intersects(
                        TypeSet::ARTIFACT
                            .union(TypeSet::ENCHANTMENT)
                            .union(TypeSet::PLANESWALKER),
                    )
                })
                .map(Target::Object)
                .collect(),
            // Composable permanent filter (Anguished Unmaking, Abrade, Skyclave Apparition,
            // Tajic's Mentor's "another attacking creature with lesser power"). Threads `source`
            // so the filter's `other` ("another permanent") and `power_less_than_source` axes
            // can read/exclude it.
            TargetSpec::Permanent(filter) => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.permanent_matches(&filter, id, controller, Some(source))
                        // Entrancing Melody's "mana value X" gate (a no-op for every other
                        // filter, none of which set `mv_eq_x`).
                        && (!filter.mv_eq_x || self.def_of(id).mana_value() == x)
                        // Kinetic Ooze's "mana value X or less" gate (same no-op default).
                        && (!filter.mv_max_x || self.def_of(id).mana_value() <= x)
                })
                .map(Target::Object)
                .collect(),
            TargetSpec::CreatureCardInYourGraveyard => {
                graveyard_creatures(Some(controller)).collect()
            }
            TargetSpec::CreatureCardInAnyGraveyard => graveyard_creatures(None).collect(),
            // Composable graveyard filter (Sevinne's Reclamation): any graveyard card whose
            // `CardDef` matches `filter`, scoped to `whose`'s graveyard(s).
            TargetSpec::CardInGraveyard { whose, filter } => self
                .live_object_ids()
                .into_iter()
                .filter(|&id| {
                    self.zone_of(id) == Zone::Graveyard
                        && filter.matches(self.def_of(id))
                        && (whose == GraveyardScope::Any || self.owner_of(id) == controller)
                })
                .map(Target::Object)
                .collect(),
            // Instant/sorcery spells on the stack (any controller's — Twincast can copy an
            // opponent's spell). Reads the live stack, not a zone scan.
            TargetSpec::InstantOrSorcerySpellOnStack => self
                .stack
                .iter()
                .filter_map(|item| match *item {
                    StackItem::Spell(id)
                        if matches!(self.def_of(id).kind, CardKind::Spell { .. }) =>
                    {
                        Some(Target::Object(id))
                    }
                    _ => None,
                })
                .collect(),
            // A spell on the stack matching `filter` (a hard counter with no filter hits creature
            // spells too; Decisive Denial's "noncreature", Quandrix Command's "artifact or
            // enchantment" narrow it — see `Game::spell_matches_filter`).
            TargetSpec::SpellOnStack(filter) => self
                .stack
                .iter()
                .filter_map(|item| match *item {
                    StackItem::Spell(id) => Some(id),
                    _ => None,
                })
                .filter(|&id| {
                    // A counter/target-spell filter never reads the cast-from zone; pass the
                    // plain hand-cast default (see `spell_matches_filter`).
                    self.spell_matches_filter(
                        filter,
                        self.def_of(id),
                        self.spell_target(id),
                        self.controller_of(id),
                        Zone::Hand,
                    )
                })
                .map(Target::Object)
                .collect(),
            // A fixed reference to the ability's own source (Hangarback's "this creature",
            // Gorma's/Primordial Hydra's counter abilities) — empty only if `source` has since
            // left the battlefield (CR 608.2b: nothing left to refer to).
            TargetSpec::ThisPermanent => {
                if self.as_permanent(source).is_some() {
                    vec![Target::Object(source)]
                } else {
                    Vec::new()
                }
            }
            // The Aura/Equipment's host (Redemption Arc's "exile enchanted creature") — empty
            // when `source` isn't currently attached to anything.
            TargetSpec::EnchantedCreature => self
                .attached_to(source)
                .map(Target::Object)
                .into_iter()
                .collect(),
            // Animate Dead's own cast-time graveyard target (CR 303.4a) — the choice already
            // happened when it was cast; empty once that card has left the graveyard (CR
            // 603.3c: an in-response exile fizzles the ETB reanimation, not a re-choice).
            TargetSpec::ThisAurasGraveyardTarget => self
                .as_permanent(source)
                .and_then(|p| p.cast_time_enchant_target)
                .filter(|&card| self.zone_of(card) == Zone::Graveyard)
                .map(Target::Object)
                .into_iter()
                .collect(),
        };
        // These specs aren't targets at all in the CR sense — the reference is fixed, not
        // chosen — so shroud/hexproof/protection (which only restrict *targeting*) don't filter (CR 702.11, CR 601.2c)
        // them (CR 115, 702.11/702.16b/702.18). A hexproof Hangarback Walker still pumps itself.
        if matches!(
            spec,
            TargetSpec::ThisPermanent
                | TargetSpec::EnchantedCreature
                | TargetSpec::ThisAurasGraveyardTarget
        ) {
            return targets;
        }
        // Shroud/hexproof/protection (CR 702.18, 702.11, 702.16b) all restrict who can target a
        // permanent. Only battlefield permanents are filtered — a keyword ability functions only
        // on the battlefield (CR 113.6a), so e.g. a pro-black creature card in a graveyard is
        // still a legal Reanimate target.
        targets.retain(|t| match *t {
            Target::Object(id) => {
                self.as_permanent(id).is_none()
                    || !self.untargetable_by(id, controller, source_colors)
            }
            Target::Player(_) => true,
        });
        targets
    }

    /// True if `id` (a battlefield permanent) can't legally be targeted by something
    /// `controller` is casting/activating, per shroud, hexproof, and protection.
    // ponytail: checked only at target *selection* time (here), matching how protection is (CR 601.2c)
    // modeled — no separate "becomes illegal on resolution" recheck; no pool card needs one.
    fn untargetable_by(
        &self,
        id: ObjectId,
        controller: PlayerId,
        source_colors: [bool; Color::COUNT],
    ) -> bool {
        // Shroud (CR 702.18): can't be targeted by anyone, even its own controller — checked
        // before the own-permanent bypass below.
        if self.has_keyword(id, Keyword::Shroud) {
            return true;
        }
        if self.controller_of(id) == controller {
            return false;
        }
        // Hexproof (CR 702.11): can't be targeted by an opponent.
        if self.has_keyword(id, Keyword::Hexproof) {
            return true;
        }
        // Protection (CR 702.16b): can't be targeted by an opponent's spell of a color it has
        // protection from.
        // ponytail: scoped to an *opponent's* spells — by the CR your own black spell can't
        // target your own pro-black creature either; no pool interaction needs that, and the
        // lenient form never rejects a legal play. (CR 702.16, CR 601.2c, CR 601)
        self.protection_blocks_source_colors(id, source_colors)
    }

    /// The seats still in the game (a spell can only target a living player, CR 115.4).
    pub(crate) fn living_players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        (0..self.players.len() as u8)
            .map(PlayerId)
            .filter(|p| !self.players[p.0 as usize].lost)
    }

    /// Number of cards in `player`'s library.
    pub fn library_size(&self, player: PlayerId) -> usize {
        self.players[player.0 as usize].library.len()
    }

    /// The card ids currently in `player`'s hand.
    pub(crate) fn hand_of(&self, player: PlayerId) -> Vec<ObjectId> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(id, o)| match o {
                Object::Card(c) if c.zone == Zone::Hand && c.owner == player => {
                    Some(id as ObjectId)
                }
                _ => None,
            })
            .collect()
    }

    /// The card ids currently in `player`'s graveyard — [`Game::cast`]'s pool of legal delve/
    /// escape graveyard-exile payments, mirroring [`Self::hand_of`] for discard payments.
    pub(crate) fn graveyard_of(&self, player: PlayerId) -> Vec<ObjectId> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(id, o)| match o {
                Object::Card(c) if c.zone == Zone::Graveyard && c.owner == player => {
                    Some(id as ObjectId)
                }
                _ => None,
            })
            .collect()
    }

    /// The number of seats at the table.
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Ids of every live object (Card/Spell/Permanent), skipping `Moved` tombstones.
    /// The caller filters by [`zone_of`] to pick the zones it cares about.
    pub fn live_object_ids(&self) -> Vec<ObjectId> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| !matches!(o, Object::Moved { .. } | Object::Removed))
            .map(|(id, _)| id as ObjectId)
            .collect()
    }

    /// Whether the spell at `spell` is a creature spell that shares a creature type with
    /// `player`'s commander (Path of Ancestry's spend-to-cast predicate). A creature card's
    /// subtypes are creature types, so a shared subtype is a shared creature type. `false` if the
    /// spell isn't a creature or `player` has no designated commander.
    pub(crate) fn spell_shares_creature_type_with_commander(
        &self,
        player: PlayerId,
        spell: ObjectId,
    ) -> bool {
        let spell_def = self.def_of(spell);
        if !matches!(spell_def.kind, CardKind::Creature { .. }) {
            return false;
        }
        let Some(commander) = self
            .live_object_ids()
            .into_iter()
            .find(|&id| self.is_commander(id) && self.owner_of(id) == player)
        else {
            return false;
        };
        let commander_subtypes = self.def_of(commander).subtypes;
        spell_def
            .subtypes
            .iter()
            .any(|s| commander_subtypes.contains(s))
    }

    /// Whether the object at `id` is (a form of) a commander.
    pub fn is_commander(&self, id: ObjectId) -> bool {
        match self.objects[id as usize] {
            Object::Card(c) => c.commander,
            Object::Spell(s) => s.commander,
            Object::Permanent(p) => p.commander,
            Object::Moved { to } => self.is_commander(to),
            Object::Removed => false,
        }
    }

    /// Whether `player` controls a commander on the battlefield right now (CR 903, "you control a
    /// commander" — Nexus Mentality's modal rider). A commander sitting anywhere else (command
    /// zone, hand, graveyard) doesn't count: control is a battlefield-only relationship, so this
    /// scans [`Game::battlefield`] rather than [`Game::live_object_ids`].
    pub(crate) fn controls_a_commander(&self, player: PlayerId) -> bool {
        self.battlefield()
            .into_iter()
            .any(|id| self.is_commander(id) && self.controller_of(id) == player)
    }

    /// The current live id an old id's lineage points to (following `Moved` tombstones).
    pub fn current_id(&self, id: ObjectId) -> ObjectId {
        match self.objects[id as usize] {
            Object::Moved { to } => self.current_id(to),
            _ => id,
        }
    }

    /// Ids of all live permanents on the battlefield. Excludes phased-out permanents (CR 702.26e:
    /// treated as though they don't exist), so every scan routed through here — statics, combat,
    /// state-based actions, targeting, board counts — skips them until they phase in.
    pub(crate) fn battlefield(&self) -> Vec<ObjectId> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| matches!(o, Object::Permanent(p) if !p.phased_out))
            .map(|(id, _)| id as ObjectId)
            .collect()
    }

    /// Ids of every card in `player`'s graveyard (CR 404). The graveyard twin of
    /// [`battlefield`](Self::battlefield) — used to scan graveyard-functional cards' triggered
    /// abilities (CR 603.6e).
    pub(crate) fn graveyard_cards(&self, player: PlayerId) -> Vec<ObjectId> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                matches!(o, Object::Card(c) if c.zone == Zone::Graveyard && c.owner == player)
            })
            .map(|(id, _)| id as ObjectId)
            .collect()
    }

    /// Whether the permanent `id` satisfies `filter`. `you` is the effect's controller (the
    /// "you" the filter's `controller` axis is relative to); `source` is the filter's own source
    /// permanent, used only by the `other` axis ("another permanent") — pass `None` where there
    /// is nothing to exclude. A non-permanent id never matches. The single evaluator behind
    /// [`TargetSpec::Permanent`], the mass effects, and sacrifice edicts.
    pub(crate) fn permanent_matches(
        &self,
        filter: &PermanentFilter,
        id: ObjectId,
        you: PlayerId,
        source: Option<ObjectId>,
    ) -> bool {
        let Some(perm) = self.as_permanent(id) else {
            return false;
        };
        // A phased-out permanent matches no filter (CR 702.26e — treated as though it doesn't
        // exist): not counted by a board scan, not hit by a mass/edict effect, and no longer a
        // legal already-chosen target (an in-response phase-out fizzles a spell aimed at it).
        if perm.phased_out {
            return false;
        }
        // Types: empty set = any; otherwise the permanent must share at least one required type.
        // Read the CR 613.4-layered types so a type-changing Aura (Darksteel Mutation → Artifact)
        // is counted / hit.
        if !filter.types.is_empty() && !filter.types.intersects(self.effective_types(id)) {
            return false;
        }
        // Subtypes: empty = any; otherwise the permanent must carry at least one named subtype
        // (Goldspan Dragon's "Treasures you control", distinct from any other artifact). Layered
        // subtypes (Angelic Destiny → Angel, Darksteel Mutation → Insect) are read here too.
        if !filter.subtypes.is_empty() {
            let subtypes = self.effective_subtypes(id);
            if !filter.subtypes.iter().any(|s| subtypes.contains(s)) {
                return false;
            }
        }
        // Controller, relative to "you".
        let yours = self.controller_of(id) == you;
        match filter.controller {
            FilterController::Any => {}
            FilterController::You if !yours => return false,
            FilterController::Opponent if yours => return false,
            _ => {}
        }
        // Token-ness.
        match filter.token {
            TokenFilter::Any => {}
            TokenFilter::Token if !perm.token => return false,
            TokenFilter::Nontoken if perm.token => return false,
            _ => {}
        }
        // "another permanent" — never the filter's own source.
        if filter.other && Some(id) == source {
            return false;
        }
        // Enchanted: whether an Aura is attached.
        if let Some(want) = filter.enchanted
            && self.is_enchanted(id) != want
        {
            return false;
        }
        // Attached-to-creature: whether this (Aura) candidate's own host is a creature (CR 303 —
        // Sage's Reverie's "attached to a creature"). The mirror of `enchanted` above, which
        // reads the host side instead of the Aura side.
        if let Some(want) = filter.attached_to_creature {
            let host_is_creature = self
                .attached_to(id)
                .is_some_and(|host| self.is_creature_on_battlefield(host));
            if host_is_creature != want {
                return false;
            }
        }
        // Enchanted by an Aura "you" control (Eriette's attack restriction) — narrower than
        // `enchanted`, which counts any attached Aura.
        if filter.enchanted_by_you && self.auras_controlled_by_attached_to(id, you).is_empty() {
            return false;
        }
        // Mana-value ceiling.
        if let Some(max) = filter.mv_max
            && self.def_of(id).mana_value() > max as u32
        {
            return false;
        }
        // Tapped status (Mana Geyser's "tapped land").
        if let Some(want) = filter.tapped
            && perm.tapped != want
        {
            return false;
        }
        // Power ceiling (Silverquill Charm's "creature with power 2 or less"). Non-creatures
        // have power 0, so they always pass — fine, since no pool card combines the two.
        if let Some(max) = filter.power_max
            && self.power(id) > max as i32
        {
            return false;
        }
        // Power parity (Zimone's Hypothesis's "power of the chosen quality" — zero is even).
        if let Some(parity) = filter.power_parity {
            let is_even = self.power(id).rem_euclid(2) == 0;
            if is_even != (parity == Parity::Even) {
                return false;
            }
        }
        // Noncreature exclusion (Haywire Mite's "noncreature artifact or noncreature
        // enchantment") — rejects an Artifact/Enchantment Creature, not just a bare Creature.
        if filter.noncreature && perm.def.kind.types().intersects(TypeSet::CREATURE) {
            return false;
        }
        // Color-count (Vanishing Verse's "monocolored permanent", CR 105.2a) — a colorless
        // permanent has zero trues in `colors_of` and correctly fails "exactly one".
        if filter.color == ColorFilter::Monocolored
            && self.colors_of(id).iter().filter(|&&c| c).count() != 1
        {
            return false;
        }
        // Specific color (CR 105.2a — Oran-Rief, the Vastwood's "each green creature").
        let specific_color = match filter.color {
            ColorFilter::White => Some(Color::White),
            ColorFilter::Blue => Some(Color::Blue),
            ColorFilter::Black => Some(Color::Black),
            ColorFilter::Red => Some(Color::Red),
            ColorFilter::Green => Some(Color::Green),
            ColorFilter::Any | ColorFilter::Monocolored => None,
        };
        if let Some(color) = specific_color
            && !self.colors_of(id)[color.index()]
        {
            return false;
        }
        // Entered the battlefield this turn (Oran-Rief, the Vastwood). A permanent in a
        // non-battlefield zone has no `Permanent` at all — `as_permanent` already returned
        // `perm` above, so this only reaches permanents that ARE on the battlefield.
        if filter.entered_this_turn && !perm.entered_this_turn {
            return false;
        }
        // Nonbasic land (CR 205.4a's "Basic" supertype — White Orchid Phantom's "target
        // nonbasic land"). Basic-ness reads the def's supertype flag, not subtype strings (a
        // nonbasic dual can share a basic's type line without being basic).
        if filter.nonbasic && is_basic_land(perm.def) {
            return false;
        }
        // "Modified" (CR 701.29 — Silkguard's hexproof rider).
        if filter.modified && !self.is_modified(id) {
            return false;
        }
        // Printed name (CR 201.2 — Leitmotif Composer's "creatures named Leitmotif Composer").
        if let Some(name) = filter.name
            && perm.def.name != name
        {
            return false;
        }
        // Declared as an attacker this combat (Tajic's Mentor — "target attacking creature").
        if filter.attacking && !self.combat.attackers.contains(&id) {
            return false;
        }
        // Nonlegendary exclusion (CR 205.4a — Muddle, the Ever-Changing's "nonlegendary
        // creature you control"). Reads the current (possibly copied) def.
        if filter.nonlegendary && self.def_of(id).legendary {
            return false;
        }
        // Strictly lesser power than the filter's own source (Mentor, CR 702.121a). No-op
        // without a source — every filter that sets this pairs it with a targeted ability,
        // which always threads one.
        if filter.power_less_than_source
            && let Some(source) = source
            && self.power(id) >= self.power(source)
        {
            return false;
        }
        true
    }

    /// Whether `host` has an Aura attached to it ("enchanted", CR 303). Equipment and other
    /// attachments don't count — only Auras enchant.
    fn is_enchanted(&self, host: ObjectId) -> bool {
        self.attachments(host)
            .into_iter()
            .any(|a| matches!(self.def_of(a).kind, CardKind::Aura))
    }

    /// Whether `id` is "modified" (CR 701.29 / Silkguard's reminder text: "Equipment, Auras you
    /// control, and counters are modifications") — has any counter, is enchanted by an Aura, or
    /// is equipped.
    /// ponytail: "Auras you control" is modeled as "any attached Aura" — every pool card that
    /// checks `modified` is checking a permanent's own controller's board, so an opponent's
    /// Aura-attached-by-you case doesn't arise yet; scope to the Aura's controller if one does.
    pub(crate) fn is_modified(&self, id: ObjectId) -> bool {
        self.has_any_counter(id)
            || self.attachments(id).into_iter().any(|a| {
                matches!(self.def_of(a).kind, CardKind::Aura)
                    || self.def_of(a).subtypes.contains(&"Equipment")
            })
    }
}

#[cfg(test)]
mod permanent_filter_tests {
    use crate::*;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    /// A minimal card definition of the given kind and mana value (all generic).
    fn def(kind: CardKind, mv: u8) -> CardDef {
        CardDef {
            name: "T",
            cost: Cost {
                generic: mv,
                colored: [0; Color::COUNT],
                colorless: 0,
                x: 0,
                hybrid: &[],
                additional: AdditionalCost::default(),
                reduce_own_generic: None,
            },
            kind,
            legendary: false,
            uncounterable: false,
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
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            enchant: None,
            enchant_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
        }
    }

    fn creature(also: TypeSet) -> CardKind {
        CardKind::Creature {
            power: 1,
            toughness: 1,
            also,
        }
    }

    #[test]
    fn types_axis_matches_by_shared_type() {
        let mut game = Game::with_players(2, 0);
        let cr = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let art = game.spawn_on_battlefield(P0, def(CardKind::Artifact, 1));
        let land = game.spawn_on_battlefield(
            P0,
            def(
                CardKind::Land {
                    produces: Some(LandProduces::Mana(Mana::Any)),
                    subtypes: &[],
                    basic: false,
                },
                0,
            ),
        );

        let creatures = PermanentFilter::of(TypeSet::CREATURE);
        assert!(game.permanent_matches(&creatures, cr, P0, None));
        assert!(!game.permanent_matches(&creatures, art, P0, None));

        // "nonland" hits creatures and artifacts, but never a land.
        let nonland = PermanentFilter::of(TypeSet::NONLAND);
        assert!(game.permanent_matches(&nonland, cr, P0, None));
        assert!(game.permanent_matches(&nonland, art, P0, None));
        assert!(!game.permanent_matches(&nonland, land, P0, None));

        // An empty type set imposes no restriction.
        let any = PermanentFilter::of(TypeSet::NONE);
        assert!(game.permanent_matches(&any, land, P0, None));
    }

    #[test]
    fn artifact_creature_counts_as_an_artifact() {
        let mut game = Game::with_players(2, 0);
        let plain = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let artifact_creature = game.spawn_on_battlefield(P0, def(creature(TypeSet::ARTIFACT), 1));

        let artifacts = PermanentFilter::of(TypeSet::ARTIFACT);
        assert!(
            !game.permanent_matches(&artifacts, plain, P0, None),
            "a plain creature is not an artifact"
        );
        assert!(
            game.permanent_matches(&artifacts, artifact_creature, P0, None),
            "an Artifact Creature counts as an artifact (#19)"
        );
        // It is still a creature, too.
        assert!(game.permanent_matches(
            &PermanentFilter::of(TypeSet::CREATURE),
            artifact_creature,
            P0,
            None
        ));
    }

    #[test]
    fn controller_axis_is_relative_to_you() {
        let mut game = Game::with_players(2, 0);
        let mine = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let theirs = game.spawn_on_battlefield(P1, def(creature(TypeSet::NONE), 1));

        let yours = PermanentFilter {
            controller: FilterController::You,
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(game.permanent_matches(&yours, mine, P0, None));
        assert!(!game.permanent_matches(&yours, theirs, P0, None));

        let opponents = PermanentFilter {
            controller: FilterController::Opponent,
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(!game.permanent_matches(&opponents, mine, P0, None));
        assert!(game.permanent_matches(&opponents, theirs, P0, None));
    }

    #[test]
    fn name_axis_matches_by_printed_name() {
        // Leitmotif Composer's "creatures named Leitmotif Composer" activated grant.
        let mut game = Game::with_players(2, 0);
        let x = game.spawn_on_battlefield(
            P0,
            CardDef {
                name: "X",
                ..def(creature(TypeSet::NONE), 1)
            },
        );
        let y = game.spawn_on_battlefield(
            P0,
            CardDef {
                name: "Y",
                ..def(creature(TypeSet::NONE), 1)
            },
        );

        let named_x = PermanentFilter {
            name: Some("X"),
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(game.permanent_matches(&named_x, x, P0, None));
        assert!(!game.permanent_matches(&named_x, y, P0, None));
    }

    #[test]
    fn token_axis_distinguishes_tokens() {
        let mut game = Game::with_players(2, 0);
        let nontoken = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let token = game.create_object(
            None,
            Object::Permanent(fresh_token(def(creature(TypeSet::NONE), 0), P0)),
        );

        let nontoken_only = PermanentFilter {
            token: TokenFilter::Nontoken,
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(game.permanent_matches(&nontoken_only, nontoken, P0, None));
        assert!(!game.permanent_matches(&nontoken_only, token, P0, None));

        let token_only = PermanentFilter {
            token: TokenFilter::Token,
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(!game.permanent_matches(&token_only, nontoken, P0, None));
        assert!(game.permanent_matches(&token_only, token, P0, None));
    }

    #[test]
    fn other_axis_excludes_the_source() {
        let mut game = Game::with_players(2, 0);
        let source = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let another = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));

        let others = PermanentFilter {
            other: true,
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(!game.permanent_matches(&others, source, P0, Some(source)));
        assert!(game.permanent_matches(&others, another, P0, Some(source)));
    }

    #[test]
    fn enchanted_axis_checks_for_an_attached_aura() {
        let mut game = Game::with_players(2, 0);
        let bare = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let enchanted = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let aura = game.spawn_on_battlefield(P0, def(CardKind::Aura, 1));
        game.permanent_mut(aura).attached_to = Some(enchanted);

        let unenchanted = PermanentFilter {
            enchanted: Some(false),
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(game.permanent_matches(&unenchanted, bare, P0, None));
        assert!(
            !game.permanent_matches(&unenchanted, enchanted, P0, None),
            "Winds of Rath spares an enchanted creature"
        );

        let has_aura = PermanentFilter {
            enchanted: Some(true),
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(!game.permanent_matches(&has_aura, bare, P0, None));
        assert!(game.permanent_matches(&has_aura, enchanted, P0, None));
    }

    #[test]
    fn attached_to_creature_axis_checks_the_aura_host_is_a_creature() {
        // Sage's Reverie's "each Aura you control that's attached to a creature" (CR 303) — an
        // Aura attached to a noncreature permanent doesn't count, even though it's the theoretical
        // "attached to anything" superset `enchanted`'s host-side check can't distinguish.
        let mut game = Game::with_players(2, 0);
        let creature_host = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 1));
        let artifact_host = game.spawn_on_battlefield(P0, def(CardKind::Artifact, 1));
        let aura_on_creature = game.spawn_on_battlefield(P0, def(CardKind::Aura, 1));
        game.permanent_mut(aura_on_creature).attached_to = Some(creature_host);
        let aura_on_artifact = game.spawn_on_battlefield(P0, def(CardKind::Aura, 1));
        game.permanent_mut(aura_on_artifact).attached_to = Some(artifact_host);

        let creature_attached = PermanentFilter {
            attached_to_creature: Some(true),
            ..PermanentFilter::default()
        };
        assert!(game.permanent_matches(&creature_attached, aura_on_creature, P0, None));
        assert!(
            !game.permanent_matches(&creature_attached, aura_on_artifact, P0, None),
            "attached to an artifact, not a creature"
        );

        let not_creature_attached = PermanentFilter {
            attached_to_creature: Some(false),
            ..PermanentFilter::default()
        };
        assert!(!game.permanent_matches(&not_creature_attached, aura_on_creature, P0, None));
        assert!(game.permanent_matches(&not_creature_attached, aura_on_artifact, P0, None));
    }

    #[test]
    fn mv_max_axis_gates_on_mana_value() {
        let mut game = Game::with_players(2, 0);
        let cheap = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 4));
        let dear = game.spawn_on_battlefield(P0, def(creature(TypeSet::NONE), 5));

        let gated = PermanentFilter {
            mv_max: Some(4),
            ..PermanentFilter::of(TypeSet::CREATURE)
        };
        assert!(
            game.permanent_matches(&gated, cheap, P0, None),
            "mana value 4 passes a 4-or-less gate"
        );
        assert!(
            !game.permanent_matches(&gated, dear, P0, None),
            "mana value 5 fails a 4-or-less gate"
        );
    }
}
