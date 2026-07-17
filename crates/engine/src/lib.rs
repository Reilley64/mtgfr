//! The game engine: a pure, deterministic, event-sourced sequential state machine.
//! Primary: CR 117 (priority), CR 405 (stack), CR 903 (Commander).
//!
//! State is mutated *only* by applying [`Event`]s for board facts; priority, pending
//! choices, and pass bookkeeping live in the submit path (see `apply.rs`). The sole
//! source of randomness is an injected seed — games replay identically from intents.
//!
//! Objects follow MTG zone rules: a card takes a distinct form — [`Card`] / [`Spell`] /
//! [`Permanent`] — and a *new* [`ObjectId`] each time it changes zones. Old slots
//! become [`Object::Moved`] tombstones (see [`Game::zone_of`] / [`Game::current_id`]).
//!
//! Implemented today: 2–4 player Commander, stack/priority, mana economy, triggered
//! and activated abilities, combat (incl. first strike, trample, multi-block), commander
//! rules, data-driven card scripts, and [`PendingChoice`] pauses. See `CONTEXT.md`,
//! `docs/FIDELITY_BACKLOG.md`, `docs/agent-navigation.md`, and `docs/CR_INDEX.md`
//! for vocabulary, gaps, and CR lookup.

/// Card-DSL deserialization (the `card-dsl` feature): manual impls for the types whose
/// TOML spelling differs structurally from their Rust shape, plus interning/default
/// helpers referenced by the `cfg_attr` serde derives on the types below.
#[cfg(feature = "card-dsl")]
mod de;

mod amount;
mod apply;
mod cast;
mod characteristics;
mod characteristics_cache;
mod combat;
mod core;
mod effects;
mod label;
mod pending;
mod pipeline;
mod playable;
mod priority;
mod query;
mod resolution;
mod spawn;
mod state;
mod triggers;
mod types;
mod zones;

/// Shared Effect-resolution context for [`Game::run`] / [`Game::run_sequence`].
pub(crate) use resolution::ResolveCtx;
pub use state::ControlCondition;
pub use types::*;

