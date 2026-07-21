//! Game construction and public object/controller/owner queries.
//!
//! Zone object identity (new [`ObjectId`] on zone change), controller vs owner.
//! Also: multiplayer elimination handoff (CR 800.4a). Deferred / gaps: see
//! per-deck increments under `docs/fidelity/` (fidelity-grind skill).

use crate::*;

impl Game {
    /// A fresh game with the default seat count, seeded for deterministic shuffles.
    pub fn with_seed(seed: u64) -> Self {
        Game::with_players(NUM_PLAYERS, seed)
    }

    /// A fresh `players`-seat game with empty zones, seeded for deterministic shuffles.
    /// Player 0 is the starting active player and holds priority; a lobby that wants a
    /// random first player randomizes the seat→person assignment instead.
    pub fn with_players(players: u8, seed: u64) -> Self {
        let mut master = [0u8; 32];
        master[..8].copy_from_slice(&seed.to_le_bytes());
        Self::with_master_seed(players, master)
    }

    /// A fresh `players`-seat game keyed by a 32-byte master seed.
    pub fn with_master_seed(players: u8, master_seed: [u8; 32]) -> Self {
        Game {
            players: vec![
                Player {
                    life: STARTING_LIFE,
                    ..Player::default()
                };
                players as usize
            ],
            objects: Vec::new(),
            stack: Vec::new(),
            // The raw constructor hands back a game already parked in the active player's first
            // main phase — the ready-to-play state direct-API tests build boards against. It does
            // NOT run turn 1's beginning steps: at construction every zone is empty, so there are
            // no libraries to draw from and nothing to untap or trigger. A real game is set up
            // (libraries shuffled, opening hands drawn) and then calls [`Game::begin_first_turn`],
            // which runs untap/upkeep/draw faithfully.
            active_player: PlayerId(0),
            step: Step::Main1,
            priority: PlayerId(0),
            consecutive_passes: 0,
            pending_trigger_groups: Vec::new(),
            pending_echo: Vec::new(),
            pending_recover: Vec::new(),
            pending_cumulative_upkeep: Vec::new(),
            pending_choice: None,
            resume: crate::resolution::ResumeState::default(),
            clash_won: false,
            skip_next_untap: Vec::new(),
            combat: CombatState::default(),
            combat_extras: state::CombatExtras::default(),
            play_permissions: state::PlayPermissions::default(),
            next_control_timestamp: 0,
            modifier_provenance: state::ModifierProvenance::default(),
            once_per_turn: state::OncePerTurnLimits::default(),
            exile_links: state::ExileLinks::default(),
            delayed_triggers: state::DelayedTriggers::default(),
            master_seed,
            mulliganing: false,
            skip_starting_players_first_draw: false,
            actions: Vec::new(),
            next_action_id: 0,
            batch_trigger_scratch: state::BatchTriggerScratch::default(),
            permanents_died_this_turn: 0,
            damaged_this_turn: Vec::new(),
            resolution_frame: crate::resolution::ResolutionFrame::default(),
            characteristics_cache: characteristics_cache::CharacteristicsCacheCell::default(),
            abilities_granted_until_eot: Vec::new(),
            pending_enter_bonus_counters: Vec::new(),
            exile_time_counters: Vec::new(),
            self_exile_time_counters: None,
            self_tuck_to_library_bottom: false,
        }
    }

    /// Run one derive-per-op random operation for `player`, bumping that seat's iteration counter.
    pub fn with_op_rng<R>(
        &mut self,
        player: PlayerId,
        f: impl FnOnce(&mut crate::rng::OpRng) -> R,
    ) -> R {
        let p = &mut self.players[player.0 as usize];
        let iteration = p.op_iteration;
        p.op_iteration = iteration + 1;
        let key = crate::rng::derive_op_key(&self.master_seed, player.0, iteration);
        let mut rng = crate::rng::op_rng_from_key(&key);
        f(&mut rng)
    }

