# Engine refactor program (design)

**Status:** Approved design input (2026-07-25). Living behavior stays in the indexed surface specs; each wave updates those specs in the same change.
**Surfaces:** `engine-core-and-event-model`, `card-dsl-and-card-pool`, `choices-actions-and-resolution`, `prompts-and-pending-choices`, `turn-priority-and-stack`, `wire-protocol-and-visibility` (hard cutover allowed for this program).

---

## Problem Statement

The engine’s `CardDef: Copy` / `Effect: Copy` / `Event: Copy` straitjacket and ad-hoc continuous/replacement/trigger/choice machinery force side tables, leaked `'static` pools, fixed caps, and 2–4k-line special-case modules. That slows faithful card work and makes CR 613/614 gaps expensive to close.

## Solution

A sequenced program of six waves (A–F). Approach: **interned handles first**, then structural cleanups, then rules engines that close **pool-relevant** deferred gaps (not full CR completion). Wire: **hard cutover** — delete obsolete `PendingChoice` / view / client prompt arms in the same PR; no expand-only staging for this program. Skip mechanical file-split wave unless a file blocks a wave.

## Locked decisions

| Decision | Choice |
|----------|--------|
| Program scope | Waves A→F, sequential PRs |
| Wire | Hard cutover; cleanup legacy arms |
| CR 613 / 614 depth | Structural pipeline + close known deferred gaps the pool already hits |
| Ownership (Wave A) | `CardId` + interned `Arc<CardDef>`; objects/events store ids/handles |
| `Effect` | `Clone` only; box large arms; nested slices via `Arc` |
| `Event` | May drop `Copy` (become `Clone`) where needed |
| `Game: Clone` | Preserved via shared defs |
| DSL TOML keys | Unchanged unless a wave explicitly needs a new surface |
| Sequential SM / event-sourced board facts | Unchanged |

## Wave map

| Wave | Goal | Primary modules | Specs to update |
|------|------|-----------------|-----------------|
| **A** | Drop `CardDef`/`Effect` Copy; intern defs | `types/`, `de.rs`, objects, events, cards load | `engine-core`, `card-dsl` |
| **B** | Collapse keyword obligation queues + resolution finish flags | `lib.rs`, `priority`, `apply`, `resolution` | `engine-core`, `choices-actions` |
| **C** | Table-driven trigger enqueue | `triggers.rs`, `types/trigger.rs` | `turn-priority-and-stack`, `engine-core` |
| **D** | Genericize `PendingChoice`; hard-cut wire/client | `types/stack.rs`, `pending/`, schema, proto, client prompts | `choices-actions`, `prompts-and-pending-choices`, `wire` |
| **E** | CR 613 continuous-effects engine + pool gaps | `characteristics.rs` | `engine-core` |
| **F** | CR 614 replacement-effects engine + pool gaps | event mint / `apply` | `engine-core` |

---

## Wave A — Interned defs / drop Copy

### Architecture

- `CardId(u32)` is `Copy`, indexes a process-global (or load-scoped) intern table of `Arc<CardDef>`.
- `Card` / `Spell` / `Permanent` store `def: CardId`.
- `Game::def(id) -> &CardDef` (and thin wrappers). Call sites that today take `CardDef` by value migrate to `CardId` or `&CardDef`.
- Load (`de.rs` / `cards`): build owned `CardDef`, intern to `CardId`. Replace required `Box::leak` / `static_slice` with `Arc<[T]>` (or owned fields on `CardDef`). Token registry stores `CardId` (or `Arc<CardDef>`), not `Copy` values.
- `CardDef`: `Clone`, not `Copy`. Ability/effect/keyword slices: `Arc<[…]>` (or equivalent shared ownership).
- `Effect`: `Clone`, not `Copy`. Large variants may `Box`. Nested `Sequence` / `Conditional` steps use `Arc<[Effect]>`.
- `Event` variants that today embed `CardDef` embed `CardId` instead. `Event` may become `Clone`-only. Projection reads defs via intern/`Game` as needed.
- `StackItem` / `StackEntry` hold `Effect` by `Clone` (boxed where useful).

### Behavior

Observable game rules unchanged. Same seed + intents → same board outcomes. Clone of `Game` still yields an independent mutable fork that shares immutable defs.

### Testing

- Existing engine nextest suite is the regression net.
- New: intern identity (same oracle id → same `CardId` within a load); clone isolation (mutate one clone’s arena, sibling unchanged); equality/`PartialEq` still usable where tests rely on it.