/// The authoritative state of one game.
#[derive(Clone)]
pub struct Game {
    pub(crate) players: Vec<Player>,
    pub(crate) objects: Vec<Object>,
    /// Spells/abilities waiting to resolve, last element = top of stack.
    pub(crate) stack: Vec<StackItem>,
    /// The player whose turn it is.
    pub(crate) active_player: PlayerId,
    /// The current step of the active player's turn.
    pub(crate) step: Step,
    /// The player who currently holds priority (may act or pass).
    pub(crate) priority: PlayerId,
    /// How many players have passed priority in succession without acting.
    pub(crate) consecutive_passes: u8,
    /// Abilities that have triggered but aren't on the stack yet; placed (in APNAP
    /// order, each controller ordering their own) the next time priority is granted.
    pub(crate) pending_trigger_groups: Vec<TriggerGroup>,
    /// Permanents whose Echo (CR 702.31) pay-or-sacrifice choice is due but not yet placed —
    /// queued at their controller's upkeep ([`Game::enqueue_triggers`]), drained one at a time
    /// (each becomes a [`PendingChoice::PayEchoOrSacrifice`]) after the ordinary trigger queue
    /// empties in [`Game::place_pending_triggers`].
    pub(crate) pending_echo: Vec<ObjectId>,
    /// A decision the engine is blocked on until the active chooser answers.
    pub(crate) pending_choice: Option<PendingChoice>,
    /// The steps of an [`Effect::Sequence`] still to run after the current pausing step
    /// ([`pending_choice`](Self::pending_choice)) is answered — a deferred continuation
    /// (Prismari Charm's "surveil 2, *then draw a card*"). `None` unless a sequence paused with
    /// more steps to go; resumed by [`Game::resume_deferred_sequence`] once the choice fully clears.
    pub(crate) pending_sequence: Option<resolution::SequenceCont>,
    /// A [`CardKind::Spell`] resolution paused mid-way (a resolution-time optional rider on the
    /// spell's own ability, e.g. `Effect::CopyThisSpell`'s `optional` gate) — the object still
    /// waiting to leave the stack once the pause clears. `None` unless a Timing::Spell ability
    /// paused; resumed by [`Game::resume_deferred_sequence`], which finishes the spell (moves it
    /// to its post-resolution zone) once `pending_choice`/`pending_sequence` both clear. Keeping
    /// the spell a live `Object::Spell` until then lets a resumed continuation
    /// (`Event::SpellCopied`'s copy-from-original) still read it back.
    pub(crate) pending_spell_finish: Option<ObjectId>,
    /// Demonstrate's (CR 702.147a) second copy: `(opponent, spell)`, recorded once the
    /// controller's own copy is minted (see the `Effect::Demonstrate` branch in
    /// [`Game::choose_targets`]). `None` unless a Demonstrate accept is mid-resolution; drained by
    /// [`Game::resume_deferred_sequence`] once `pending_choice`/`pending_sequence` both clear, the
    /// same "wait for the current pause, then continue" shape as
    /// [`pending_spell_finish`](Self::pending_spell_finish) — the opponent's copy can't mint
    /// alongside the controller's own copy's CR 707.10c retarget pause, since the two copies have
    /// different controllers and `mint_spell_copies` shares one [`resolution::ResolveCtx`] per call.
    pub(crate) pending_demonstrate_opponent_copy: Option<(PlayerId, ObjectId)>,
    /// The current combat's attackers, blocks, and orderings (empty outside combat).
    pub(crate) combat: CombatState,
    /// Goad and other combat-adjacent state lifted off `Permanent` so it stays `Copy`.
    pub(crate) combat_extras: state::CombatExtras,
    /// Active play/control permissions lifted off `Card`/`Permanent` so they stay `Copy`.
    pub(crate) play_permissions: state::PlayPermissions,
    /// Sourced counter/EOT-boost batches for the Alt-inspect mod ledger.
    pub(crate) modifier_provenance: state::ModifierProvenance,
    /// Once-per-turn activation/trigger caps, reset at each untap step.
    pub(crate) once_per_turn: state::OncePerTurnLimits,
    /// Links between exiling sources and the cards they exiled.
    pub(crate) exile_links: state::ExileLinks,
    /// Pending CR 603.7 delayed triggered abilities not yet placed on the stack.
    pub(crate) delayed_triggers: state::DelayedTriggers,
    /// Injected PRNG state (splitmix64) — the only source of randomness, so the
    /// engine stays deterministic: same seed ⇒ same shuffles.
    pub(crate) rng_state: u64,
    /// Whether the active player's next draw step is skipped: the starting player skips their
    /// first draw in a two-player game only (CR 103.8a; multiplayer skips no one, CR 103.8c).
    /// Armed by [`Game::begin_first_turn`] from the seat count and spent on the first draw step.
    pub(crate) skip_starting_players_first_draw: bool,
    /// Every player's full legal-action list, recomputed at the tail of each state change
    /// ([`Game::refresh_actions`]). Rides the per-viewer snapshot so each player sees only their
    /// own. Empty while a [`pending_choice`](Self::pending_choice) is up — the choice's answer
    /// intents are then the only legal moves.
    pub(crate) actions: Vec<LegalAction>,
    /// Monotonic source of [`LegalAction`] ids — never reset, so an id is unique for the game's
    /// lifetime and a stale id can't be confused with a live one.
    pub(crate) next_action_id: u64,
    /// Transient per-batch scratch for trigger enqueueing — not event-sourced.
    pub(crate) batch_trigger_scratch: state::BatchTriggerScratch,
    /// How many permanents (any type, any controller) were put into a graveyard from the
    /// battlefield this turn — a game-wide (not per-player) turn-scoped tally, reset at every
    /// Untap step alongside the per-player tallies. Feeds `Amount::PermanentsDiedThisTurn`
    /// (Ominous Harvest's Gravestorm).
    pub(crate) permanents_died_this_turn: u32,
    /// Resolution-local "this way" scratch (DestroyAll / ExileAll / mill / council / edict riders).
    /// Not turn-scoped — see [`resolution::ResolutionFrame`].
    pub(crate) resolution_frame: resolution::ResolutionFrame,
    /// Controllers owed 2 life by Serra Paragon's granted rider (CR 118.9) for a permanent just
    /// exiled on death this event batch — captured at the [`Event::MovedToExile`] choke (the
    /// permanent's `serra_recursion` tag is gone once it's moved) and drained into
    /// [`Event::LifeChanged`] by the SBA fixpoint sweep ([`Game::sweep_state_based_actions`]).
    pub(crate) pending_serra_lifegain: Vec<PlayerId>,
    /// Memoized effective P/T and keywords per object; invalidated on relevant [`Event`]s.
    pub(crate) characteristics_cache: characteristics_cache::CharacteristicsCacheCell,
    /// `(target, source)` pairs where `target` has gained `source`'s other abilities until end of
    /// turn (CR 702.166 Backup — Guardian Scalelord). The granted set is read live off `source`'s
    /// `CardDef.abilities`/`keywords` (minus the granting ability itself), so it tracks the
    /// source's current characteristics; cleared at cleanup by [`Event::GrantedAbilitiesEnded`].
    pub(crate) abilities_granted_until_eot: Vec<(ObjectId, ObjectId)>,
    /// Additional +1/+1 counters a specific about-to-resolve spell must enter with, keyed by the
    /// spell's stack object id. Captured at cast payment by Opal Palace's
    /// [`Trigger::SpendManaToCast`] rider ([`Effect::CommanderEntersWithBonusCounters`]) and drained
    /// by [`Game::resolve_spell`] when the spell becomes a permanent (CR 601.2 — the "enters with"
    /// rider is set before resolution). Engine-internal scratch, never wire-mirrored.
    pub(crate) pending_enter_bonus_counters: Vec<(ObjectId, u32)>,
    /// Time counters (CR 702.62 — suspend) on cards in exile, each `(exile object, count)`. Kept
    /// off the `Copy` [`Object::Card`] (an exiled card carries no counter field, unlike a
    /// [`Permanent`]); an entry is created as the card is suspended and dropped when the last
    /// counter is removed (the card becomes castable) — see [`Game::exile_time_counters`].
    pub(crate) exile_time_counters: Vec<(ObjectId, u32)>,
    /// Set by an [`Effect::ExileSelfWithTimeCounters`] step while a spell resolves, so
    /// [`Game::finish_instant_sorcery_resolution`] exiles that spell with time counters rather
    /// than sending it to the graveyard (Rousing Refrain). Consumed (`take`) in `finish`, which
    /// always runs right after the spell's effects — only one spell resolves at a time.
    pub(crate) self_exile_time_counters: Option<u32>,
}

