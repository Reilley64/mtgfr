# Engine Core and Event Model

**Status:** Current (as of 2026-07-25)
**Module:** `crates/engine` (`src/lib.rs`, `src/core.rs`, `src/apply.rs`, `src/pipeline.rs`, `src/zones.rs`, `src/state.rs`, `src/characteristics.rs`, `src/characteristics_cache.rs`, `src/spawn.rs`)

---

## Problem Statement

A Commander game needs an authoritative, deterministic, server-side rules engine that can:

- Represent the complete board state for 2–4 players across all seven MTG zones.
- Mutate state only in response to validated player intents, emitting a reproducible event stream.
- Run state-based actions (SBAs) to a fixpoint, then queue and place triggered abilities, before returning control.
- Keep the engine pure (no I/O, no wall-clock, no external randomness) so two runs with the same seed and intents produce identical events.
- Support data-driven card scripts without baking game logic per-card into the engine core.

---

## Solution

`crates/engine` is a pure Rust library exposing a single `Game` struct — the authoritative state of one match. All mutation flows through `Game::submit(intent) -> Result<Vec<Event>, Reject>`, which validates the intent, applies the resulting `Event`s, then runs a fixed post-intent pipeline (SBAs → triggers → action refresh) before returning.

State is **event-sourced for board facts** (objects, mana, zones, counters, damage) and **orchestration-tracked for priority and choices** (priority holder, pass count, pending choice). This distinction is an explicit design commitment: `pending_choice`, `consecutive_passes`, `pending_obligations`, `resolution_finish`, and similar orchestration fields live directly on `Game`, not in events, so the event log is an audit trail of what happened rather than a replay harness.

The sole randomness source is an injected 32-byte master seed. Each logical random operation derives an isolated splitmix64 stream from `BLAKE3(master_seed || player || op_iteration)`, so the engine is deterministic without coupling one player's library order to another player's random operations.

Real games enter a simultaneous pre-game mulligan phase after libraries are stacked and 2-sample BO1 land-smoothed opening hands are drawn. The first turn is not begun until every living player has kept.

---

## User Stories

1. As a **server**, I want to call `Game::submit(intent)` and receive the events that result, so I can broadcast deltas to clients.
2. As a **server**, I want the engine to reject invalid intents (wrong player, wrong timing, unknown object id) with a typed `Reject` so I can surface the reason without parsing event logs.
3. As a **test author**, I want to construct a `Game` via `Game::with_players(n, seed)` or `Game::with_master_seed(n, master_seed)`, populate zones with `spawn_*` helpers, and call `submit` directly, so I can drive the engine without a server or network.
4. As a **test author**, I want `Game::fund_mana(player)` so I can skip land setup in cost-agnostic tests.
5. As a **rules engine consumer**, I want state-based actions (lethal damage, 0-toughness, planeswalker 0-loyalty, empty library, Aura attachment) checked automatically after every intent, so I don't have to call them explicitly.
6. As a **rules engine consumer**, I want triggered abilities enqueued and placed in APNAP order automatically after every intent, so trigger ordering is always rules-correct.
7. As a **spectator or eliminated player**, I want to continue receiving the game stream after losing, so the table can keep playing without my connection.
8. As a **card author**, I want new card behavior implemented as a new `Effect` variant plus a `Game::run` arm, so card logic stays data-driven and isolated from the engine core.
9. As a **server operator**, I want games to live only in memory, so I need no durable storage for live game state and no replay.
10. As a **client**, I want each player's legal-action list recomputed and attached to every snapshot, so I can render only legal affordances without re-implementing rules on the client.

---

## Behavior

### Game construction and zones