### Out of scope for A

CR 613/614, trigger table rewrite, PendingChoice reshape, obligation queue collapse.

---

## Wave B — Obligations + finish policy

### Architecture

Replace parallel `Game` fields:

- `pending_echo` / `pending_recover` / `pending_cumulative_upkeep` → one `pending_obligations: Vec<Obligation>` (kind + object + cost payload).
- `self_exile_time_counters` / `self_tuck_to_library_bottom` / `self_exile_on_resolve` → one `resolution_finish: Option<FinishPolicy>` on the resolving spell / resume frame.

Drain order preserved (echo → recover → cumulative upkeep) as obligation priority, not separate vecs.

### Behavior

Identical player-visible choices and finish destinations. Specs describe the unified queues.

### Testing

Existing echo/recover/cumulative-upkeep and finish-destination tests; add one regression per collapsed path if coverage is thin.

---

## Wave C — Table-driven triggers

### Architecture

Trigger defs declare watch predicates (event family + filters). `enqueue_triggers` iterates produced events × battlefield (and look-back snapshots) against the table instead of a giant per-`Event` match. Keep APNAP placement, intervening-if, and delayed-trigger machinery.

Migrate today’s arms incrementally behind the table; delete dead match arms when covered. Behavior-identical for existing cards.

### Testing

Existing trigger suite; add a unit test that a registered watch fires for a synthetic event without a bespoke match arm.

---

## Wave D — Generic PendingChoice + hard wire cut

### Architecture

Collapse near-duplicate target/mode/pay-or choices into a smaller enum (choose-N-from-legal, yes/no, order, arrange, pay-or-X, search, combat assign, …) with typed resume continuations. Remove card-named variants (`TradeSecrets*`, etc.) by encoding them as generic choices + resume.

**Wire hard cutover (approved):** rewrite `PendingChoiceView` / proto oneofs / client prompt mounts to the new shape in the same PR; delete legacy arms and dead client branches. No N/N−1 expand-only window for this wave.

### Testing

Engine choice tests + schema projection tests + client Scene tests for prompt surfaces touched. Interaction checklist for prompts.

---

## Wave E — CR 613 continuous effects (depth 2)

### Architecture

Replace ad-hoc grant readers with a layered continuous-effect pipeline (type-changing → abilities → P/T, etc.). Migrate today’s anthem/Aura/attachment/keyword special cases onto layer registration. Implement **pool-relevant** deferred pieces: stacked base-sets / dependency-or-timestamp where pool cards need them, lose-all-abilities where needed, color SET layering already partially present.

Still not full CR 613 completeness; remaining gaps stay in fidelity increments.

### Testing

Characteristics tests for migrated cases + new tests for each closed gap (stacked base P/T, lose-all-abilities host, etc.).

---

## Wave F — CR 614 replacements (depth 2)

### Architecture

Replacement registry consulted at event-mint time (damage, enters, counter placement, etc.). Migrate today’s prevention shields, enter-as-copy, and counter-replacement one-offs onto it. Close pool-relevant doubling/prevention/“as enters” gaps called out in fidelity increments.

### Testing

Existing prevention/enter-as-copy tests + new tests per closed gap.

---

## Global constraints

- Pure engine: no I/O, no wall-clock, injected RNG only.
- Guard-return-first; TDD for behavior changes; every bug fix gets a regression test.
- Update living surface specs in the same change as behavior/representation changes; this design doc is not a substitute.
- Angular commit subjects on squash PR titles (`refactor:` / `feat:` as appropriate).
- Branch naming: `cursor/<wave-slug>-6cd5`.
- Verify with `just server-check` (or focused nextest + clippy/fmt) before claiming a wave done.
- Do not relitigate sequential state machine or event-sourced board facts.

## Success criteria (program)

- Waves A–F each mergeable with green CI.
- No `CardDef: Copy` / `Effect: Copy` requirement; interned defs.
- No parallel echo/recover/cumulative vecs or finish bool sprawl.
- Trigger enqueue is table-driven for migrated watches.
- PendingChoice + wire/client prompts use the generic model; legacy arms gone.
- Characteristics/replacements go through 613/614 pipelines for migrated + newly closed pool gaps.
- Existing engine suite remains green after each wave.

## Out of scope (program)

- Full CR rules completion beyond pool-relevant gaps.
- Intent replay / durable game persistence.
- Premature DSL generalization unrelated to a wave’s migration.
- Mechanical-only file splits (Wave G) unless blocking.