impl Game {
    /// The decision the engine is currently waiting on, if any.
    pub fn pending_choice(&self) -> Option<PendingChoice> {
        self.pending_choice.clone()
    }

    /// Validate an intent, emit the resulting events, apply them, and return them.
    ///
    /// After the action: state-based actions are swept to a fixpoint, then any newly-triggered
    /// abilities are put on the stack (which may raise a choice) — both happen when a
    /// player would receive priority.
    pub fn submit(&mut self, intent: Intent) -> Result<Vec<Event>, Reject> {
        // While a choice is pending, only an answer intent from that player is legal (the
        // specific handler rejects an answer that doesn't match the pending choice's kind).
        // Conceding is the exception: a player must be able to quit whoever the game is waiting on,
        // including themselves — otherwise the seat that walked away blocks the whole table.
        if let Some(choice) = &self.pending_choice
            && !matches!(intent, Intent::Concede { .. })
            && (!pending::is_answer(&intent) || intent.actor() != choice.player())
        {
            return Err(Reject::ChoicePending);
        }

        // Reject any out-of-range object id before dispatch, so untrusted input can't index
        // past the arena and panic. In-range-but-illegal ids fall to each action's own checks.
        if intent
            .object_ids()
            .iter()
            .any(|&id| id as usize >= self.objects.len())
        {
            return Err(Reject::UnknownObject);
        }

        let mut events = if pending::is_answer(&intent) {
            pending::answer(self, intent)?
        } else {
            match intent {
                Intent::Cast {
                    player,
                    object,
                    target,
                    x,
                    modes,
                    discard_cost,
                    graveyard_exile,
                    sacrifice_cost,
                    kicked,
                    bought_back,
                    evoked,
                    strive_count,
                    replicate_count,
                } => self.cast(
                    player,
                    object,
                    target,
                    x,
                    modes,
                    discard_cost,
                    graveyard_exile,
                    sacrifice_cost,
                    kicked,
                    bought_back,
                    evoked,
                    strive_count,
                    replicate_count,
                )?,
                Intent::PlayLand { player, object } => self.play_land(player, object)?,
                Intent::Cycle { player, card } => self.cycle(player, card)?,
                Intent::ActivateHandAbility { player, card } => {
                    self.activate_hand_ability(player, card)?
                }
                Intent::Suspend { player, card } => self.suspend(player, card)?,
                Intent::Encore { player, card } => self.encore(player, card)?,
                Intent::TurnFaceUp { player, permanent } => self.turn_face_up(player, permanent)?,
                Intent::CastPrepared {
                    player,
                    source,
                    target,
                    x,
                } => self.cast_prepared(player, source, target, x)?,
                Intent::CastAdventure {
                    player,
                    source,
                    target,
                    x,
                } => self.cast_adventure(player, source, target, x)?,
                Intent::CastBestow {
                    player,
                    object,
                    target,
                } => self.cast_bestow(player, object, target)?,
                Intent::CastFaceDown { player, card } => self.cast_face_down(player, card)?,
                Intent::TapForMana { player, object } => self.tap_for_mana(player, object)?,
                Intent::ChannelColorlessMana { player } => self.channel_colorless_mana(player)?,
                Intent::Concede { player } => self.concede(player),
                Intent::ActivateAbility {
                    player,
                    object,
                    ability_index,
                    target,
                    sacrifice,
                    x,
                } => self.activate_ability(player, object, ability_index, target, sacrifice, x)?,
                Intent::DeclareAttackers { player, attackers } => {
                    self.declare_attackers(player, &attackers)?
                }
                Intent::DeclareBlockers { player, blocks } => {
                    self.declare_blockers(player, &blocks)?
                }
                Intent::PassPriority { player } => self.pass_priority(player)?,
                Intent::TakeAction {
                    player,
                    id,
                    target,
                    x,
                    modes,
                    sacrifice,
                    discard_cost,
                    graveyard_exile,
                    attackers,
                    blocks,
                } => self.take_action(
                    player,
                    id,
                    target,
                    x,
                    modes,
                    sacrifice,
                    &discard_cost,
                    &graveyard_exile,
                    attackers,
                    blocks,
                )?,
                _ => unreachable!("choice answers gated above"),
            }
        };

        self.resume_deferred_sequence(&mut events);

        self.after_events(&mut events);
        Ok(events)
    }

