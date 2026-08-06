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
//! `docs/fidelity/` (per-deck increments via fidelity-grind), `docs/AGENT_NAVIGATION.md`, and `docs/CR_INDEX.md`
//! for vocabulary, gaps, and CR lookup.

/// The card-DSL vocabulary — `CardDef`, `Effect` and its families, filters, mana, triggers —
/// is defined by the `cards` crate (which also owns the TOML surface, its JSON Schema, and the
/// generated `DSL_REFERENCE.md`). The engine implements the rules logic around those types and
/// re-exports them so `engine::Effect` and friends keep resolving for `schema` / `server`.
pub use cards::types::*;
pub use cards::{
    CardId, CardToml, card_def, install_token_defs, intern_card_def, interned_len, token_def,
};

mod amount;
mod apply;
mod cast;
mod characteristics;
mod characteristics_cache;
mod combat;
mod core;
mod effects;
mod hand_smooth;
mod message;
mod mulligan;
mod pending;
mod pipeline;
mod playable;
mod priority;
mod query;
mod replacements;
mod resolution;
pub mod rng;
mod spawn;
mod state;
mod triggers;
mod types;
mod zones;

pub use crate::core::GameOutcome;
pub use message::*;
pub use mulligan::hand_size_after_mulligans;
/// What a would-be counter placement is aimed at — see [`replacements::CounterRecipient`].
pub(crate) use replacements::CounterRecipient;
/// Shared Effect-resolution context for [`Game::run`] / [`Game::run_sequence`].
pub(crate) use resolution::ResolveCtx;
/// All-players search fan-out continuation state (Veteran Explorer) — see
/// [`resolution::SearchFanout`].
pub(crate) use resolution::SearchFanout;
/// The one draw funnel's parked batch and its follow-up rider — see [`Game::draw_with_replacements`].
pub(crate) use resolution::{DrawAfter, DrawBatch};
pub use state::{ControlCondition, DyingCreatureStats};
pub(crate) use state::{Modifier, ModifierDuration, ModifierKind};
pub use types::*;

/// Keyword-trigger obligations queued outside ordinary triggered abilities and drained when the
/// normal pending-trigger queue empties.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Obligation {
    Echo { permanent: ObjectId },
    Recover { card: ObjectId },
    CumulativeUpkeep { permanent: ObjectId },
}

impl Obligation {
    pub(crate) fn object(self) -> ObjectId {
        match self {
            Self::Echo { permanent } | Self::CumulativeUpkeep { permanent } => permanent,
            Self::Recover { card } => card,
        }
    }
}