    /// Begin the game's first turn, once setup is done (libraries shuffled, opening hands drawn).
    /// Runs the active player's untap step and rolls forward to their upkeep, landing priority
    /// there so an upkeep trigger on a permanent that was set up before the game gets its window
    /// (the server's auto-pass then carries an empty upkeep through the draw step into Main1).
    ///
    /// This is the real game-start seam: the constructor deliberately parks at Main1 with the
    /// beginning steps un-run (zones are empty then), and this reruns them once the board exists.
    /// The starting player draws in their first draw step in every game *except* a two-player one,
    /// where they skip it (CR 103.8a/c) — armed here, spent in [`Game::perform_turn_based_actions`].
    pub(crate) fn begin_first_turn_events(&mut self) -> Vec<Event> {
        self.skip_starting_players_first_draw = self.players.len() == 2;

        let mut events = Vec::new();
        let active = self.active_player;
        self.push_apply(
            &mut events,
            Event::StepBegan {
                step: Step::Untap,
                active_player: active,
            },
        );
        self.perform_turn_based_actions(Step::Untap, active, &mut events);
        // Untap has no priority window, so this rolls straight on to the upkeep and stops there. (CR 117, CR 502.1, CR 503)
        events.extend(self.advance_step());

        events
    }

    pub fn begin_first_turn(&mut self) -> Vec<Event> {
        let mut events = self.begin_first_turn_events();
        // Mirror `submit`'s tail so an upkeep trigger reaches the stack.
        self.after_events(&mut events);
        events
    }

    /// A fresh two-player game with the default seed.
    pub fn new() -> Self {
        Game::with_seed(0)
    }

    /// The player whose turn it currently is.
    pub fn active_player(&self) -> PlayerId {
        self.active_player
    }

    /// The current step of the turn.
    pub fn current_step(&self) -> Step {
        self.step
    }

    /// The player who currently holds priority.
    pub fn priority_holder(&self) -> PlayerId {
        self.priority
    }

    /// Whether the stack is empty — cheaper than [`Game::stack`] (which builds a render view)
    /// for callers that only need the emptiness fact (the server's yield scoping).
    pub fn stack_is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// The stack, bottom-first (last element is the top, which resolves next).
    /// A read-only view for rendering — spells carry their stack-object id, abilities
    /// their source and effect.
    pub fn stack(&self) -> Vec<StackEntry> {
        self.stack
            .iter()
            .map(|item| match *item {
                StackItem::Spell(id) => StackEntry::Spell(id),
                // `x` (the ability's chosen `{X}`) and `targets_second` (a second target clause's
                // chosen targets) are internal resolution state, not rendered on the stack view, so
                // they're dropped from the public `StackEntry` (which shows the primary target).
                StackItem::Ability {
                    controller,
                    source,
                    effect,
                    target,
                    targets_second: _,
                    x: _,
                    spent_mana: _,
                    activated: _,
                } => StackEntry::Ability {
                    controller,
                    source,
                    effect,
                    target,
                },
            })
            .collect()
    }

    /// A player's current life total.
    pub fn life(&self, player: PlayerId) -> i32 {
        self.players[player.0 as usize].life
    }

    /// Commander damage `player` has taken, as `(source commander's owner, amount)` pairs. Only
    /// commanders that have actually connected appear. 21 from any single one is lethal (CR 903.10a).
    pub fn commander_damage(&self, player: PlayerId) -> &[(PlayerId, i32)] {
        &self.players[player.0 as usize].commander_damage
    }

    /// Whether a player has lost the game.
    pub fn has_lost(&self, player: PlayerId) -> bool {
        self.players[player.0 as usize].lost
    }

    /// The winner once the game is over: the sole surviving player after everyone else has
    /// been eliminated. `None` while two or more players are still in the game.
    pub fn winner(&self) -> Option<PlayerId> {
        let mut living = (0..self.players.len() as u8)
            .map(PlayerId)
            .filter(|p| !self.players[p.0 as usize].lost);
        let first = living.next();
        match living.next() {
            Some(_) => None, // still ≥2 players in the game
            None => first,   // exactly one (or zero) remain
        }
    }