    /// Post-intent (and first-turn) pipeline. Delegates to [`pipeline::PostIntentPipeline`].
    pub(crate) fn after_events(&mut self, events: &mut Vec<Event>) {
        pipeline::PostIntentPipeline::run(self, events);
    }

    /// The engine's stored per-player legal-action list, recomputed after every state change.
    /// Each entry carries a unique [`LegalAction::id`] a client sends back via
    /// [`Intent::TakeAction`]. Filter by `player` for one seat's actions (the snapshot layer
    /// does exactly this per viewer). Empty while a pending choice is up.
    pub fn legal_actions(&self) -> &[LegalAction] {
        &self.actions
    }

    /// The single legal answer to the current pending choice, if the choice is *forced* — i.e.
    /// only one answer is possible, so the server can auto-submit it (tagging it for the client).
    /// `None` when there is no choice, or when the choice genuinely has more than one option.
    ///
    /// Conservative on purpose (a real decision must never be made for the player): a forced
    /// discard is only forced when the whole hand must go; a target/order/edict only when a
    /// single option exists. Never a [`PendingChoice::MayYesNo`]/[`PendingChoice::PayCost`]
    /// (declining is always a legal choice), never a [`PendingChoice::ArrangeTop`] scry/surveil
    /// (top-vs-bottom is a real choice even for one card), never a
    /// [`PendingChoice::SearchLibrary`] (fail-to-find is legal), never a keep-one
    /// [`PendingChoice::SacrificeEdict`] (which one to keep is a real choice).
    pub fn forced_action(&self) -> Option<Intent> {
        pending::forced(self)
    }

