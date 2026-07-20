# Engine Core and Event Model

**Status:** Current (as of 2026-07-20)
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

State is **event-sourced for board facts** (objects, mana, zones, counters, damage) and **orchestration-tracked for priority and choices** (priority holder, pass count, pending choice). This distinction is an explicit design commitment: `pending_choice`, `consecutive_passes`, and similar orchestration fields live directly on `Game`, not in events, so the event log is an audit trail of what happened rather than a replay harness.

The sole randomness source is an injected splitmix64 seed (`Game::rng_state`), so the engine is deterministic: same seed + same intents = same shuffle order and same events every run.

---

## User Stories

1. As a **server**, I want to call `Game::submit(intent)` and receive the events that result, so I can broadcast deltas to clients.
2. As a **server**, I want the engine to reject invalid intents (wrong player, wrong timing, unknown object id) with a typed `Reject` so I can surface the reason without parsing event logs.
3. As a **test author**, I want to construct a `Game` via `Game::with_players(n, seed)`, populate zones with `spawn_*` helpers, and call `submit` directly, so I can drive the engine without a server or network.
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

- `Game::with_players(n: u8, seed: u64)` creates a game with `n` seats (2–4), each starting at 40 life (Commander default), empty zones, and the given seed. Player 0 is the starting active player holding priority; the game is parked in their first main phase with beginning steps un-run.
- `Game::begin_first_turn()` must be called after setup (libraries shuffled, opening hands drawn): it runs Untap → Upkeep → Draw for the starting player, feeding the post-intent pipeline so upkeep triggers reach the stack.
- In a two-player game the starting player skips their first draw step (CR 103.8a). In three- or four-player games, no player skips (CR 103.8c).

### Objects and zone identity