    /// Test/setup helper: deal `amount` commander damage to `player` from `source` (routed through
    /// an event so state stays mutated only by [`Game::apply`], exactly as [`Game::set_life`] does).
    pub fn deal_commander_damage(&mut self, source: ObjectId, player: PlayerId, amount: i32) {
        self.apply(&Event::CommanderDamageDealt {
            source,
            player,
            amount,
        });
    }

    /// Test/setup helper: set a player's life to `value` (routed through an event
    /// so state stays mutated only by [`Game::apply`]).
    pub fn set_life(&mut self, player: PlayerId, value: i32) {
        let delta = value - self.life(player);
        self.apply(&Event::LifeChanged {
            player,
            amount: delta,
            source: None,
        });
    }

    // ── Object arena ────────────────────────────────────────────────────────────
    // Objects live in an append-only `Vec<Object>`; id = index. A zone change mints a
    // new object (a new id) and leaves an `Object::Moved { to }` tombstone behind.

    /// The id the next created object will receive (pure — for precomputing event ids).
    pub(crate) fn next_object_id(&self) -> ObjectId {
        self.objects.len() as ObjectId
    }

    /// Push `object`, returning its (new) id. If `from` is given, tombstone it to point here.
    pub(crate) fn create_object(&mut self, from: Option<ObjectId>, object: Object) -> ObjectId {
        let id = self.objects.len() as ObjectId;
        // A card leaving a graveyard (reanimation, graveyard recursion, cast-from-graveyard) marks
        // its owner's turn-scoped "a card left your graveyard this turn" flag — the CR 603.4
        // intervening-if behind Relic Retriever / Primary Research. This single object-move choke
        // point catches every graveyard-exit path; a graveyard is only ever left, never entered
        // from itself, so a `from` card in the graveyard is always an exit.
        if let Some(from) = from
            && let Object::Card(c) = self.objects[from as usize]
            && c.zone == Zone::Graveyard
        {
            self.players[c.owner.0 as usize].card_left_graveyard_this_turn = true;
            // ponytail: pushed unconditionally (deduped on drain, not here) — see
            // `graveyard_exits_this_batch`'s doc comment on `Game`.
            self.batch_trigger_scratch
                .graveyard_exits_this_batch
                .push((c.owner, from));
        }
        // Laelia, the Blade Reforged's growth trigger (CR "one or more cards put into exile from
        // your library and/or your graveyard"): this same object-move choke point catches every
        // library/graveyard→exile path (impulse draw, mill-to-exile, graveyard hate) — pushed
        // unconditionally here, deduped on drain like `graveyard_exits_this_batch` above.
        if let Some(from) = from
            && let Object::Card(c) = self.objects[from as usize]
            && matches!(c.zone, Zone::Library | Zone::Graveyard)
            && let Object::Card(new) = &object
            && new.zone == Zone::Exile
        {
            self.batch_trigger_scratch
                .library_or_graveyard_exits_this_batch
                .push(c.owner);
        }
        self.objects.push(object);
        if let Some(from) = from {
            self.objects[from as usize] = Object::Moved { to: id };
        }
        id
    }

    /// The permanent at `id`, panicking if it isn't currently a permanent.
    pub(crate) fn permanent(&self, id: ObjectId) -> &Permanent {
        match &self.objects[id as usize] {
            Object::Permanent(p) => p,
            other => panic!("object {id} is not a permanent: {other:?}"),
        }
    }

    pub(crate) fn permanent_mut(&mut self, id: ObjectId) -> &mut Permanent {
        match &mut self.objects[id as usize] {
            Object::Permanent(p) => p,
            other => panic!("object {id} is not a permanent: {other:?}"),
        }
    }