    /// Resolve an [`Intent::TakeAction`]: look `id` up in the stored legal-action list and
    /// dispatch to the same private handler the equivalent concrete intent would. An unknown id,
    /// or one whose stored `player` isn't the submitter, is [`Reject::UnknownAction`].
    #[allow(clippy::too_many_arguments)]
    fn take_action(
        &mut self,
        player: PlayerId,
        id: u64,
        target: Option<Target>,
        x: u32,
        modes: Vec<(usize, Option<Target>)>,
        sacrifice: Vec<ObjectId>,
        discard_cost: &[ObjectId],
        graveyard_exile: &[ObjectId],
        attackers: Vec<(ObjectId, PlayerId)>,
        blocks: Vec<(ObjectId, ObjectId)>,
    ) -> Result<Vec<Event>, Reject> {
        // `LegalAction` is `Copy`, so this ends the immutable borrow before dispatch.
        let Some(action) = self.actions.iter().find(|a| a.id == id).copied() else {
            return Err(Reject::UnknownAction);
        };
        if action.player != player {
            return Err(Reject::UnknownAction);
        }
        match action.kind {
            MeaningfulAction::PlayLand { card, .. } => self.play_land(player, card),
            MeaningfulAction::Cast { card, .. } => self.cast_with_kind(
                player,
                card,
                target,
                x,
                &modes,
                discard_cost,
                graveyard_exile,
                &[],
                false,
                false,
                false,
                0,
                0,
                playable::CastPlayKind::OneClick,
            ),
            MeaningfulAction::Activate { source, ability } => {
                // ponytail: a one-click/take-action activation picks no `{X}` (x = 0) — no pool
                // card has an `{X}`-cost activated ability, so the action list never offers one.
                // Thread a chosen X through here when a card first needs it.
                self.activate_ability(player, source, ability, target, sacrifice, 0)
            }
            MeaningfulAction::Cycle { card } => self.cycle(player, card),
            MeaningfulAction::ActivateHandAbility { card } => {
                self.activate_hand_ability(player, card)
            }
            MeaningfulAction::Suspend { card } => self.suspend(player, card),
            MeaningfulAction::Encore { card } => self.encore(player, card),
            MeaningfulAction::TurnFaceUp { permanent } => self.turn_face_up(player, permanent),
            MeaningfulAction::CastPrepared { source } => {
                self.cast_prepared(player, source, target, x)
            }
            MeaningfulAction::CastFaceDown { card } => self.cast_face_down(player, card),
            MeaningfulAction::DeclareAttackers => self.declare_attackers(player, &attackers),
            MeaningfulAction::DeclareBlockers => self.declare_blockers(player, &blocks),
        }
    }