- `Game::with_players(n: u8, seed: u64)` creates a deterministic compatibility seed by expanding the `u64` into the first eight bytes of a 32-byte master seed. `Game::with_master_seed(n: u8, master_seed: [u8; 32])` is the production constructor. Both create `n` seats (2–4), each starting at 40 life (Commander default), empty zones, and player 0 as the starting active player holding priority; the game is parked in their first main phase with beginning steps un-run.
- `Game::choose_starting_player()` rolls a random seat via `with_op_rng(PlayerId(0), …)` and sets both `active_player` and `priority` to it (CR 103.1). The production seeding path (`seed_game`) calls it immediately after construction, before stacking any library or dealing any hand, so the roll is always the game's first random operation and precedes library shuffling (CR 103.2) and opening hands (CR 103.4). Direct-API tests built via the raw constructors above skip this call and stay parked at seat 0.
- Real setup stacks each library, runs 2-sample BO1 land smoothing for seven cards via `Game::deal_smoothed_hand`, then calls `Game::begin_mulligans()`. During this simultaneous mulligan phase, any undecided living seat may `KeepHand` or `Mulligan`; ordinary game actions are blocked until all living seats keep.
- `Game::begin_first_turn()` must be called after setup and mulligans: it runs Untap → Upkeep → Draw for the starting player, feeding the post-intent pipeline so upkeep triggers reach the stack. The seeded-game path calls it automatically when the final player keeps.
- In a two-player game the starting player skips their first draw step (CR 103.8a). In three- or four-player games, no player skips (CR 103.8c).

### Pre-game mulligans

- `hand_size_after_mulligans(0) == 7`, `hand_size_after_mulligans(1) == 7` for the friendly mulligan, then later mulligans draw to size 6, 5, ... down to 1. There is no London bottoming or Vancouver scry.
- `KeepHand { player }` emits `HandKept`. `Mulligan { player }` returns that player's hand to the top of their library, emits `LibraryHandSmoothed { hand_size }` (two derive-per-op shuffles, keeping the closer land count and projecting as visible `LibraryShuffled`), draws the new hand size, then emits `MulliganTaken { player, mulligans_taken, hand_size }`.
- Mulligan redraws are deliberately smoothed too, unlike Arena's reported BO1 behavior that leaves mulligans unsmoothed.
- A seat at hand size 1 auto-keeps after the mulligan redraw. Further keep/mulligan intents from lost seats, already-kept seats, or outside the mulligan phase are rejected as `Reject::Mulliganing`.
- When all living seats have kept, the engine emits `MulligansFinished`, clears `mulliganing`, and begins the first turn. The first-draw skip rule is unchanged and is armed only once the first turn begins.

### Objects and zone identity