/// Post-resolution destination override for the spell currently finishing resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FinishPolicy {
    ExileWithTimeCounters(u32),
    TuckLibraryBottom,
    Exile,
}

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
    /// Keyword-trigger obligations queued outside ordinary trigger groups: Echo's pay-or-sacrifice
    /// upkeep choice, Recover's pay-or-exile graveyard choice, and Cumulative upkeep's
    /// age-counter-then-pay-or-sacrifice choice. [`Game::place_pending_triggers`] drains this one
    /// at a time after ordinary triggers, preserving today's priority: every Echo first, then
    /// every Recover, then every Cumulative upkeep.
    pub(crate) pending_obligations: Vec<Obligation>,
    /// A decision the engine is blocked on until the active chooser answers.
    pub(crate) pending_choice: Option<PendingChoice>,
    /// Deferred resolution resume riders (clash scry, sequence tail, demonstrate opponent copy,
    /// spell finish) — drained by [`Game::resume_deferred_sequence`] once
    /// [`pending_choice`](Self::pending_choice) clears. See [`resolution::ResumeState`].
    pub(crate) resume: resolution::ResumeState,
    /// Whether the current resolution's [`Effect::Dig(DigEffect::Clash)`] was won by its controller (CR 701.22d),
    /// read by a following [`Condition::WonClash`] step. Resolution-scoped, not a persistent board
    /// fact: each clash overwrites it and only a same-resolution `conditional` reads it, so it is
    /// never carried meaningfully between resolutions (see [`Condition::WonClash`]).
    pub(crate) clash_won: bool,
    /// Permanents that skip their controller's next untap step (Pollen Lullaby's win rider —
    /// [`Effect::Misc(MiscEffect::SkipNextUntapOpponentCreatures)`]). Each id is consumed the next time that
    /// permanent's controller reaches their untap step (see [`Game::advance_step`]'s `Untap` arm),
    /// whether or not it was tapped.
    pub(crate) skip_next_untap: Vec<ObjectId>,
    /// Extra turns owed, oldest first (Time Walk — [`Effect::Misc(MiscEffect::TakeExtraTurn)`]).
    /// [`Game::advance_step`] pops the *last* entry as a turn ends, so of several extra turns
    /// created in one turn the most recent is taken first (CR 500.7), and only an empty queue
    /// hands the turn to the next player in the rotation.
    pub(crate) extra_turns: Vec<PlayerId>,
    /// The current combat's attackers, blocks, and orderings (empty outside combat).
    pub(crate) combat: CombatState,
    /// Goad and other combat-adjacent state lifted off `Permanent` so it stays `Copy`.
    pub(crate) combat_extras: state::CombatExtras,
    /// Active play/control permissions lifted off `Card`/`Permanent` so they stay `Copy`.
    pub(crate) play_permissions: state::PlayPermissions,
    /// Monotonic source of control-change timestamps (CR 800.4a — "the most recent control-changing
    /// effect wins"). Stamped onto each control override / control Aura as it takes hold and read by
    /// [`Game::controller_of`] to rank several control effects on one permanent. Never reset, so an
    /// earlier steal always compares older than a later one for the game's lifetime.
    pub(crate) next_control_timestamp: u64,
    /// Monotonic source of continuous-effect timestamps (CR 613.7 — later same-layer effects win
    /// absent dependency). Stamped when a permanent starts applying a static continuous effect and
    /// when a runtime base/type set lands on a permanent.
    pub(crate) next_continuous_timestamp: u64,
    /// Sourced counter/EOT-boost batches for the Alt-inspect mod ledger.
    pub(crate) modifier_provenance: state::ModifierProvenance,
    /// Once-per-turn activation/trigger caps, reset at each untap step.
    pub(crate) once_per_turn: state::OncePerTurnLimits,
    /// Links between exiling sources and the cards they exiled.
    pub(crate) exile_links: state::ExileLinks,
    /// Pending CR 603.7 delayed triggered abilities not yet placed on the stack.
    pub(crate) delayed_triggers: state::DelayedTriggers,
    /// Master seed for derive-per-op random streams (BLAKE3 keyed by player + iteration).
    pub(crate) master_seed: [u8; 32],
    /// Whether the game is in the pre-game simultaneous mulligan phase.
    pub(crate) mulliganing: bool,
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
    /// `(source, victim)` pairs recorded at every creature-damage choke this turn — combat or
    /// noncombat alike (CR 510.2 / 120.3/506) — feeding
    /// [`Trigger::CreatureDealtDamageByThisDies`] (Vampiric Dragon: "a creature dealt damage by
    /// this creature this turn dies"). `victim` is the object id it held *at damage time*; a
    /// creature that left and re-entered the battlefield between the damage and its death is a
    /// new object (CR 400.7) and rightly won't match. Reset alongside
    /// [`permanents_died_this_turn`](Self::permanents_died_this_turn) at every Untap step.
    pub(crate) damaged_this_turn: Vec<(ObjectId, ObjectId)>,
    /// `(source, recipient, amount)` rows for every point of damage actually dealt this turn (CR
    /// 120.3), recorded in `apply` as each damage event lands so prevention and redirection have
    /// already had their say. The history behind `Amount::DamageDealtToThisThisTurnByOthersNamed`
    /// (Blazing Effigy) and `Condition::DealtDamageToOpponentThisTurn` (Whirling Dervish). Unlike
    /// [`damaged_this_turn`](Self::damaged_this_turn) — which answers "did this hit that at all" —
    /// these rows keep the amount and the source, so several hits from the same source are several
    /// rows. Reset at every Untap step.
    pub(crate) damage_dealt_this_turn: Vec<(ObjectId, Target, i32)>,
    /// Every hand-object id drawn this turn, in draw order — Sylvan Library's "choose two cards in
    /// your hand **drawn this turn**". Pushed by `Event::CardDrawn`, reset at every Untap step.
    /// Ids, not counts, because the card that matters is a specific object: a card drawn and then
    /// put back is a new object when it returns (CR 400.7) and stops matching, which is right.
    pub(crate) drawn_this_turn: Vec<ObjectId>,
    /// `(looker, card)` pairs for every hand card a player has privately looked at (CR 701.20 —
    /// Glasses of Urza). Read only by the wire redaction layer, which itemizes a hand card to its
    /// owner and to anyone holding a pair for it. Never cleared: a card that leaves the hand is a
    /// new object when it comes back (CR 400.7), so a stale pair can't re-expose it, and cards
    /// drawn after the look were never in it — which is exactly what "look at" means, as against a
    /// standing window onto the hand.
    pub(crate) hand_cards_seen: Vec<(PlayerId, ObjectId)>,
    /// `(card, owner)` for every hand card that came back from a death replacement which shuts it
    /// down for a turn cycle — Firestorm Phoenix's "Until that player's next turn, that player
    /// plays with that card revealed in their hand and can't play it." Both halves of that clause
    /// read this one list: [`Game::has_seen_hand_card`] treats an entry as a standing window onto
    /// the card for every seat, and the cast path refuses it. Cleared for the active player at
    /// their untap step, the same "until your next turn" boundary
    /// [`CombatExtras::repelled_until_next_turn`] uses.
    pub(crate) revealed_unplayable_until_next_turn: Vec<(ObjectId, PlayerId)>,
    /// "Prevent the next N damage that would be dealt to `target` this turn" (CR 615 — Healing
    /// Salve, Samite Healer, Conservator): each entry is a *consumable* shield, `(what it
    /// protects, how many points are left on it)`. Spent at the two damage chokes
    /// ([`Game::creature_damage_events`] and [`Game::player_damage_events`]) by
    /// [`Game::spend_prevention_shields`], which is the only reader; the spend rides
    /// [`Event::DamagePrevented`] so the decrement happens in `apply` like every other state
    /// change. Cleared at the next Untap step, the same "this turn" boundary as
    /// [`CombatExtras::combat_damage_prevention_shields`](state::CombatExtras::combat_damage_prevention_shields)
    /// — that shield's uncountable "prevent all combat damage" twin.
    ///
    /// ponytail: several shields on one target are spent in the order they were created, not in
    /// the order their controller chooses (CR 615.8). Same total prevented either way unless a
    /// card reads *which* shield paid, and none in the pool does.
    pub(crate) damage_prevention_shields: Vec<state::PreventionShield>,
    /// "Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent
    /// the next 1 damage that would be dealt to that permanent or player this turn" (Guardian
    /// Angel): a standing *offer* rather than a shield — nothing is prevented until someone pays.
    /// Enumerated by [`Game::legal_actions`] as a
    /// [`MeaningfulAction::PayStandingPrevention`](crate::MeaningfulAction) and taken through
    /// [`Intent::TakeAction`](crate::Intent), so the offer reaches the client on the same action
    /// list every ability does; paying pushes an ordinary entry onto
    /// [`damage_prevention_shields`](Self::damage_prevention_shields) above. Runtime orchestration
    /// state like the shields, cleared at the same next-Untap "this turn" boundary.
    pub(crate) standing_preventions: Vec<state::StandingPrevention>,
    /// Resolution-local "this way" scratch (DestroyAll / ExileAll / mill / council / edict riders).
    /// Not turn-scoped — see [`resolution::ResolutionFrame`].
    pub(crate) resolution_frame: resolution::ResolutionFrame,
    /// Memoized effective P/T and keywords per object; invalidated on relevant [`Event`]s.
    pub(crate) characteristics_cache: characteristics_cache::CharacteristicsCacheCell,
    /// `(target, source)` pairs where `target` has gained `source`'s other abilities until end of
    /// turn (CR 702.166 Backup — Guardian Scalelord). The granted set is read live off `source`'s
    /// `CardDef.abilities`/`keywords` (minus the granting ability itself), so it tracks the
    /// source's current characteristics; cleared at cleanup by [`Event::GrantedAbilitiesEnded`].
    pub(crate) abilities_granted_until_eot: Vec<(ObjectId, ObjectId)>,
    /// Additional +1/+1 counters a specific about-to-resolve spell must enter with, keyed by the
    /// spell's stack object id. Captured at cast payment by Opal Palace's
    /// [`Trigger::SpendManaToCast`] rider ([`Effect::Counters(CountersEffect::CommanderEntersWithBonusCounters)`]) and drained
    /// by [`Game::resolve_spell`] when the spell becomes a permanent (CR 601.2 — the "enters with"
    /// rider is set before resolution). Engine-internal scratch, never wire-mirrored.
    pub(crate) pending_enter_bonus_counters: Vec<(ObjectId, u32)>,
    /// Time counters (CR 702.62 — suspend) on cards in exile, each `(exile object, count)`. Kept
    /// off the `Copy` [`Object::Card`] (an exiled card carries no counter field, unlike a
    /// [`Permanent`]); an entry is created as the card is suspended and dropped when the last
    /// counter is removed (the card becomes castable) — see [`Game::exile_time_counters`].
    pub(crate) exile_time_counters: Vec<(ObjectId, u32)>,
    /// Post-resolution destination override for the spell currently finishing resolution. Set by
    /// self-move zone effects during that spell's own resolution and consumed immediately by
    /// [`Game::finish_instant_sorcery_resolution`], so the one-spell-at-a-time resolution model
    /// is enough lifetime management.
    pub(crate) resolution_finish: Option<FinishPolicy>,
    /// The game ended in a draw (CR 104.4 — Divine Intervention). Not a loss for anyone, so it
    /// can't be read off the players' `lost` flags; see [`Game::outcome`].
    pub(crate) drawn: bool,
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
    #[tracing::instrument(
        name = "engine.submit",
        level = "debug",
        skip(self, intent),
        fields(accepted = tracing::field::Empty)
    )]
    pub fn submit(&mut self, intent: Intent) -> Result<Vec<Event>, Reject> {
        let result = self.submit_inner(intent);
        tracing::Span::current().record("accepted", result.is_ok());
        result
    }

    fn submit_inner(&mut self, intent: Intent) -> Result<Vec<Event>, Reject> {
        // CR 104.4b: a draw ends the game immediately for every player still in it, so nothing
        // further is legal — not even conceding.
        // ponytail: reuses `WrongTiming` rather than minting a `Reject::GameOver`, which would need
        // a proto/client arm; add the dedicated variant when the client shows an end-of-game state.
        if self.drawn {
            return Err(Reject::WrongTiming);
        }

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

        if self.mulliganing
            && !matches!(
                intent,
                Intent::KeepHand { .. }
                    | Intent::Mulligan { .. }
                    | Intent::Concede { .. }
                    | Intent::TakeAction { .. }
            )
        {
            return Err(Reject::Mulliganing);
        }

        let mut events = if pending::is_answer(&intent) {
            pending::answer(self, intent)?
        } else {
            match intent {
                Intent::KeepHand { player } => self.keep_hand(player)?,
                Intent::Mulligan { player } => self.take_mulligan(player)?,
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
                    multikicker_count,
                    alternative_cost,
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
                    multikicker_count,
                    alternative_cost,
                )?,
                Intent::PlayLand { player, object } => self.play_land(player, object)?,
                Intent::Cycle {
                    player,
                    card,
                    sacrifice,
                } => self.cycle(player, card, sacrifice)?,
                Intent::ActivateHandAbility {
                    player,
                    card,
                    index,
                } => self.activate_hand_ability(player, card, index)?,
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
                Intent::CastSplitHalf {
                    player,
                    source,
                    half,
                    target,
                    x,
                } => self.cast_split_half(player, source, half, target, x)?,
                Intent::CastBestow {
                    player,
                    object,
                    target,
                } => self.cast_bestow(player, object, target)?,
                Intent::CastFaceDown { player, card } => self.cast_face_down(player, card)?,
                Intent::TapForMana { player, object } => self.tap_for_mana(player, object)?,
                Intent::ChannelColorlessMana { player } => self.channel_colorless_mana(player)?,
                Intent::Concede { player } => {
                    let mut events = self.concede(player);
                    self.finish_mulligans_if_all_kept(&mut events);
                    events
                }
                Intent::ActivateAbility {
                    player,
                    object,
                    ability_index,
                    target,
                    sacrifice,
                    discard_cost,
                    x,
                } => self.activate_ability(
                    player,
                    object,
                    ability_index,
                    target,
                    sacrifice,
                    discard_cost,
                    x,
                )?,
                Intent::DeclareAttackers { player, attackers } => {
                    self.declare_attackers(player, &attackers)?
                }
                Intent::DeclareAttackersInBands {
                    player,
                    attackers,
                    bands,
                } => self.declare_attackers_in_bands(player, &attackers, &bands)?,
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
        attackers: Vec<(ObjectId, Defender)>,
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
            MeaningfulAction::KeepHand => self.keep_hand(player),
            MeaningfulAction::Mulligan => self.take_mulligan(player),
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
                0,
                false,
                playable::CastPlayKind::OneClick,
            ),
            MeaningfulAction::Activate { source, ability } => self.activate_ability(
                player,
                source,
                ability,
                target,
                sacrifice,
                discard_cost.to_vec(),
                x,
            ),
            MeaningfulAction::Cycle { card } => {
                self.cycle(player, card, sacrifice.first().copied())
            }
            MeaningfulAction::ActivateHandAbility { card, index } => {
                self.activate_hand_ability(player, card, index)
            }
            MeaningfulAction::Suspend { card } => self.suspend(player, card),
            MeaningfulAction::Encore { card } => self.encore(player, card),
            MeaningfulAction::TurnFaceUp { permanent } => self.turn_face_up(player, permanent),
            MeaningfulAction::CastPrepared { source } => {
                self.cast_prepared(player, source, target, x)
            }
            MeaningfulAction::CastSplitHalf { card, half } => {
                self.cast_split_half(player, card, half, target, x)
            }
            MeaningfulAction::CastFaceDown { card } => self.cast_face_down(player, card),
            MeaningfulAction::PayStandingPrevention { index } => {
                self.pay_standing_prevention(player, index)
            }
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
            let meaningful = self
                .meaningful_actions(player)
                .into_iter()
                .map(|k| (k, false));
            let mana_only = self
                .paid_mana_activates(player)
                .into_iter()
                .map(|k| (k, true));
            for (kind, mana_only) in meaningful.chain(mana_only) {
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
                self.actions.push(LegalAction {
                    id,
                    player,
                    kind,
                    mana_only,
                });
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
            effect: Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            }),
            resume: crate::MayYesNoResume::Default,
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
            controller: P0,
            source: 0,
            effect: Some(Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            })),
            legal: vec![Target::Object(7)],
            count: TargetCount::default(),
            clause: 0,
            target: None,
            x: 0,
            spent_mana: [0; 6],
            activated: false,
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
            controller: P0,
            source: 0,
            effect: Some(Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            })),
            legal: vec![Target::Object(7), Target::Object(8)],
            count: TargetCount::default(),
            clause: 0,
            target: None,
            x: 0,
            spent_mana: [0; 6],
            activated: false,
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
            controller: P0,
            source: 0,
            effect: Some(Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            })),
            legal: vec![Target::Object(7)],
            count: TargetCount {
                min: 0,
                max: 1,
                ..TargetCount::default()
            },
            clause: 0,
            target: None,
            x: 0,
            spent_mana: [0; 6],
            activated: false,
        });
        assert_eq!(game.forced_action(), None);
    }

    #[test]
    fn a_single_trigger_to_order_is_forced() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::OrderTriggers {
            player: P0,
            source: 0,
            effects: vec![Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            })],
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
                Effect::Draw(DrawEffect::Cards {
                    who: PlayerSet::You,
                    count: Amount::Fixed(1),
                }),
                Effect::Draw(DrawEffect::Cards {
                    who: PlayerSet::You,
                    count: Amount::Fixed(2),
                }),
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
            count: 1,
            floor: None,
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
            count: 1,
            floor: None,
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
            count: 1,
            floor: None,
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
            cast_only_after_upkeep: false,
            cast_only_after_combat: false,
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
                cast_only_after_upkeep: false,
                cast_only_after_combat: false,
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
            effect: Effect::Draw(DrawEffect::Cards {
                who: PlayerSet::You,
                count: Amount::Fixed(1),
            }),
            resume: crate::MayYesNoResume::Default,
        });
        game.refresh_actions();
        assert!(
            game.actions.is_empty(),
            "choice answers replace the action list"
        );
    }
}