    /// Recompute every living player's legal-action list. An action that survived the state
    /// change (same `player` + `kind` as before) keeps its id — a client holding an id (e.g.
    /// mid auto-tap-then-cast) isn't invalidated by intents that didn't remove the action.
    /// Genuinely new entries mint fresh monotonic ids, so a dead id can never collide with a
    /// live one. Runs at the tail of every state change ([`Game::submit`],
    /// [`Game::begin_first_turn`]).
    /// While a pending choice is up the list is empty — the choice's answer intents are then the
    /// only legal moves (`submit` gates on this), so there is nothing to offer as an action.
    /// A non-priority seat naturally gets an empty list from [`Game::meaningful_actions`]'s own
    /// predicates, so this recomputes for every seat without special-casing whose turn it is.
    /// ponytail: recompute-everything each frame; the per-seat lists are tiny, so no incremental
    /// diffing — revisit only if profiling ever shows this hot. (CR 117, CR 601, CR 500)
    pub(crate) fn refresh_actions(&mut self) {
        let previous = std::mem::take(&mut self.actions);
        if self.pending_choice.is_some() {
            return;
        }
        for seat in 0..self.players.len() as u8 {
            let player = PlayerId(seat);
            if self.players[seat as usize].lost {
                continue;
            }
            for kind in self
                .meaningful_actions(player)
                .into_iter()
                .chain(self.paid_mana_activates(player))
            {
                let id = match previous
                    .iter()
                    .find(|a| a.player == player && a.kind == kind)
                {
                    Some(kept) => kept.id,
                    None => {
                        let id = self.next_action_id;
                        self.next_action_id += 1;
                        id
                    }
                };
                self.actions.push(LegalAction { id, player, kind });
            }
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod forced_action_tests {
    //! `forced_action` reads only the pending choice's own fields, so these construct choices
    //! directly with dummy object ids — no board is needed to exercise the forced/not-forced set.
    use crate::*;

    const P0: PlayerId = PlayerId(0);

    #[test]
    fn no_choice_is_never_forced() {
        let game = Game::with_players(2, 0);
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_whole_hand_cleanup_discard_is_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::DiscardToHandSize {
            player: P0,
            hand: vec![0, 1],
            count: 2, // the whole hand must go — no room to choose which cards
        });
        assert_eq!(
            game.forced_action(),
            Some(Intent::Discard {
                player: P0,
                cards: vec![0, 1],
            }),
        );
    }

    #[test]
    fn a_partial_cleanup_discard_is_a_real_choice() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::DiscardToHandSize {
            player: P0,
            hand: vec![0, 1, 2],
            count: 1, // pick one of three to pitch — a real decision
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_may_yes_no_is_never_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::MayYesNo {
            player: P0,
            source: 0,
            effect: Effect::DrawCards {
                count: Amount::Fixed(1),
            },
        });
        assert_eq!(
            game.forced_action(),
            None,
            "declining an optional trigger is always a legal choice",
        );
    }

    #[test]
    fn a_single_legal_target_is_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::ChooseTarget {
            player: P0,
            source: 0,
            effect: Effect::DrawCards {
                count: Amount::Fixed(1),
            },
            legal: vec![Target::Object(7)],
            optional: false,
        });
        assert_eq!(
            game.forced_action(),
            Some(Intent::ChooseTargets {
                player: P0,
                targets: vec![Target::Object(7)],
            }),
        );
    }

    #[test]
    fn two_legal_targets_are_a_real_choice() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::ChooseTarget {
            player: P0,
            source: 0,
            effect: Effect::DrawCards {
                count: Amount::Fixed(1),
            },
            legal: vec![Target::Object(7), Target::Object(8)],
            optional: false,
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_single_legal_target_is_not_forced_when_declinable() {
        // "Up to one" (min 0) is a real decision even with exactly one legal target — the player
        // may still decline (Killian, Decisive Mentor's "tap up to one target creature").
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::ChooseTarget {
            player: P0,
            source: 0,
            effect: Effect::DrawCards {
                count: Amount::Fixed(1),
            },
            legal: vec![Target::Object(7)],
            optional: true,
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_single_trigger_to_order_is_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::OrderTriggers {
            player: P0,
            source: 0,
            effects: vec![Effect::DrawCards {
                count: Amount::Fixed(1),
            }],
        });
        assert_eq!(
            game.forced_action(),
            Some(Intent::ChooseOrder {
                player: P0,
                order: vec![0],
            }),
        );
    }

    #[test]
    fn two_triggers_to_order_are_a_real_choice() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::OrderTriggers {
            player: P0,
            source: 0,
            effects: vec![
                Effect::DrawCards {
                    count: Amount::Fixed(1),
                },
                Effect::DrawCards {
                    count: Amount::Fixed(2),
                },
            ],
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_single_option_plain_edict_is_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::SacrificeEdict {
            player: P0,
            options: vec![3],
            keep_one: false,
            filter: PermanentFilter::of(TypeSet::CREATURE),
            remaining: vec![],
            controller: P0,
            source: 0,
            follow_up: &[],
        });
        assert_eq!(
            game.forced_action(),
            Some(Intent::ChooseSacrifices {
                player: P0,
                sacrifices: vec![3],
            }),
        );
    }

    #[test]
    fn a_keep_one_edict_is_never_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::SacrificeEdict {
            player: P0,
            options: vec![3],
            keep_one: true,
            filter: PermanentFilter::of(TypeSet::CREATURE),
            remaining: vec![],
            controller: P0,
            source: 0,
            follow_up: &[],
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn two_edict_options_are_a_real_choice() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::SacrificeEdict {
            player: P0,
            options: vec![3, 4],
            keep_one: false,
            filter: PermanentFilter::of(TypeSet::CREATURE),
            remaining: vec![],
            controller: P0,
            source: 0,
            follow_up: &[],
        });
        assert_eq!(game.forced_action(), None);
    }
}