- Every card, spell, and permanent is an `Object` in a flat arena (`Vec<Object>`), addressed by `ObjectId` (a `u32` index).
- An object takes a **new `ObjectId`** each time it changes zones (CR 400.7). Old slots become `Object::Moved { to }` tombstones so that any holder of an old id can follow the chain to the current id via `Game::zone_of` / `Game::current_id`.
- An `Object::Removed` sentinel is used for objects that have left the game (eliminated player's owned cards after `PlayerLost`). Accessing a `Removed` object panics — these are illegal inputs.
- Objects are typed by zone: `Object::Card` (library / hand / graveyard / exile / command), `Object::Spell` (stack, awaiting resolution), `Object::Permanent` (battlefield).
- Every live `Card`, `Spell`, and `Permanent` stores `def: CardId` rather than embedding a whole `CardDef`. Rules code resolves the printed definition through the process-global intern table (`card_def(id) -> Arc<CardDef>`) when it needs card text, types, costs, or abilities.

### Event sourcing

- `Event`s are the sole mechanism for mutating **board facts**: life totals, zone membership, counters, tapped/untapped, damage marks, mana pools, stack contents.
- `Game::apply(event)` and `Game::apply_all(events)` apply events individually. Every handler in `apply.rs` is a direct, pattern-matched mutation with no callbacks.
- Event variants that need printed card identity carry `CardId` handles instead of embedding `CardDef` values. Apply, trigger, and projection code dereference those handles through the same intern table when they need the printed definition.
- **Priority, pending choices, pass bookkeeping, keyword obligations, and resolution-finish scratch are not event-sourced.** They live as plain fields on `Game` and are updated in the submit path directly. This means the event log alone does not reconstitute priority state — which is intentional: games are in-memory only (lobby-table-routing-and-live-game spec) and do not need replay.
- Library order is not event-sourced either: shuffles and draws mutate `Player::library` directly rather than emitting a full-reorder event, preserving privacy (other players must not see the order).

### Post-intent pipeline

After every `submit` (and after `begin_first_turn`), `PostIntentPipeline::run` executes these phases in order:

1. **StateBasedActions** — `check_state_based_actions` sweeps to a fixpoint (repeatedly until no new events are produced): creature lethal damage death, 0-or-less toughness death, planeswalker 0-loyalty, Aura-falls-off, Equipment detaches, empty-library loss, player life ≤ 0.
2. **PriorityHandoffOnElimination** — if the priority holder just lost, advance to the next living player.
3. **TriggerEnqueue** — scan just-produced events and enqueue triggered abilities (self-referential ETBs, watch-others death triggers, controller-scoped upkeep/end-step triggers, etc.). `enqueue_triggers` uses a table-driven watch registry for the recurring self/controller/every-player families, and now also routes spell-cast and player-damage watch families through that dispatcher. The remaining bespoke paths are the look-back / scratch-heavy cases such as creature-death watches and other one-off ordering-sensitive hooks.
4. **DelayedTriggers** — fire CR 603.7 scheduled delayed triggers whose step has now arrived.
5. **NextCastTriggers** / **CombatDamageWatchTriggers** / **CombatDamageCopyTriggers** — event-armed one-shot and repeatable delayed watches.
6. **TriggerPlacement** — place enqueued pending triggers onto the stack in APNAP order (active player's triggers first; each player orders their own simultaneous triggers), then drain one queued keyword obligation at a time once ordinary triggers are exhausted: every Echo first, then every Recover, then every Cumulative upkeep.
7. **RefreshActions** — recompute every living seat's `Vec<LegalAction>`.

### State-based actions (SBA)

Implemented SBAs (CR 704):

- Creature with `marked_damage >= toughness` or `deathtouched = true` → dies (unless indestructible). Indestructible creatures are still killed by 0-or-less toughness.
- Creature with `toughness <= 0` → dies (indestructible does not save it from this SBA).
- A regeneration shield (`regeneration_shields > 0`) replaces a "destroyed" SBA with regeneration (not for 0-toughness).
- Planeswalker with `loyalty <= 0` → goes to graveyard.
- Aura attached to nothing or to an illegal host → goes to graveyard (token Auras cease to exist).
- Equipment attached to an illegal host → detaches (does not die).
- Permanent with both +1/+1 and −1/−1 counters → remove `min` of each as a pair (CR 704.5r), before death checks in the same scan.
- Legend rule (CR 704.5j): after event-producing SBAs settle, if a living controller has two or more legendary permanents with the same printed name, pause on `ChooseLegendaryKeep` (one conflict group per sweep, lowest seat then name). The answer keeps one permanent; the rest leave via graveyard / command divert / token cease. Further groups wait for the next sweep.
- A player at ≤ 0 life → loses. A player who must draw from an empty library → loses.
- `PlayerLost` tombstones every object owned by the loser (CR 800.4a); any permanent others control that was owned by the loser returns to its new owner (control effects end). The last surviving player is the winner.

### Effective characteristics (continuous effects, layers, cache)

- `characteristics.rs` rebuilds a per-query **continuous-effect pipeline** (`ContinuousEffect`) for the object being read. Today's readers register attachment statics, runtime self-animation / reanimation sets, anthem statics, and keyword grants into that engine-internal pipeline rather than hard-coding separate layer passes per case.
- **Every duration-scoped continuous effect lives in one modifier registry.** A resolving effect that pumps, grants or strips keywords, sets base P/T, animates, recolors, or copies until end of turn registers a `Modifier { host, source_name, timestamp, duration, kind }` rather than writing a field on `Permanent`. The registry is appended in CR 613.7 stamp order, so readers walk it as-is: `runtime_continuous_effects` turns each entry into its layer entry, `colors_of` folds the layer-5 kinds, and the effective-keyword read subtracts the "loses … and can't have" kinds from the fully-unioned set. Durations are "until end of turn" (swept at cleanup, CR 514.2), "until end of combat" (Jade Statue, swept at the End of Combat step, CR 511.3), and durationless (a lace's "becomes black" — never swept; it lapses with the object per CR 400.7). Both sweeps scan the registry alone and emit `Event::TempBoostsEnded` per host, which drops that host's durationed modifiers in one `retain`.
- **Type/subtype changes** read the layer-4-ish `SetTypes` entries first: attached Auras such as Darksteel Mutation plus runtime self-animation / reanimation type additions (Restless Spire, Excava). Added card types are unioned on; subtype-set entries replace the current creature subtype line in timestamp order, then later subtype-add entries union on top.
- **Ability removal** reads `LoseAllAbilities` entries before the object's own printed abilities/keywords are consulted, so Darksteel Mutation-style "loses all other abilities" suppresses the host's printed static/activated/triggered text while still allowing later granted keywords from the Aura itself.
- **Power/toughness** are computed on demand from ordered layer entries:
  - Layer 7b: base P/T (printed, or set by a `BasePtSet` continuous effect such as Darksteel Mutation, Trench Gorger, Quandrix Charm, or a self-animation).
  - Layer 7c: additive modifications — +1/+1 counters, -1/-1 counters, until-EOT pumps, anthem static effects, and `grant_to_attached` Aura/Equipment bonuses.
  - Every runtime base/type set and every static continuous source carries a CR 613.7 timestamp, so same-layer ordering now handles the pool-relevant stacked-base case where a later Darksteel Mutation overrides an earlier Trench Gorger base-P/T set.
- **Colors** start from the card's colored cost pips (CR 105.2), then apply every layer-5 modifier on the object in timestamp order (CR 613.3c/613.7): a color *SET* (Deathlace, Wild Mongrel's chosen color) replaces everything established before it, a color *ADD* (a manland's animated form) unions onto it. So a Spire laced black and then animated reads blue-black-red, not black alone. A spell on the stack keeps a plain `Spell::set_color` field instead — it has no duration to sweep and ceases to exist as it resolves. Kormus Bell's land-type static folds in ahead of the registered effects rather than at its own timestamp.
- **Keywords** start from the object's printed keywords/conditional keywords (unless a lose-all-abilities effect removed them), then union keyword-grant `ContinuousEffect`s from attachments, runtime grants, and anthems. Backup / granted printed abilities, chosen-color protection grants, and temp "can't have" strips still apply in the existing follow-up reads around that pipeline.
- Results are memoized in `CharacteristicsCache` and invalidated on relevant events (counter changes, pump effects, anthem attachment/detachment). Cache cells are per-object.
- **Copy effects and their exception riders (CR 706/707.2).** `Event::BecameCopy` swaps a permanent's `def` handle for the copied definition (an until-end-of-turn copy stashes the original for cleanup). A copy made "except it has <keywords>" records those keywords as `Permanent::copy_rider_keywords` — a *copiable* characteristic that `Game::copiable_keywords` reports (via `Event::CopyRiderKeywordsGranted`, unioned across stacked riders), so a further copy of that object carries the rider forward. Because a new copy effect replaces the copiable characteristics wholesale, `Event::BecameCopy` clears any prior rider; this effect's own rider (if any) is re-established by the `CopyRiderKeywordsGranted` events that follow it, so a stale "except it has haste" never survives onto a later copy of a vanilla creature.

### Replacement effects (depth 2)

- `replacements.rs` builds a live replacement registry from runtime combat shields plus battlefield static abilities that modify damage, counters, token creation, life gain, or ETB counters.
- Combat-damage chokes consult that registry for player shields (Inkshield), table-wide combat prevention (Moment's Peace), permanent combat-prevention statics (Guard Gomazoa / Fog Bank), Phantom Centaur's self-shield, and Tajic's noncombat "other creatures" shield.
- Counter placements, token creation, and life gain route through the same registry, so controller-owned doublers/adders (Hardened Scales, Benevolent Hydra, Doubling Season, Pest Rescuer, Ozolith, the Shattered Spire) are read through one path instead of per-call-site scans.
- Non-spell battlefield-entry events (`ReanimatedToBattlefield`, `FlickeredToBattlefield`, `ReturnedFromLinkedExile`, `SearchedToBattlefield`, `PutOntoBattlefieldFromHand`) immediately apply their printed `enters_with_counters` / vanishing counters and any live "creatures you control enter with additional counters" statics through that same registry, matching the cast/land entry path.
- Spell-only pausing as-enters choices such as devour and `enter_as_copy` still run in the spell-resolution path rather than the generic registry. The engine does not yet implement full CR 614/616 ordering across arbitrary overlapping replacements.

### Elimination

- Any player whose life total drops to 0 or below, or who must draw from an empty library, or who concedes, emits `PlayerLost`.
- `PlayerLost` apply arm: removes every object the loser owns from all zones; ends every control effect granted by or to that player; drops the player from turn-order and priority rotation.
- The active player checks `next_player` (which skips `lost` seats) to hand off priority.
- The sole survivor after all losses is the winner; `Game::winner()` returns `Some(PlayerId)`.
- Eliminated players stay in the `players` vec (with `lost = true`) and are skipped by all iteration paths.

### Determinism and RNG

- The sole randomness source is `Game::master_seed: [u8; 32]`.
- Each player has a monotonic `op_iteration: u64`. `Game::with_op_rng(player, f)` derives `BLAKE3(master_seed || player_index:u8 || op_iteration:u64-le)`, increments that player's iteration once, and gives `f` a short-lived splitmix64 `OpRng`.
- `OpRng::gen_index(upper)` uses rejection sampling to avoid modulo bias. `Game::shuffle` uses Fisher-Yates over that unbiased index helper.
- One mid-game library shuffle, one random graveyard pick, one random opponent pick, or one random-order bottoming is one logical operation and bumps exactly one player's counter. `Game::deal_smoothed_hand` and mulligan redraws run two shuffle samples and bump the seat's counter twice when the library has at least two cards. Controller-scoped card effects attribute random operations to that effect's controller.
- The CR 103.1 starting-player roll is a game-level operation carried on seat 0's counter: `choose_starting_player` spends seat 0's iteration 0 before any seat's library or hand operations run, so from that point on seat 0's op-iteration stream is offset by one relative to the other seats'.
- Seeding is injected at construction (`with_master_seed(n, master_seed)`); `with_players(n, seed)` remains a deterministic `u64` test convenience.

---

## Implementation Decisions

- **Printed definitions are interned behind `CardId`.** `CardDef` is `Clone`, not `Copy`. `intern_card_def(def)` stores an `Arc<CardDef>` in a process-global table and returns a small `CardId`; `card_def(id)` clones the shared `Arc` back out. Non-empty Scryfall oracle ids dedupe to one stable handle, nested back/adventure faces are interned up front, and runtime restore paths (flip, adventure, split-card stack exits) reuse those handles instead of cloning fresh defs.
- **`Effect` is `Clone`, not `Copy`.** Abilities, stack entries, and event handlers clone effects when they need owned values. Sequence-like effect payloads (`Effect::Sequence::steps`, `ChooseOne::options`, `Conditional::then`) are shared `Arc<[Effect]>` lists, so runtime rebuilds and pause/resume continuations own their tails without leaking. Printed `CardDef` lists (`abilities`, `keywords`, `conditional_keywords`, `identity_pips`, `colors`, `subtypes`, `otags`, `hand_ability`, `halves`) are also Arc-backed, so the interned `CardId -> Arc<CardDef>` table shares card text without the older plain-slice leaks.
- **`Effect` enum grows only from real card demand (card-dsl-and-card-pool spec).** New card behavior = new `Effect` variant + one `Game::run` dispatch arm + `Event::apply` arm + TOML entry. No caller bypasses `Game::run` to apply effects directly.
- **Keyword obligations share one queue.** `Game::pending_obligations: Vec<Obligation>` carries Echo, Recover, and Cumulative upkeep work that is not represented as ordinary `TriggerGroup`s. Placement preserves the existing priority order by selecting Echo obligations first, then Recover, then Cumulative upkeep, while keeping FIFO order within each kind.
- **Recurring trigger watches are table-driven.** `triggers.rs` models the common enqueue shapes (self source, one player's battlefield, one player's graveyard, every battlefield permanent, every battlefield permanent except one player) as `TriggerWatch` rows plus a small event-context carrier. `enqueue_triggers` dispatches ETB/turned-face-up/attack, step-begin, life-change, spell-cast, player-damage, and batch token/exile families through that table, while death look-back and other scratch-heavy or ordering-sensitive cases stay bespoke.
- **Resolving instants and sorceries share one finish-policy scratch slot.** Self-move riders like Spell Crumple, Rousing Refrain, and Vengeful Rebirth set `Game::resolution_finish: Option<FinishPolicy>` during their own resolution; `finish_instant_sorcery_resolution` consumes that slot immediately after the spell's effect body finishes.
- **P/T layers are engine-internal** (`PtLayer` is not a DSL or TOML surface), not stored, and rebuilt fresh on each query. Real CR 613 timestamps and dependency ordering are forward-compatible stubs.
- **Replacement reads are registry-backed.** `replacements.rs` materializes live damage/counter/token/life/ETB-counter replacement entries from runtime shields and functional static abilities; existing helper reads (`counters_after_replacements`, prevention predicates, life/token doublers, extra ETB counters) delegate to that registry instead of open-coding fresh battlefield scans.
- **No I/O, no `async`, no wall-clock in the engine.** Beacon fetching and seed policy live in the server; the engine only receives the master seed. Time-based behavior (suspend, time counters) is event-triggered, not polled.
- **Game state is `Clone`.** `Game` derives `Clone` so the server can snapshot for spectator projection or the engine can be forked for look-ahead without additional complexity. Those clones share immutable printed definitions through the intern table while keeping mutable board state independent.
- **`ObjectId` is a `u32` arena index.** Out-of-range ids are rejected at the `submit` gate before any handler sees them, preventing untrusted input from causing panics.
- **`Reject` is typed.** `submit` returns `Err(Reject::ChoicePending)`, `Err(Reject::UnknownObject)`, etc., so callers can log the exact reason without parsing events.

---

## Testing Decisions

- **Direct-API unit tests are the default.** The engine has no server or network; tests call `Game::with_players`, `spawn_in_hand`/`spawn_on_battlefield`/`stack_library`, `fund_mana`, then `submit` and assert on the returned events or board state.
- **Test seam:** `Game::with_players(n, 0)` is a fully deterministic zero-seed game; tests that need production-shaped seeding inject a fixed `[u8; 32]` via `Game::with_master_seed`.
- **Mulligan tests** should assert the friendly mulligan redraws to seven, later mulligans draw to size, hand size 1 auto-keeps, and the first turn does not begin until every living player has kept.
- **SBA tests** should construct a minimal board (a creature with lethal damage marks), submit a `PassPriority`, and assert the creature moved to the graveyard.
- **Trigger tests** should verify the pending trigger group is populated after the triggering event and that `place_pending_triggers` puts it on the stack.
- **Keyword-obligation tests** should assert the unified `pending_obligations` queue still drains Echo before Recover before Cumulative upkeep.
- **Elimination tests** should assert `Game::winner()` changes correctly and that the loser's objects are gone.
- **Characteristics tests** should construct an attacker, attach an anthem, and assert `Game::power` returns the boosted value.
- **Replacement tests** should cover at least one prevention/static case and one non-spell battlefield-entry counter case, so the shared registry is exercised from both damage and ETB chokes.
- Prior art: `tests/game.rs` in the `engine` crate holds the canonical multi-player integration scenarios.

---

## Out of Scope

- **Full CR 613 completion.** The engine now has a real continuous-effect registry plus pool-relevant timestamp handling for stacked base-P/T sets, but it still does not model the full rules space: general dependency ordering, full card-type replacement/removal ordering, and every exotic same-layer timestamp conflict remain fidelity-driven follow-up work in `docs/fidelity/<slug>-increments.md`.
- **Full CR 614 / 616 completeness.** The engine now has a live replacement registry covering prevention shields, counter/token/life doublers, and non-spell ETB counter propagation, but it still does not model arbitrary replacement ordering/choice, every as-enters modifier, or every damage-prevention observability case. Remaining gaps stay fidelity-driven.
- **Durable game persistence.** Games are in-memory only; lost on server restart (lobby-table-routing-and-live-game spec).
- **Intent replay.** The old `SavedGame`/`SavedIntent` replay path was deleted in lobby-table-routing-and-live-game spec; the event log is audit-only.
- **Spectator projection from library / hand contents.** Hand and library contents are already filtered at the schema/wire layer, not in the engine.

---

## Further Notes

- See `2026-07-20-turn-priority-and-stack.md` for the priority model and step sequencing that sits above this core.
- See `2026-07-20-choices-actions-and-resolution.md` for how `PendingChoice` and `LegalAction` interact with this submit path.
- See `2026-07-20-card-dsl-and-card-pool.md` for the `CardDef`/`Effect` DSL that feeds into `Game::run`.
- `CONTEXT.md` is the canonical vocabulary reference; keep code and test names aligned to it.
- Engine gaps for cards in an active grind live in that deck's `docs/fidelity/<slug>-increments.md` (fidelity-grind skill).
