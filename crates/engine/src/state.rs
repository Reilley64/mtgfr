//! Cohesive `Game` state buckets — `Vec` fields lifted off `Copy` zone objects
//! (`Permanent`, `Card`, `Effect`) so those types stay by-value throughout the engine.
//!
//! Side state for goad (CR 701.38), delayed triggers (CR 603.7), exile links,
//! once-per-turn flags, until-EOT control (CR 720), and inspect-ledger provenance.

use crate::{CardDef, Effect, Keyword, ObjectId, PlayerId, SpellFilter, Step};

/// The CR 611.2b duration condition scoping a control-changing effect (Rubinia Soulsinger's "for
/// as long as you control Rubinia and Rubinia remains tapped"). Stored alongside the override in
/// [`PlayPermissions::conditioned_control_overrides`] and re-evaluated as a state-based check
/// ([`Game::check_conditioned_control_reversions`](crate::Game::check_conditioned_control_reversions));
/// the moment it stops holding, the override is dropped and control reverts. "You control the
/// source" is checked against the override's own controller (the thief), so it isn't a field here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlCondition {
    /// The permanent whose continued control (by the thief) and tapped state sustain the steal
    /// (Rubinia herself). When it leaves the battlefield the condition fails.
    pub source: ObjectId,
    /// Whether `source` must remain tapped for the steal to hold ("Rubinia remains tapped").
    pub needs_tapped: bool,
}