    /// The permanent at `id`, or `None` if it isn't currently a live permanent.
    pub(crate) fn as_permanent(&self, id: ObjectId) -> Option<&Permanent> {
        match &self.objects[id as usize] {
            Object::Permanent(p) => Some(p),
            _ => None,
        }
    }

    pub(crate) fn spell(&self, id: ObjectId) -> &Spell {
        match &self.objects[id as usize] {
            Object::Spell(s) => s,
            other => panic!("object {id} is not a spell: {other:?}"),
        }
    }

    /// The mutable spell object at `id`. Panics if it isn't a spell on the stack.
    pub(crate) fn spell_mut(&mut self, id: ObjectId) -> &mut Spell {
        match &mut self.objects[id as usize] {
            Object::Spell(s) => s,
            other => panic!("object {id} is not a spell: {other:?}"),
        }
    }

    /// The card definition of whatever live form the object at `id` currently has.
    pub fn def_of(&self, id: ObjectId) -> CardDef {
        match self.objects[id as usize] {
            Object::Card(c) => c.def,
            Object::Spell(s) => s.def,
            // CR 712: a flipped permanent (a Kamigawa flip card) permanently uses its back face's
            // characteristics — every accessor that reads `def_of` (name, types, subtypes,
            // abilities, and `pt_base`) sees the back face at once.
            Object::Permanent(p) if p.flipped => p.def.back.copied().unwrap_or(p.def),
            Object::Permanent(p) => p.def,
            Object::Moved { to } => self.def_of(to),
            Object::Removed => panic!("object {id} has left the game"),
        }
    }

    /// The object's *printed front* card definition, ignoring any flip swap. For a CR 712 flip
    /// card this is the physical printing's front face — the identity a single Scryfall print (one
    /// image) is keyed by — whereas [`def_of`](Self::def_of) returns the live (back) face once
    /// flipped. Used by the wire snapshot to keep a flipped permanent's `card_id`/`print` (art,
    /// oracle lookup) on the shared physical print while name/type/P-T display the back face.
    pub fn front_def_of(&self, id: ObjectId) -> CardDef {
        match self.objects[id as usize] {
            Object::Card(c) => c.def,
            Object::Spell(s) => s.def,
            Object::Permanent(p) => p.def,
            Object::Moved { to } => self.front_def_of(to),
            Object::Removed => panic!("object {id} has left the game"),
        }
    }