- Every card, spell, and permanent is an `Object` in a flat arena (`Vec<Object>`), addressed by `ObjectId` (a `u32` index).
- An object takes a **new `ObjectId`** each time it changes zones (CR 400.7). Old slots become `Object::Moved { to }` tombstones so that any holder of an old id can follow the chain to the current id via `Game::zone_of` / `Game::current_id`.
- An `Object::Removed` sentinel is used for objects that have left the game (eliminated player's owned cards after `PlayerLost`). Accessing a `Removed` object panics — these are illegal inputs.
- Objects are typed by zone: `Object::Card` (library / hand / graveyard / exile / command), `Object::Spell` (stack, awaiting resolution), `Object::Permanent` (battlefield).

### Event sourcing

- `Event`s are the sole mechanism for mutating **board facts**: life totals, zone membership, counters, tapped/untapped, damage marks, mana pools, stack contents.
- `Game::apply(event)` and `Game::apply_all(events)` apply events individually. Every handler in `apply.rs` is a direct, pattern-matched mutation with no callbacks.
- **Priority, pending choices, and pass bookkeeping are not event-sourced.** They live as plain fields on `Game` and are updated in the submit path directly. This means the event log alone does not reconstitute priority state — which is intentional: games are in-memory only (lobby-table-routing-and-live-game spec) and do not need replay.
- Library order is not event-sourced either: shuffles and draws mutate `Player::library` directly rather than emitting a full-reorder event, preserving privacy (other players must not see the order).

### Post-intent pipeline

After every `submit` (and after `begin_first_turn`), `PostIntentPipeline::run` executes these phases in order:

1. **StateBasedActions** — `check_state_based_actions` sweeps to a fixpoint (repeatedly until no new events are produced): creature lethal damage death, 0-or-less toughness death, planeswalker 0-loyalty, Aura-falls-off, Equipment detaches, empty-library loss, player life ≤ 0.
2. **PriorityHandoffOnElimination** — if the priority holder just lost, advance to the next living player.
3. **TriggerEnqueue** — scan just-produced events and enqueue triggered abilities (self-referential ETBs, watch-others death triggers, controller-scoped upkeep/end-step triggers, etc.).
4. **DelayedTriggers** — fire CR 603.7 scheduled delayed triggers whose step has now arrived.
5. **NextCastTriggers** / **CombatDamageWatchTriggers** / **CombatDamageCopyTriggers** — event-armed one-shot and repeatable delayed watches.
6. **TriggerPlacement** — place enqueued pending triggers onto the stack in APNAP order (active player's triggers first; each player orders their own simultaneous triggers).
7. **RefreshActions** — recompute every living seat's `Vec<LegalAction>`.

### State-based actions (SBA)

Implemented SBAs (CR 704):

- Creature with `marked_damage >= toughness` or `deathtouched = true` → dies (unless indestructible). Indestructible creatures are still killed by 0-or-less toughness.
- Creature with `toughness <= 0` → dies (indestructible does not save it from this SBA).
- A regeneration shield (`regeneration_shields > 0`) replaces a "destroyed" SBA with regeneration (not for 0-toughness).
- Planeswalker with `loyalty <= 0` → goes to graveyard.
- Aura attached to nothing or to an illegal host → goes to graveyard (token Auras cease to exist).
- Equipment attached to an illegal host → detaches (does not die).
- A player at ≤ 0 life → loses. A player who must draw from an empty library → loses.
- `PlayerLost` tombstones every object owned by the loser (CR 800.4a); any permanent others control that was owned by the loser returns to its new owner (control effects end). The last surviving player is the winner.

### Effective characteristics (P/T, keywords)

- **Power/toughness** are computed on demand via a two-pass ordered layer list (`PtLayer`):
  - Layer 7b: base P/T (printed, or set by a `BasePtSet` continuous effect like Darksteel Mutation).
  - Layer 7c: additive modifications — +1/+1 counters, until-EOT pumps, anthem static effects, `grant_to_attached` Aura/Equipment bonuses.
  - Each `PtLayer` entry carries a `source` ObjectId and a `timestamp` for tie-breaking. Full CR 613 dependency ordering is deferred (engine-core-and-event-model spec).
- **Keywords** are a set-union of the permanent's base keywords (from `CardDef`), granted keywords (anthems, backup, attached Aura grants), and conditional keywords (e.g. first strike if attacking). Full CR 613 lose-all-abilities ordering is deferred.
- Results are memoized in `CharacteristicsCache` and invalidated on relevant events (counter changes, pump effects, anthem attachment/detachment). Cache cells are per-object.

### Elimination

- Any player whose life total drops to 0 or below, or who must draw from an empty library, or who concedes, emits `PlayerLost`.
- `PlayerLost` apply arm: removes every object the loser owns from all zones; ends every control effect granted by or to that player; drops the player from turn-order and priority rotation.
- The active player checks `next_player` (which skips `lost` seats) to hand off priority.
- The sole survivor after all losses is the winner; `Game::winner()` returns `Some(PlayerId)`.
- Eliminated players stay in the `players` vec (with `lost = true`) and are skipped by all iteration paths.

### Determinism and RNG

- The sole randomness source is `Game::rng_state: u64` (splitmix64 seed).
- `Game::next_u64()` advances the state and returns a value; `Game::shuffle` uses Fisher-Yates over this.
- Seeding is injected at construction (`with_players(n, seed)`); the lobby's server picks a seed before dealing opening hands.

---

## Implementation Decisions

- **`CardDef` is `Copy` and `&'static`.** Abilities are `&'static [Ability]`, so `CardDef` fields never heap-allocate at runtime. The `card-dsl` feature's `intern` / `static_slice` helpers leak owned vecs into static slices at load time (a bounded, load-once pool). This enables zero-cost `Clone` of `Game` (needed for look-ahead and snapshot forking).
- **`Effect` enum grows only from real card demand (card-dsl-and-card-pool spec).** New card behavior = new `Effect` variant + one `Game::run` dispatch arm + `Event::apply` arm + TOML entry. No caller bypasses `Game::run` to apply effects directly.
- **P/T layers are engine-internal** (`PtLayer` is not a DSL or TOML surface), not stored, and rebuilt fresh on each query. Real CR 613 timestamps and dependency ordering are forward-compatible stubs.
- **No I/O, no `async`, no wall-clock in the engine.** Time-based behavior (suspend, time counters) is event-triggered, not polled.
- **Game state is `Clone`.** `Game` derives `Clone` so the server can snapshot for spectator projection or the engine can be forked for look-ahead without additional complexity.
- **`ObjectId` is a `u32` arena index.** Out-of-range ids are rejected at the `submit` gate before any handler sees them, preventing untrusted input from causing panics.
- **`Reject` is typed.** `submit` returns `Err(Reject::ChoicePending)`, `Err(Reject::UnknownObject)`, etc., so callers can log the exact reason without parsing events.

---

## Testing Decisions

- **Direct-API unit tests are the default.** The engine has no server or network; tests call `Game::with_players`, `spawn_in_hand`/`spawn_on_battlefield`/`stack_library`, `fund_mana`, then `submit` and assert on the returned events or board state.
- **Test seam:** `Game::with_players(n, 0)` is a fully deterministic zero-seed game; tests that need shuffles inject a fixed seed.
- **SBA tests** should construct a minimal board (a creature with lethal damage marks), submit a `PassPriority`, and assert the creature moved to the graveyard.
- **Trigger tests** should verify the pending trigger group is populated after the triggering event and that `place_pending_triggers` puts it on the stack.
- **Elimination tests** should assert `Game::winner()` changes correctly and that the loser's objects are gone.
- **Characteristics tests** should construct an attacker, attach an anthem, and assert `Game::power` returns the boosted value.
- Prior art: `tests/game.rs` in the `engine` crate holds the canonical multi-player integration scenarios.

---

## Out of Scope

- **Full CR 613 layers** (type-changing, lose-all-abilities, dependency ordering, timestamp conflicts beyond 7b/7c). Flagged when a deck needs them via that deck's `docs/fidelity/<slug>-increments.md`.
- **Replacement effects** (general: doubling effects, damage prevention beyond the implemented per-player/table-wide combat shields, enter-as-copy). Partial implementations exist; the full CR 614 replacement-effect engine is a backlog item.
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