/// Combat-adjacent state stored outside `Permanent` because `Permanent` is `Copy`.
#[derive(Clone, Default)]
pub(crate) struct CombatExtras {
    /// Goad (CR 701.38): each entry is (goaded creature, the player who goaded it, source card
    /// name for the inspect ledger). A creature can appear more than once — it may be goaded by
    /// several players. Each entry ends at the start of the goader's next turn (cleared in that
    /// player's untap turn-based action).
    pub goaded: Vec<(ObjectId, PlayerId, &'static str)>,
    /// "Attacks … this turn if able" (CR 508.1a — Furygale Flocking's minted tokens): each entry
    /// is `(creature, the defender it must attack)`. Enforced as a requirement in
    /// [`Game::declare_attackers`], mirroring the goad requirement loop. Cleared at the next
    /// turn's Untap step (the "this turn" boundary).
    /// ponytail: cleared at Untap rather than the creating turn's cleanup — nothing reads
    /// `must_attack` between cleanup and the next Untap, so it's CR-equivalent to end-of-turn
    /// expiry, the same reasoning `pending_next_cast`'s turn-boundary clear uses. (CR 508.1a, CR 502.1, CR 514.3)
    pub must_attack: Vec<(ObjectId, PlayerId)>,
    /// "Prevent all combat damage that would be dealt to you this turn" (CR 615 — Inkshield):
    /// each entry is `(the protected player, the token minted per point of combat damage
    /// prevented)`. Consulted at the combat-damage-to-a-player choke
    /// ([`Game::damage_player`](crate::Game::damage_player)): a matching entry cancels the life
    /// loss and mints one copy of `token` per prevented point. Cleared at the next turn's Untap
    /// step (the "this turn" boundary).
    /// ponytail: "this turn" modeled as "until the next untap" — combat is always within the
    /// turn, so a combat-only shield cleared at Untap is behavior-exact (same turn-boundary idiom
    /// `must_attack`/`pending_next_cast` use). Combat-damage-to-a-*player* only — the N-point
    /// (Inkshield) shape stays this-turn/per-player; the permanent per-source shape (Guard
    /// Gomazoa, Fog Bank, #220) is a separate static, [`Effect::Static(StaticEffect::PreventCombatDamage)`],
    /// scanned live off the permanent rather than stored here.
    pub combat_damage_prevention_shields: Vec<(PlayerId, CardDef)>,
    /// "Prevent all combat damage that would be dealt this turn" (Moment's Peace, #150 — the
    /// table-wide, no-token scope generalization of [`combat_damage_prevention_shields`](Self::combat_damage_prevention_shields)'s
    /// per-player Inkshield shield): every player's combat damage, to creatures and to players
    /// alike, is prevented for the rest of the turn. Consulted at all three combat-damage chokes
    /// ([`Game::deal_creature_damage`](crate::Game::deal_creature_damage) for fight/single-blocker
    /// damage, [`Game::assign_attacker_damage`](crate::Game::assign_attacker_damage) for a blocked
    /// attacker's own inlined damage-marking path, and [`Game::damage_player`](crate::Game::damage_player)).
    /// Cleared at the next turn's Untap step, the same "this turn" idiom as
    /// `combat_damage_prevention_shields`.
    pub prevent_all_combat_damage_this_turn: bool,
}

/// Active play and control permissions stored outside `Card`/`Permanent` so they stay `Copy`.
#[derive(Clone, Default)]
pub(crate) struct PlayPermissions {
    /// Active one-shot until-end-of-turn control changes (CR 720), each entry (the controlled
    /// object, its new controller, source card name) — Besmirch's steal. Read by
    /// [`Game::controller_of`](crate::Game::controller_of) and cleared per-object at cleanup by
    /// [`Event::ControlEndedUntilEndOfTurn`](crate::Event::ControlEndedUntilEndOfTurn).
    /// Each entry `(the controlled object, its new controller, source card name, the control
    /// timestamp)` — Besmirch's steal. The timestamp is [`Game::next_control_timestamp`](crate::Game),
    /// stamped when the entry is recorded, so [`Game::controller_of`](crate::Game::controller_of)
    /// can pick the most recent control-changing effect (CR 800.4a) when several apply to one
    /// permanent. Cleared per-object at cleanup by
    /// [`Event::ControlEndedUntilEndOfTurn`](crate::Event::ControlEndedUntilEndOfTurn).
    pub control_overrides: Vec<(ObjectId, PlayerId, &'static str, u64)>,
    /// Permanent control changes with no stated duration (CR 720 — Entrancing Melody), each
    /// entry `(the controlled object, its new controller, the control timestamp)`. Unlike
    /// [`control_overrides`](Self::control_overrides), never cleared at cleanup;
    /// [`Game::controller_of`](crate::Game::controller_of) ranks it against the other registries
    /// by timestamp (CR 800.4a).
    pub permanent_control_overrides: Vec<(ObjectId, PlayerId, u64)>,
    /// Condition-scoped control changes (CR 611.2b — Rubinia Soulsinger's "for as long as you
    /// control Rubinia and Rubinia remains tapped"), each entry `(the controlled object, its new
    /// controller, the sustaining condition, the control timestamp)`. Unlike
    /// [`control_overrides`](Self::control_overrides) (cleanup) and
    /// [`permanent_control_overrides`](Self::permanent_control_overrides) (never), an entry here is
    /// dropped the moment its [`ControlCondition`] stops holding, detected as a state-based check
    /// ([`Game::check_conditioned_control_reversions`](crate::Game::check_conditioned_control_reversions))
    /// that emits [`Event::ConditionedControlEnded`](crate::Event::ConditionedControlEnded). Read
    /// by [`Game::controller_of`](crate::Game::controller_of), ranked against the other registries
    /// by its timestamp (CR 800.4a).
    pub conditioned_control_overrides: Vec<(ObjectId, PlayerId, ControlCondition, u64)>,
    /// Control-changing Aura (CR 720 — [`Effect::Static(StaticEffect::ControlAttached)`](crate::Effect::Static(StaticEffect::ControlAttached)))
    /// timestamps, each `(the Aura object, the control timestamp it attached at)`. The Aura path
    /// keeps no controller entry (the controller is read live as the Aura's owner off its live
    /// attachment in [`Game::control_aura`](crate::Game::control_aura)); this only records *when*
    /// it took hold, so [`Game::controller_of`](crate::Game::controller_of) can rank it against the
    /// three override registries by recency (CR 800.4a). Stamped in
    /// [`Event::AttachedTo`](crate::Event::AttachedTo)'s control-gain branch. A stale entry for a
    /// detached/dead Aura is harmless — it's only consulted for an Aura currently attached to the
    /// queried host (and object ids retire on zone change, CR 400.7), the same rationale the three
    /// override registries rely on.
    pub aura_control_timestamps: Vec<(ObjectId, u64)>,
    /// Impulse draw (CR 118.6): each entry is `(an exiled card, the player who may play it,
    /// extended)` — the play permission granted by
    /// [`Effect::Mill(MillEffect::ExileTopMayPlay)`](crate::Effect::Mill(MillEffect::ExileTopMayPlay)). A plain entry (`extended =
    /// false`) expires at the very next cleanup. An `extended` entry (Atsushi, the Blazing Sky's
    /// `until_next_turn` mode) is shielded from that cleanup until it arms — flips to
    /// `extended = false` — at its player's own next untap
    /// ([`Event::PlayFromExilePermissionArmed`](crate::Event::PlayFromExilePermissionArmed)), then
    /// clears like a normal entry at that turn's cleanup.
    pub play_from_exile: Vec<(ObjectId, PlayerId, bool)>,
    /// Intet, the Dreamer's "you may play that card without paying its mana cost for as long as
    /// Intet remains on the battlefield" (CR 118.5 plus a live, source-scoped duration): each entry
    /// is `(an exiled card, the player who may play it, the granting permanent)`. Unlike
    /// [`play_from_exile`](Self::play_from_exile) there is no cleanup expiry — the duration is read
    /// live by [`Game::may_play_from_exile_free_while_source`](crate::Game), which requires
    /// `source` to still be on the battlefield, so an entry for a dead source simply stops
    /// matching. Granted by [`Effect::Mill(MillEffect::ExileTopMayPlay)`](crate::Effect::Mill(MillEffect::ExileTopMayPlay))'s
    /// `free_while_source` mode.
    pub play_from_exile_free_while_source: Vec<(ObjectId, PlayerId, ObjectId)>,
    /// Free-cast-from-exile (CR 118.5, "without paying its mana cost") — each entry is `(an
    /// exiled card, the player who may cast it for free)`, granted by
    /// [`Effect::Dig(DigEffect::CastExiledWithThisFree)`](crate::Effect::Dig(DigEffect::CastExiledWithThisFree)) (Quintorius,
    /// Loremaster). Distinct from [`play_from_exile`](Self::play_from_exile), which permits
    /// playing from exile but still charges the normal cost. Expires unconditionally at the next
    /// cleanup ([`Event::CastFromExileFreeEnded`](crate::Event::CastFromExileFreeEnded)) — no
    /// `until_next_turn` extension exists for this permission (no card needs one yet).
    pub cast_from_exile_free: Vec<(ObjectId, PlayerId)>,
    /// Quintorius, Loremaster's replacement rider (CR 614.6): "If that spell would be put into a
    /// graveyard, put it on the bottom of its owner's library instead." Each entry is the
    /// exile-zone id the free-cast permission was granted for — the same id at rest in
    /// [`cast_from_exile_free`](Self::cast_from_exile_free), pushed by
    /// [`Event::CastFromExileFreeBottomsLibraryOnLeave`](crate::Event::CastFromExileFreeBottomsLibraryOnLeave)
    /// at the same site as the free-cast grant. Casting the card mints it a fresh stack object id
    /// (CR 400.7), so the two stack→graveyard chokes
    /// ([`Game::finish_instant_sorcery_resolution`](crate::Game::finish_instant_sorcery_resolution),
    /// [`Game::counter_spell`](crate::Game::counter_spell)) match an entry by following
    /// [`Game::current_id`](crate::Game::current_id) from the stored exile id forward, not by a
    /// raw id comparison. Cleared unconditionally alongside `cast_from_exile_free` at cleanup —
    /// same CR 118.5 "this turn" expiry, no separate lifetime.
    pub stack_object_bottoms_library_on_leave: Vec<ObjectId>,
    /// Adventure (CR 715.3d): each entry is `(a card exiled "on an adventure", its owner)` — the
    /// owner may cast that card (its creature front face) from exile at its normal cost. Unlike
    /// [`play_from_exile`](Self::play_from_exile), this **never expires** ("You may cast the
    /// creature later from exile" has no turn limit); the entry simply goes stale once the card
    /// leaves exile (its object id changes), and is dropped when it's cast from exile (see the
    /// [`Event::SpellCast`](crate::Event::SpellCast) handler). Consulted by
    /// [`Game::may_play_from_exile`](crate::Game::may_play_from_exile).
    pub on_adventure: Vec<(ObjectId, PlayerId)>,
    /// Adventure spells currently on the stack, each `(spell object, the creature front face to
    /// restore to exile)`. Kept off the `Copy` [`Spell`](crate::Spell) struct (a `CardDef` by value
    /// would double `Spell`'s size). Pushed by
    /// [`Event::AdventureSpellCast`](crate::Event::AdventureSpellCast), read + dropped when the
    /// spell finishes ([`Event::ExiledOnAdventure`](crate::Event::ExiledOnAdventure)).
    /// ponytail: a *countered* adventure spell (which goes to the graveyard, not exile) leaves its
    /// entry here stale — keyed on a now-dead spell id it can never re-match, so it's harmless. No
    /// pool card counters an adventure; drop the entry in `counter_spell` if one ever does.
    pub adventure_fronts: Vec<(ObjectId, CardDef)>,
    /// Split-card halves currently on the stack, each `(the half's spell object, the fused card to
    /// restore)` — the same off-`Copy` shape as [`adventure_fronts`](Self::adventure_fronts). Only
    /// the cast half is on the stack (CR 709.4a); in every other zone the object is the whole split
    /// card again (CR 709.4), so [`Game::create_object`](crate::Game) swaps the fused def back in
    /// on the way out — one choke covering resolution, being countered, and a tuck alike.
    /// ponytail: entries are never removed. Object ids retire on zone change (CR 400.7), so a stale
    /// entry can never re-match; drop it in the stack-exit paths if a game ever runs long enough
    /// for the list's length to matter.
    pub split_halves_on_stack: Vec<(ObjectId, CardDef)>,
}

/// Transient per-batch scratch for trigger enqueueing — not event-sourced.
#[derive(Clone, Default)]
pub(crate) struct BatchTriggerScratch {
    /// Owners who lost one or more graveyard cards in the event batch currently being applied —
    /// the accumulator behind [`Trigger::CardsLeaveYourGraveyard`](crate::Trigger::CardsLeaveYourGraveyard) (Quintorius Field Historian).
    /// Pushed by [`Game::create_object`](crate::Game::create_object)'s graveyard-exit branch (the same choke point that sets
    /// [`Player::card_left_graveyard_this_turn`](crate::Player::card_left_graveyard_this_turn)), drained (deduped) and cleared by
    /// [`Game::enqueue_triggers`](crate::Game::enqueue_triggers) at the end of every batch.
    /// ponytail: transient per-batch scratch, not event-sourced — mirrors
    /// [`Player::card_left_graveyard_this_turn`](crate::Player::card_left_graveyard_this_turn), which isn't event-sourced either.
    ///
    /// Each entry is `(owner, graveyard-object id that left)`; the ids are the CR 603.10a
    /// last-known information behind Spirit of Resilience's "become a copy … from among those
    /// cards" — [`Game::enqueue_triggers`](crate::Game::enqueue_triggers) threads each owner's
    /// ids into [`TriggerContext::cards_left_graveyard`](crate::TriggerContext) at placement.
    pub graveyard_exits_this_batch: Vec<(PlayerId, ObjectId)>,
    /// Owners whose library and/or graveyard lost one or more cards to **exile** this event
    /// batch — the accumulator behind
    /// [`Trigger::CardsExiledFromYourLibraryOrGraveyard`](crate::Trigger::CardsExiledFromYourLibraryOrGraveyard)
    /// (Laelia, the Blade Reforged). Same drain-dedup-clear shape as
    /// [`graveyard_exits_this_batch`](Self::graveyard_exits_this_batch); pushed by
    /// [`Game::create_object`](crate::Game::create_object)'s exile-destination branch.
    pub library_or_graveyard_exits_this_batch: Vec<PlayerId>,
    /// Controllers who created one or more **creature** tokens in the event batch currently
    /// being applied — the accumulator behind
    /// [`Trigger::YouCreateToken`](crate::Trigger::YouCreateToken) (Staff of the Storyteller),
    /// mirroring [`graveyard_exits_this_batch`](Self::graveyard_exits_this_batch)'s batch-once
    /// shape (CR 603.3b's "one or more"). Pushed by [`Game::enqueue_triggers`]'s
    /// `Event::TokenCreated` handling, drained (deduped) and cleared at the end of every batch.
    pub creature_tokens_created_this_batch: Vec<PlayerId>,
    /// `(dying creature's pre-move id, attached Aura's pre-move id, that Aura's controller, that
    /// Aura's def)` tuples — CR 603.6c last-known information for
    /// [`Trigger::EnchantedCreatureDies`](crate::Trigger::EnchantedCreatureDies). Captured by
    /// `Game::apply`'s `MovedToGraveyard`/`TokenCeasedToExist` handling, synchronously as each
    /// creature's own death event applies (whether the death came from a state-based action or a
    /// direct effect like Destroy) — at that instant the creature is still a live permanent, so
    /// its Auras are still attached; by the time a later event orphans the Aura into its *own*
    /// graveyard card (a separate state-based action, applied afterward in the same batch),
    /// `Game::attachments` on the dying creature would already read empty. The controller/def are
    /// captured here too (rather than read back off `aura` at trigger-queue time) because a
    /// *token* Aura's own same-batch orphan event (`TokenCeasedToExist`) tombstones it straight to
    /// `Object::Removed` with no `Moved` lineage to follow — unlike an ordinary Aura's
    /// `MovedToGraveyard`, which still resolves `controller_of`/`def_of` post-move (a Replicate
    /// copy's dies-trigger, CR 707.10a). Read (not drained) per dying creature by
    /// [`Game::queue_enchanted_creature_dies_triggers`](crate::Game::queue_enchanted_creature_dies_triggers), cleared wholesale at the end of every
    /// [`Game::enqueue_triggers`](crate::Game::enqueue_triggers) batch.
    pub dying_creature_attachments: Vec<(ObjectId, ObjectId, PlayerId, CardDef)>,
    /// `(dying creature's pre-move id, power, +1/+1 counters)` — CR 603.10a last-known
    /// information for a Dies trigger's [`Amount::SourcePower`](crate::Amount::SourcePower) /
    /// [`Amount::PerCounterOnSource`](crate::Amount::PerCounterOnSource) reads (Lifeblood
    /// Hydra's "gain life and draw cards equal to its power", Hangarback Walker's Thopter
    /// swarm). Captured at the same choke point and for the same reason as
    /// [`dying_creature_attachments`](Self::dying_creature_attachments) — the creature is still
    /// a live permanent the instant its death event applies, before `create_object` tombstones
    /// it. Read (not drained) by `Game::enqueue_triggers`'s `MovedToGraveyard`/
    /// `TokenCeasedToExist` trigger-scan arms, cleared wholesale at the end of every batch.
    pub dying_creature_stats: Vec<(ObjectId, i32, i32)>,
    /// Pre-move ids of objects that were live battlefield [`Object::Permanent`]s the instant
    /// they were put into a graveyard this batch (CR "put into a graveyard from the
    /// battlefield") — the accumulator behind
    /// [`Trigger::ThisAuraLeaves`](crate::Trigger::ThisAuraLeaves) (Fallen Ideal). Any permanent
    /// kind, not creature-scoped like [`dying_creature_attachments`](Self::dying_creature_attachments).
    /// Captured at the same `Game::apply` `MovedToGraveyard` choke point as `Game`'s own
    /// `permanents_died_this_turn` check, since by the time `Game::enqueue_triggers` runs the
    /// pre-move object has already been
    /// overwritten into `Object::Moved`. Read (not drained) by `Game::enqueue_triggers`'s
    /// `Event::MovedToGraveyard` arm, cleared wholesale at the end of every batch.
    pub permanents_put_into_graveyard_from_battlefield: Vec<ObjectId>,
    /// `(pre-move id, host it was attached to)` pairs for every object that left the battlefield
    /// to ANY zone this batch (destroy/exile, bounce, tuck — not just the graveyard-only
    /// [`permanents_put_into_graveyard_from_battlefield`](Self::permanents_put_into_graveyard_from_battlefield))
    /// — the accumulator behind
    /// [`Trigger::ThisPermanentLeavesBattlefield`](crate::Trigger::ThisPermanentLeavesBattlefield)
    /// (Animate Dead). `host` is CR 603.10a last-known information — `Game::attached_to` read at
    /// the same `Game::apply` exit choke points as `permanents_put_into_graveyard_from_battlefield`,
    /// before the exit tears the attachment down. Read (not drained) by
    /// `Game::queue_leaves_battlefield_triggers`, cleared wholesale at the end of every batch.
    pub permanents_left_battlefield: Vec<(ObjectId, Option<ObjectId>)>,
    /// Pre-move ids of objects that were live battlefield [`Permanent`]s tagged
    /// [`Permanent::serra_recursion`](crate::Permanent) the instant they were put into a
    /// graveyard from the battlefield this batch — Serra Paragon's granted rider (CR 118.9). A
    /// subset of [`permanents_put_into_graveyard_from_battlefield`](Self::permanents_put_into_graveyard_from_battlefield),
    /// captured at the same `Game::apply` `MovedToGraveyard` choke point for the same
    /// last-known-information reason (the flag lives on the live `Object::Permanent`, gone once
    /// `create_object` tombstones it). Read (not drained) by `Game::enqueue_triggers`'s
    /// `Event::MovedToGraveyard` arm to fabricate the real placed trigger (see
    /// [`crate::Effect::Zone(ZoneEffect::ExileGraveyardObjectGainLife)`]), cleared wholesale at the end of every batch.
    pub serra_recursion_deaths: Vec<ObjectId>,
    /// `(pre-move id, def, owner)` for every creature put into a graveyard from the battlefield
    /// this batch — CR 603.10a last-known information for [`Game::queue_watch_death_triggers`],
    /// read only when a *different* event in the same batch (`Event::PlayerLost`) has since
    /// tombstoned that id to [`Object::Removed`]. CR 800.4a: a creature whose owner leaves the
    /// game in the same SBA sweep it dies in leaves the game with them — its own Dies/self-watch
    /// stays suppressed (`Game::enqueue_triggers` never reads this for the self arms) — but the
    /// death is still visible to *other*, surviving players' death-watch (CR 603.6e; Hissing
    /// Iguanar). Captured at the same `Game::apply` `MovedToGraveyard` choke point as
    /// [`dying_creature_stats`](Self::dying_creature_stats), before `PlayerLost` (later in the
    /// same batch) can tombstone the object and make `def_of`/`owner_of` panic. Read (not
    /// drained) by `Game::enqueue_triggers`'s `Event::MovedToGraveyard` arm, cleared wholesale at
    /// the end of every batch.
    pub dying_creature_lki: Vec<(ObjectId, CardDef, PlayerId)>,
}

/// Once-per-turn activation and trigger caps, reset at each untap step.
#[derive(Clone, Default)]
pub(crate) struct OncePerTurnLimits {
    /// Activations this turn of a `once_each_turn`-capped activated ability (CR 602.2b), each
    /// entry (source object, ability index). Checked by
    /// [`Game::ability_activation_gate`](crate::Game::ability_activation_gate); cleared at the start of every turn.
    pub activated: Vec<(ObjectId, usize)>,
    /// Placements this turn of a `once_each_turn`-capped *triggered* ability (CR "this ability
    /// triggers only once each turn"), each entry the ability's source object. Checked and
    /// recorded by [`Game::place_pending_triggers`](crate::Game::place_pending_triggers); cleared at the start of every turn.
    /// ponytail: keyed by source object alone (not (source, ability index) like `activated`) —
    /// no pool card has two once-each-turn *triggered* abilities on one permanent; widen to a pair
    /// if one ever does.
    pub triggered: Vec<ObjectId>,
}

/// Links between exiling sources and the cards they exiled.
#[derive(Clone, Default)]
pub(crate) struct ExileLinks {
    /// The O-Ring pattern (CR 603.6e): each entry is `(source, exiled)` — an object exiled by
    /// [`Effect::Destroy(DestroyEffect::ExileUntilSourceLeaves)`](crate::Effect::Destroy(DestroyEffect::ExileUntilSourceLeaves)), linked to the ability's own source for as long as that source stays on the battlefield.
    pub until_source_leaves: Vec<(ObjectId, ObjectId)>,
    /// Skyclave Apparition's linked exile: each entry is `(source, exiled)` — an object exiled
    /// by [`Effect::Destroy(DestroyEffect::ExileTargetMintingIllusionOnLeave)`](crate::Effect::Destroy(DestroyEffect::ExileTargetMintingIllusionOnLeave)), linked to the ability's own source. Unlike
    /// `until_source_leaves` the card is never returned; [`Game::check_leaves_battlefield_illusions`](crate::Game::check_leaves_battlefield_illusions) mints its owner an Illusion instead once `source`
    /// leaves the battlefield, then drops the entry.
    pub illusion_on_source_leave: Vec<(ObjectId, ObjectId)>,
    /// The "exiled with" pattern (CR 400.10a): each entry is `(source, exiled)` — a card exiled
    /// by [`Effect::Mill(MillEffect::ExileDiscardedWithThis)`](crate::Effect::Mill(MillEffect::ExileDiscardedWithThis)), linked to the ability's own source (Currency Converter).
    pub with_source: Vec<(ObjectId, ObjectId)>,
    /// Hofri Ghostforge's minted Spirit token: each entry is `(token, exiled)` — the token's
    /// granted "When this token leaves the battlefield, return the exiled card to its owner's
    /// graveyard" rider (recorded by [`Event::TokenGrantedReturnExiledOnLeave`](crate::Event::TokenGrantedReturnExiledOnLeave) at mint time),
    /// baking in exactly which exiled card `token` must return. Read (not drained) by
    /// [`Game::queue_token_return_exiled_trigger`](crate::Game::queue_token_return_exiled_trigger) once `token` leaves the battlefield — unlike
    /// `illusion_on_source_leave`, this places a real CR 603 triggered ability rather than an
    /// SBA-style departure sweep, so it never needs to be swept/polled; a stale leftover entry
    /// for a token that already left is harmless (object ids are never reused).
    pub token_leaves_returns_exiled: Vec<(ObjectId, ObjectId)>,
}

/// Pending CR 603.7 delayed triggered abilities not yet placed on the stack.
#[derive(Clone, Default)]
pub(crate) struct DelayedTriggers {
    /// Each `(controller, source, fire_at, effect)`, scheduled by
    /// [`Effect::Misc(MiscEffect::ScheduleAtNextUpkeep)`](crate::Effect::Misc(MiscEffect::ScheduleAtNextUpkeep)) and drained in full
    /// the next time a step matching `fire_at` begins
    /// ([`Game::fire_delayed_triggers`](crate::Game::fire_delayed_triggers)).
    pub scheduled: Vec<(PlayerId, ObjectId, Step, Effect)>,
    /// Each `(controller, source, filter, then)`, armed by
    /// [`Effect::Misc(MiscEffect::ScheduleNextCastTrigger)`](crate::Effect::Misc(MiscEffect::ScheduleNextCastTrigger)) — CR 603.7's
    /// event-armed sibling of `scheduled` above: fires once the next time `controller` casts a
    /// spell matching `filter` this turn, then removed
    /// ([`Game::fire_next_cast_triggers`](crate::Game::fire_next_cast_triggers)). Cleared
    /// unconsumed at the next turn's Untap step (`Game::apply`'s `Step::Untap` arm) — the same
    /// "this turn" boundary `Player::spells_cast_this_turn` resets at.
    pub pending_next_cast: Vec<(PlayerId, ObjectId, SpellFilter, &'static [Effect])>,
    /// Each `(controller, source, watched)`, armed by
    /// [`Effect::Misc(MiscEffect::ArmCombatDamageWatch)`](crate::Effect::Misc(MiscEffect::ArmCombatDamageWatch)) — CR 603.7's
    /// event-armed sibling of `pending_next_cast` above, but object-scoped (watches one specific
    /// creature) rather than filter-scoped: fires
    /// [`Effect::Misc(MiscEffect::BecomePrepared)`](crate::Effect::Misc(MiscEffect::BecomePrepared)) on `source` the first time
    /// `watched` deals combat damage to a player
    /// ([`Game::fire_combat_damage_watch_triggers`](crate::Game::fire_combat_damage_watch_triggers)),
    /// then removed. Cleared unconsumed at end of combat (CR "this combat" — `Game::apply`'s
    /// `Step::EndCombat` arm), unlike `pending_next_cast`'s turn-boundary expiry.
    pub pending_combat_damage_watch: Vec<(PlayerId, ObjectId, ObjectId)>,
    /// Each `(controller, source, card)`, armed by
    /// [`Effect::Misc(MiscEffect::ScheduleThisTurnCombatDamageCopy)`](crate::Effect::Misc(MiscEffect::ScheduleThisTurnCombatDamageCopy))
    /// (Surge to Victory) — CR 603.7's *repeatable* sibling of `pending_combat_damage_watch`
    /// above: controller-scoped rather than watching one chosen creature, and never removed on
    /// fire (CR "this turn" fires again on every subsequent qualifying combat-damage event,
    /// unlike `pending_combat_damage_watch`'s "this combat" one-shot). `source` is the arming
    /// ability's own source (the resolving spell), reused as the delayed trigger's stack source
    /// each time it fires, the same shape `pending_next_cast`'s `source` uses. Cleared unconsumed
    /// at the next turn's Untap step (CR "this turn" — `Game::apply`'s `Step::Untap` arm), the
    /// same boundary `pending_next_cast` itself clears at. See
    /// [`Game::fire_combat_damage_copy_triggers`].
    pub pending_combat_damage_copy: Vec<(PlayerId, ObjectId, ObjectId)>,
}

/// A permanent's controller/token-ness/card-def facts, snapshotted at the moment `Effect::Destroy(DestroyEffect::DestroyAll)`
/// destroys it — captured because the permanent is already gone from the battlefield by the time
/// a later `Sequence` step (`Amount::PermanentsDestroyedThisWay`) needs to count how many matched
/// some filter (Ceaseless Conflict's token rider, Culling Ritual's mana rider). `def` carries the
/// type/subtype info a `PermanentFilter` needs to match against.
#[derive(Clone, Copy)]
pub(crate) struct DestroyedThisWay {
    pub(crate) def: CardDef,
    pub(crate) controller: PlayerId,
    pub(crate) token: bool,
}

/// A creature's controller and power, snapshotted at the moment `Effect::Destroy(DestroyEffect::ExileAll)` exiles it —
/// captured because the creature is already gone from the battlefield by the time
/// `Effect::Choice(ChoiceEffect::EachPlayerCreatesFractalFromExiledPower)` (Oversimplify) needs to sum each player's
/// own share (CR 603.6d LKI-adjacent: the exile has already happened, so this reads a snapshot
/// rather than the live board).
#[derive(Clone, Copy)]
pub(crate) struct PowerExiledThisWay {
    pub(crate) controller: PlayerId,
    pub(crate) power: i32,
}

/// Sourced batches for inspect-ledger provenance, lifted off `Permanent` so it stays `Copy`.
/// Batches are the write path for counters / EOT boosts; `plus_counters` / `temp_*` on the
/// permanent are a derived cache refreshed by [`Game::resync_modifier_aggregates`](crate::Game::resync_modifier_aggregates).
#[derive(Clone, Default)]
pub(crate) struct ModifierProvenance {
    /// `(host, count, source_name)` — positive placements; removals shrink batches LIFO.
    pub counter_batches: Vec<(ObjectId, i32, &'static str)>,
    /// `(host, power, toughness, keywords, source_name)` until end of turn; cleared with
    /// [`Event::TempBoostsEnded`](crate::Event::TempBoostsEnded).
    pub temp_boosts: Vec<(ObjectId, i32, i32, &'static [Keyword], &'static str)>,
}