    /// Card name for inspect-ledger provenance when `id` may already be [`Object::Removed`]
    /// (a Dies trigger whose source token vanished, or a mana ability whose sacrifice cost
    /// was paid before the effect resolves).
    pub(crate) fn source_name_of(&self, id: ObjectId) -> &'static str {
        match self.objects[id as usize] {
            Object::Removed => "",
            _ => self.def_of(id).name,
        }
    }

    /// The owner of the object at `id` (a spell's controller counts as its owner here).
    pub fn owner_of(&self, id: ObjectId) -> PlayerId {
        match self.objects[id as usize] {
            Object::Card(c) => c.owner,
            Object::Spell(s) => s.controller,
            Object::Permanent(p) => p.owner,
            Object::Moved { to } => self.owner_of(to),
            Object::Removed => panic!("object {id} has left the game"),
        }
    }

    /// The player currently controlling `id` (owner for cards/permanents, caster for a
    /// spell on the stack). Distinct from [`owner_of`] once control-changing effects exist.
    pub fn controller_of(&self, id: ObjectId) -> PlayerId {
        match self.objects[id as usize] {
            Object::Card(c) => c.owner,
            Object::Spell(s) => s.controller,
            Object::Permanent(p) => self.permanent_controller(id, p.owner),
            Object::Moved { to } => self.controller_of(to),
            Object::Removed => panic!("object {id} has left the game"),
        }
    }

    /// The controller of the permanent at `id` under CR 800.4a: when several control-changing
    /// effects apply to one permanent, the most recent (highest-timestamp) one wins. Collects
    /// every live control source for `id` — the three override registries plus any attached
    /// control-changing Aura — and returns the controller of the latest, falling back to the base
    /// `owner` when none applies. A condition-scoped entry is only present while its
    /// [`ControlCondition`] holds (the SBA sweep drops it otherwise), so a present entry means the
    /// steal is still in force.
    fn permanent_controller(&self, id: ObjectId, owner: PlayerId) -> PlayerId {
        let perms = &self.play_permissions;
        // (timestamp, controller) of the most recent control source seen so far.
        let mut latest: Option<(u64, PlayerId)> = None;
        let consider = |ts: u64, controller: PlayerId, latest: &mut Option<(u64, PlayerId)>| {
            if latest.is_none_or(|(t, _)| ts >= t) {
                *latest = Some((ts, controller));
            }
        };
        for &(o, controller, _, ts) in &perms.control_overrides {
            if o == id {
                consider(ts, controller, &mut latest);
            }
        }
        for &(o, controller, _, ts) in &perms.conditioned_control_overrides {
            if o == id {
                consider(ts, controller, &mut latest);
            }
        }
        for &(o, controller, ts) in &perms.permanent_control_overrides {
            if o == id {
                consider(ts, controller, &mut latest);
            }
        }
        if let Some(aura) = self.control_aura(id) {
            let ts = perms
                .aura_control_timestamps
                .iter()
                .find_map(|&(a, t)| (a == aura).then_some(t))
                .unwrap_or(0);
            consider(ts, self.owner_of(aura), &mut latest);
        }
        latest.map_or(owner, |(_, controller)| controller)
    }

    /// The next control-change timestamp (CR 800.4a), consuming it. Called as each control
    /// override / control Aura takes hold so later steals compare newer than earlier ones.
    pub(crate) fn stamp_control_timestamp(&mut self) -> u64 {
        let ts = self.next_control_timestamp;
        self.next_control_timestamp += 1;
        ts
    }

    /// The control-changing Aura (CR 720 — [`Effect::ControlAttached`]) currently attached to
    /// `host`, if any — the object whose owner controls `host` while it stays attached. `None`
    /// when no such Aura is attached. Applied additively over the base owner (engine-core-and-event-model spec), so the
    /// override vanishes on its own when the Aura leaves the battlefield.
    pub(crate) fn control_aura(&self, host: ObjectId) -> Option<ObjectId> {
        self.attachments(host).into_iter().find(|&aura| {
            self.def_of(aura).abilities.iter().any(|a| {
                matches!(
                    (a.timing, a.effect),
                    (Timing::Static, Effect::ControlAttached)
                )
            })
        })
    }

    /// Net +1/+1 counters on the permanent at `id` (0 if it isn't a permanent).
    /// Sourced from inspect-ledger provenance batches (authoritative for counter attribution).
    pub fn plus_counters(&self, id: ObjectId) -> i32 {
        if self.as_permanent(id).is_none() {
            return 0;
        }
        self.modifier_provenance
            .counter_batches
            .iter()
            .filter(|&&(o, _, _)| o == id)
            .map(|&(_, c, _)| c)
            .sum()
    }

    /// Whether any inspect-ledger provenance batches remain for `object` (cleared when it leaves
    /// the battlefield).
    pub fn has_modifier_provenance(&self, object: ObjectId) -> bool {
        self.modifier_provenance
            .counter_batches
            .iter()
            .any(|&(o, ..)| o == object)
            || self
                .modifier_provenance
                .temp_boosts
                .iter()
                .any(|&(o, ..)| o == object)
    }

    /// How many `kind`-counters the permanent at `id` has (0 if it isn't a permanent) — the
    /// named-counter-kind sibling of [`Self::plus_counters`].
    pub fn counters_of_kind(&self, id: ObjectId, kind: CounterKind) -> u8 {
        self.as_permanent(id)
            .map_or(0, |p| p.kind_counters[kind as usize])
    }

    /// How many time counters (CR 702.62 — suspend) the exiled card at `id` has (0 if it carries
    /// none). Read off [`Game::exile_time_counters`], the exile-zone counter store.
    pub fn time_counters(&self, id: ObjectId) -> u32 {
        self.exile_time_counters
            .iter()
            .find(|(o, _)| *o == id)
            .map_or(0, |(_, count)| *count)
    }

    /// The expiry payload of the exiled card at `id`: the `on_expiry` effects of its
    /// [`Effect::ExileSelfWithTimeCounters`] step (All Hallow's Eve's scream-counter self-exile).
    /// Empty (Rousing Refrain's plain suspend, or any card without such a step) means "grant the
    /// suspend free-cast permission when the last counter is removed" — a non-empty slice replaces
    /// that with a graveyard move plus these effects (see [`Step::Upkeep`](crate::Step) tick).
    /// ponytail: scans the ability's own effect and one level of [`Effect::Sequence`] nesting —
    /// deep enough for the pool's self-exile cards; recurse when a card buries the step deeper.
    pub(crate) fn suspend_expiry_payload(&self, id: ObjectId) -> &'static [Effect] {
        let payload = |effect: &Effect| match effect {
            Effect::ExileSelfWithTimeCounters { on_expiry, .. } => Some(*on_expiry),
            _ => None,
        };
        for ability in self.def_of(id).abilities {
            if let Some(on_expiry) = payload(&ability.effect) {
                return on_expiry;
            }
            if let Effect::Sequence { steps } = ability.effect
                && let Some(on_expiry) = steps.iter().find_map(payload)
            {
                return on_expiry;
            }
        }
        &[]
    }

    /// A planeswalker's current loyalty (0 if `id` isn't a permanent).
    pub fn loyalty(&self, id: ObjectId) -> i32 {
        self.as_permanent(id).map_or(0, |p| p.loyalty)
    }

    /// Damage marked on the permanent at `id` this turn (0 if it isn't a permanent).
    pub fn marked_damage(&self, id: ObjectId) -> i32 {
        self.as_permanent(id).map_or(0, |p| p.marked_damage)
    }

    /// Whether the permanent at `id` has a finality counter (CR 122.3g), i.e. it's exiled
    /// instead of dying (`false` if it isn't a permanent).
    pub fn finality_counter(&self, id: ObjectId) -> bool {
        self.as_permanent(id).is_some_and(|p| p.finality_counter)
    }

    /// How many regeneration shields the permanent at `id` currently has (CR 701.15b); 0 if it
    /// isn't a permanent.
    pub fn regeneration_shields(&self, id: ObjectId) -> u8 {
        self.as_permanent(id).map_or(0, |p| p.regeneration_shields)
    }

    /// Whether the permanent at `id` has any counter on it at all — CR 122.1's unqualified
    /// "counter" (Nev, the Practical Dean's "with counters on them"), covering +1/+1, every
    /// named kind, and the finality counter. `false` if `id` isn't a permanent.
    pub fn has_any_counter(&self, id: ObjectId) -> bool {
        self.plus_counters(id) > 0
            || CounterKind::ALL
                .iter()
                .any(|&kind| self.counters_of_kind(id, kind) > 0)
            || self.finality_counter(id)
    }

    /// The total number of counters on the permanent at `id` — CR 122.1's unqualified count (Nils,
    /// Discipline Enforcer's "the number of counters on that creature"), summing +1/+1, every named
    /// kind, and the finality counter. `0` if `id` isn't a permanent.
    pub fn total_counters(&self, id: ObjectId) -> u32 {
        let named: u32 = CounterKind::ALL
            .iter()
            .map(|&kind| self.counters_of_kind(id, kind) as u32)
            .sum();
        self.plus_counters(id).max(0) as u32 + named + self.finality_counter(id) as u32
    }

    /// Whether the permanent at `id` is "prepared" (soc/sos prepare DFCs — its controller may
    /// cast a copy of its back-face spell; see [`Game::cast_prepared`]). `false` if `id` isn't a
    /// permanent.
    pub fn prepared(&self, id: ObjectId) -> bool {
        self.as_permanent(id).is_some_and(|p| p.prepared)
    }

    /// Whether the permanent at `id` is phased out (CR 702.26 — treated as though it doesn't
    /// exist until its controller's next turn). `false` if `id` isn't a permanent.
    pub fn is_phased_out(&self, id: ObjectId) -> bool {
        self.as_permanent(id).is_some_and(|p| p.phased_out)
    }

    /// Whether the permanent at `id` is face down (CR 708 — a manifested card): a 2/2 colorless
    /// creature with no name/types/subtypes/abilities/mana cost until turned face up. `false` if
    /// `id` isn't a permanent. Read by the characteristics overrides and the wire redaction layer.
    pub fn is_face_down(&self, id: ObjectId) -> bool {
        self.as_permanent(id).is_some_and(|p| p.face_down)
    }

    /// Whether the card at `id` sits face down in a hidden/graveyard/exile/command zone (CR
    /// 701.9 — Abstract Performance's first exile pile): hidden from every viewer but its
    /// owner while it holds this flag. `false` if `id` isn't a bare [`Card`] object (a
    /// permanent's own face-down status is [`Self::is_face_down`]). Read by the wire redaction
    /// layer.
    pub fn is_card_face_down(&self, id: ObjectId) -> bool {
        match self.objects[id as usize] {
            Object::Card(c) => c.face_down,
            Object::Moved { to } => self.is_card_face_down(to),
            _ => false,
        }
    }

    /// What casting the card at `id` targets (its first spell-timed targeting effect).
    /// `TargetSpec::None` means the card takes no target.
    pub fn target_spec_of(&self, id: ObjectId) -> TargetSpec {
        // ponytail: mode-less — a modal card's per-mode target need isn't surfaced here (the UI
        // picks a mode first). Reports None for a modal card; wire per-mode specs if the UI wants
        // to preview them.
        self.required_target(self.def_of(id), None)
    }

    /// Target need and legal targets for casting a prepared permanent's back face.
    /// Empty when `source` is not a prepared permanent with a back face.
    pub fn prepared_cast_targets(&self, source: ObjectId) -> (TargetSpec, Vec<Target>) {
        let Some(perm) = self.as_permanent(source) else {
            return (TargetSpec::None, Vec::new());
        };
        if !perm.prepared {
            return (TargetSpec::None, Vec::new());
        }
        let Some(back) = perm.def.back else {
            return (TargetSpec::None, Vec::new());
        };
        let back = *back;
        let controller = self.controller_of(source);
        let spec = self.required_target(back, None);
        if spec == TargetSpec::None {
            return (spec, Vec::new());
        }
        (
            spec,
            self.legal_targets_for(spec, source, controller, color_identity(back), 0),
        )
    }

    /// What activating ability `index` on the permanent at `id` targets (`TargetSpec::None` if it
    /// takes no target). [`Game::target_spec_of`]'s sibling for an activated ability rather than a
    /// cast — the wire layer's `needs_target` for an `Activate` action reads this.
    pub fn ability_target_spec(&self, id: ObjectId, index: usize) -> TargetSpec {
        self.ability_at(id, index)
            .map_or(TargetSpec::None, |a| a.effect.target())
    }

    /// The chosen target of a spell on the stack (`None` if it doesn't target or `id` isn't a spell).
    /// ponytail: a modal spell reports its first chosen mode's target — the stack snapshot shows
    /// one target per spell; surface per-mode targets if the UI wants to preview them all.
    pub fn spell_target(&self, id: ObjectId) -> Option<Target> {
        match &self.objects[id as usize] {
            Object::Spell(s) => s.targets.primary().or_else(|| s.modes.first_target()),
            _ => None,
        }
    }

    /// Whether the spell at `id` currently has exactly one target (CR 114.6's "single target" —
    /// Willbender). Counts the chosen targets across both independent clauses; `false` if `id`
    /// isn't a spell or targets zero/two-plus.
    /// ponytail: a modal spell's per-mode targets aren't counted (they live on `modes`, not the
    /// clause lists) — no pool card bends a modal spell, so the clause count is exact for what's here.
    pub(crate) fn spell_has_single_target(&self, id: ObjectId) -> bool {
        let Object::Spell(s) = &self.objects[id as usize] else {
            return false;
        };
        s.targets.iter().count() + s.targets_second.iter().count() == 1
    }

    /// How many permanents were sacrificed to pay a spell's additional sacrifice cost
    /// ([`AdditionalCost::sacrifice`] — Plumb the Forbidden's "you may sacrifice one or more
    /// creatures"), 0 if `id` isn't a spell, has no such cost, or the caster declined. The seam a
    /// copy-per-sacrifice rider reads once one exists (CR 601.2f's "copy this spell for each
    /// creature sacrificed this way").
    pub fn spell_sacrifice_count(&self, id: ObjectId) -> u8 {
        match &self.objects[id as usize] {
            Object::Spell(s) => s.sacrifice_count,
            _ => 0,
        }
    }

    /// Whether the spell at `id` was cast with its kicker cost paid (CR 702.33d —
    /// [`AdditionalCost::kicker`]), `false` if `id` isn't a spell, has no kicker, or the caster
    /// declined. The seam [`Amount::IfSpellKicked`] reads (Rite of Replication's "If this spell
    /// was kicked, create five of those tokens instead"), the kicked-flag sibling of
    /// [`Self::spell_sacrifice_count`]'s read.
    pub fn spell_was_kicked(&self, id: ObjectId) -> bool {
        match &self.objects[id as usize] {
            Object::Spell(s) => s.kicked,
            _ => false,
        }
    }

    /// The spell at `id`'s declared Strive target count (CR 702.42 — [`AdditionalCost::strive`]),
    /// 0 if `id` isn't a spell or has no Strive cost. [`TargetCount::strive_scaled`]'s cast-time
    /// substitution reads this, the Strive sibling of [`Self::spell_sacrifice_count`]'s read.
    pub(crate) fn spell_strive_count(&self, id: ObjectId) -> u8 {
        match &self.objects[id as usize] {
            Object::Spell(s) => s.strive_count,
            _ => 0,
        }
    }

    /// How many times the spell at `id` had its Replicate cost paid (CR 702.108 —
    /// [`AdditionalCost::replicate`]), 0 if `id` isn't a spell or has no Replicate cost. The
    /// Replicate sibling of [`Self::spell_was_kicked`]'s read — the seam a future "if this spell
    /// was replicated" rider would read (no pool card needs one yet; the copies themselves are
    /// already minted at the cast choke).
    pub fn spell_replicate_count(&self, id: ObjectId) -> u8 {
        match &self.objects[id as usize] {
            Object::Spell(s) => s.replicate_count,
            _ => 0,
        }
    }

    /// The creatures currently declared as attackers.
    pub fn attackers(&self) -> Vec<ObjectId> {
        self.combat.attackers.clone()
    }

    /// Each declared attacker paired with the player it is attacking.
    pub fn attack_targets(&self) -> Vec<(ObjectId, PlayerId)> {
        self.combat.attack_targets.clone()
    }

    /// Whether the active player has already finalized their attack declaration this combat
    /// (including a zero-attacker declaration).
    pub fn attackers_declared(&self) -> bool {
        self.combat.attackers_declared
    }

    /// The declared blocks as `(blocker, attacker)` pairs.
    pub fn blocks(&self) -> Vec<(ObjectId, ObjectId)> {
        self.combat.blocks.clone()
    }

    /// Seats that have already finalized their block declaration this combat (including empty).
    pub fn blockers_declared(&self) -> Vec<PlayerId> {
        self.combat.blocked_by.clone()
    }
}