#[cfg(test)]
mod refresh_actions_tests {
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
    fn preserves_action_id_when_kind_unchanged() {
        let mut game = Game::new();
        let forest = game.spawn_in_hand(P0, forest());
        game.refresh_actions();
        let id_before = game
            .actions
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    MeaningfulAction::PlayLand {
                        card,
                        zone: Zone::Hand
                    } if card == forest
                )
            })
            .expect("land drop listed")
            .id;
        game.refresh_actions();
        let id_after = game
            .actions
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    MeaningfulAction::PlayLand {
                        card,
                        zone: Zone::Hand
                    } if card == forest
                )
            })
            .expect("land drop still listed")
            .id;
        assert_eq!(id_before, id_after);
    }

    #[test]
    fn preserves_distinct_ids_per_action_kind() {
        let mut game = Game::new();
        game.fund_mana(P0);
        let forest = game.spawn_in_hand(P0, forest());
        let bear = game.spawn_in_hand(
            P0,
            CardDef {
                name: "Bear",
                id: "",
                default_print: "",
                cost: Cost {
                    generic: 2,
                    ..Cost::FREE
                },
                kind: CardKind::Creature {
                    power: 2,
                    toughness: 2,
                    also: TypeSet::NONE,
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
            },
        );
        game.refresh_actions();
        let land_id = game
            .actions
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    MeaningfulAction::PlayLand {
                        card,
                        zone: Zone::Hand
                    } if card == forest
                )
            })
            .expect("land drop")
            .id;
        let cast_id = game
            .actions
            .iter()
            .find(|a| {
                matches!(
                    a.kind,
                    MeaningfulAction::Cast {
                        card,
                        zone: Zone::Hand
                    } if card == bear
                )
            })
            .expect("creature cast")
            .id;
        assert_ne!(land_id, cast_id);
        game.refresh_actions();
        assert_eq!(
            land_id,
            game.actions
                .iter()
                .find(|a| matches!(
                    a.kind,
                    MeaningfulAction::PlayLand {
                        card,
                        zone: Zone::Hand
                    } if card == forest
                ))
                .unwrap()
                .id
        );
        assert_eq!(
            cast_id,
            game.actions
                .iter()
                .find(|a| matches!(
                    a.kind,
                    MeaningfulAction::Cast {
                        card,
                        zone: Zone::Hand
                    } if card == bear
                ))
                .unwrap()
                .id
        );
    }

    #[test]
    fn clears_actions_while_a_pending_choice_is_up() {
        let mut game = Game::new();
        game.spawn_in_hand(P0, forest());
        game.refresh_actions();
        assert!(!game.actions.is_empty());
        game.pending_choice = Some(PendingChoice::MayYesNo {
            player: P0,
            source: 0,
            effect: Effect::DrawCards {
                count: Amount::Fixed(1),
            },
        });
        game.refresh_actions();
        assert!(
            game.actions.is_empty(),
            "choice answers replace the action list"
        );
    }
}
