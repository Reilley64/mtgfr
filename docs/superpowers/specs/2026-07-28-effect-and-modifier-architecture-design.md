# Effect / modifier architecture split (design)

**Status:** Approved design input (2026-07-28). Living behavior stays in the indexed surface specs; each wave updates those specs in the same change.
**Surfaces:** [`card-dsl-and-card-pool`](2026-07-20-card-dsl-and-card-pool.md), [`engine-core-and-event-model`](2026-07-20-engine-core-and-event-model.md), [`choices-actions-and-resolution`](2026-07-20-choices-actions-and-resolution.md), [`card-inspect`](2026-07-20-card-inspect.md).
**Relationship to the program:** Refines Wave E of [`engine-refactor-program`](2026-07-25-engine-refactor-program-design.md) with a concrete architecture, and adds the DSL vocabulary work that program explicitly deferred ("DSL TOML keys unchanged unless a wave explicitly needs a new surface"). This design is that explicit need.

---

## Problem Statement

Engine growth is expressed as variant growth. Today: `Effect` 22 variants, `Trigger` 65, `Condition` 47, and — because each new card that needs to pass a value between steps has nowhere generic to put it — 15 bespoke `ResolutionFrame` fields plus 8 `Amount::*ThisWay` variants plus 8 fused `*Then*` effect variants that exist only to weld two steps together.

`crates/cards/src/types/effect/counters.rs:118` states the failure in its own `ponytail:` note: `RemoveAllButOnePlusOneCounterThenGainLife` is *"+1/+1-only and always 'keep one, gain life' — Lily Bowen is the only consumer."* One card, one variant, one exhaustive-match arm in every walker.

Separately, and for a different reason, the characteristics half sprawls. `ContinuousEffect { source, timestamp, kind }` is documented as *"built fresh per query from today's board/runtime state"* (`characteristics.rs:22`), which means the state it reads has nowhere to live but bespoke fields: 13 until-EOT / boost fields on `Permanent`, 6 registry-shaped fields on `Game` (`skip_next_untap`, `abilities_granted_until_eot`, `pending_enter_bonus_counters`, `exile_time_counters`, `play_permissions`, `combat_extras`), 20 reconstruct sites, a hand-maintained reset in `Event::TempBoostsEnded` that must be extended by hand for every new field, and a `Box::leak` kept solely so `Permanent` stays `Copy`.

These two sprawls look identical from a distance and have been treated as one problem. They are not, and the fixes are opposite.

## Goal

State the architectural split, then fix each half with the mechanism that half actually wants:

1. **Resolution stays data** — generalize the vocabulary so cards compose from generic verbs instead of accreting fused variants. No Rust per card.
2. **Modifiers become one registry** — collapse 19 ad-hoc fields into a single registered, layer-ordered list.

## The split (core principle)

The engine has two halves with opposite requirements. Every architectural argument in this design follows from which half you are in.

| | **Resolution** — *what happens* | **Modifiers** — *what is true now* |
|---|---|---|
| Examples | `Effect`, `Sequence`, `Conditional`, resolution steps | P/T, types, colors, keywords, granted abilities, untap restrictions, play permissions, cost changes, replacements |
| Authored in TOML | Yes | No — derived from resolved effects |
| Serialized | Yes (`serde` + schemars) | Never (`characteristics.rs:22`) |
| Replayed | Yes — stashed in `SequenceCont` across a pause, replayed on answer | No — events replay, modifiers get rebuilt |
| Pattern-matched | Yes — projection, description, legal-action computation | No — only the recompute loop reads them |
| Composition shape | A tree (`[f, g]`, `then`/`otherwise`) | Function composition (layer over layer, timestamp over timestamp) |
| **Therefore** | **Data. Closures cost inspection and buy nothing.** | **Closures are viable and the set is open-ended.** |

Composition is a property of the graph, not of the closure. `f.and_then(g)` and `[f, g]` are the same graph — one opaque, one readable. In the resolution half we need it readable. In the modifier half nothing reads it.

## Locked decisions

| Decision | Choice |
|---|---|
| ECS | **Rejected.** The arena already exists; the matches are opcode dispatch, not a modelling failure; a game-loop shape conflicts with the sequential stack-and-priority commitment in `AGENTS.md`; CR 613 layering is already timestamp-ordered and correct in kind. |
| Cards authored in | TOML. Unchanged. No Rust per card in any wave. |
| Resolution representation | Data tree (enum + `serde`). Unchanged. |
| Modifier representation | One registered `Modifier` list on `Game`, replacing per-object and per-`Game` ad-hoc fields |
| Closures | Permitted **only** in the modifier half, and only after the registry lands (Wave 4, optional) |
| Layer / timestamp | Declared as data on `Modifier` even if the transform is a closure — CR 613 ordering must be readable without calling anything |
| Value channel | Named `*_this_way` tallies — the oracle wording (*"for each +1/+1 counter removed this way"*) and already the codebase convention across 8 `Amount` variants and 15 `ResolutionFrame` fields. Named, not anonymous: the name is what keeps a read unambiguous across branches and pauses, so no producer/consumer binding rider is needed |
| Trigger normalization | Out of this design — belongs to program Wave C (table-driven trigger enqueue). Same principle (one generic watch + `CardFilter`), different wave. |
| Wire | No proto change expected. If a wave needs one, follow `docs/WIRE_COMPAT.md`. |
| Sequential SM / event-sourced board facts | Unchanged. Not relitigated. |

## Approaches considered

### 1. ECS for the whole engine
Components on entities, systems iterating. **Rejected** — see Locked decisions. The `ObjectId` arena is already the entity table; systems-over-frames is the wrong execution model for stack-and-priority.

### 2. Closures everywhere (`Effect` becomes `Arc<dyn Fn>`)
Uniformly composable, no exhaustive matches. **Rejected for the resolution half.** `Effect` is deserialized from 665 TOML files, pattern-matched by projection and description, stashed in `SequenceCont` across pauses, and replayed. A closure erases all four. It also erases per-effect typed payloads (`TokenEffect::Create` carries nine named fields) into a generic bag, and moves description generation next to behavior where it can silently drift.

### 3. Split by half — generic data vocabulary + modifier registry (chosen)
Each half gets the mechanism it wants. The DSL generalization is authorable in TOML today and is independent of whether the modifier registry ever uses closures — so the waves ship in any order and Wave 4 is genuinely optional.

---

## Design

### Half 1 — Resolution: generic vocabulary

Three generalizations, each of which dissolves a family of one-off variants.

**A. Generic verbs replace fused variants.** `RemoveAllButOne…ThenGainLife` decomposes into a generic `remove` step followed by an ordinary `gain life` step. The verb carries axes (`all_kinds`, `keep`), not a card's name.

**B. Generic comparison replaces bespoke conditions.** `Condition::SourcePowerAtMost { at_most: 16 }` becomes `Condition::Compare { subject, op, amount }`. This is the same collapse `ZoneChange + CardFilter` performs on triggers, applied to the 47-variant `Condition` enum.

**C. Named `*_this_way` tallies carry values between steps.** A producing step writes a named tally on `ResolutionFrame`; a following step reads it through the matching `Amount::*ThisWay`. Both halves of a "for each … this way" clause become ordinary steps in the existing `[[abilities.effects]]` array.

**No `and_then` field.** An earlier draft bound producer to consumer with an `and_then` rider on the producing effect, to keep an anonymous `this_way` unambiguous across a branch or a pause. Naming the tally solves that instead, and the codebase already works this way: the 8 existing `*ThisWay` amounts disambiguate by name, `Effect::Conditional`'s `then`/`otherwise` are already `Arc<[Effect]>`, and each producer resets its own tally immediately before producing — so a stale or cross-branch read is impossible without a rider. Named tally + step array is strictly less machinery than anonymous value + `and_then`.

**No tally array either.** A `Tally` enum indexed into `tallies: [u32; Tally::COUNT]` was considered to stop `ResolutionFrame` growing a field per card. It does not: a `Tally` variant per card is the same churn in different syntax, and the 15 existing fields are heterogeneously typed (`Vec<_>`, `Option<(ObjectId, u32)>`, `bool`, `u32`) so they do not fit one array. Keep the per-field idiom.

**Worked example — Lily Bowen, Raging Grandma** (`crates/cards/data/lily_bowen_raging_grandma.toml`):

```toml
# Lily Bowen enters with two +1/+1 counters on it.   (unchanged by this design)
[[abilities]]
timing = "static"

# Lily Bowen enters with two +1/+1 counters on it.
[[abilities.effects]]
type = "static"
mode = "enters_with_counters"
count = 2

# At the beginning of your upkeep, double the number of +1/+1 counters on Lily Bowen if its power
# is 16 or less. Otherwise, remove all but one +1/+1 counter from it, then you gain 1 life for
# each +1/+1 counter removed this way.
[[abilities]]
timing = "upkeep"

[[abilities.effects]]
type = "conditional"
condition = { type = "compare", left = "source_power", op = "at_most", right = 16 }

then = [
    # ... double the number of +1/+1 counters on Lily Bowen ...
    { type = "counters", mode = "double_counters", target = "this" },
]

otherwise = [
    # ... remove all but one +1/+1 counter from it,
    { type = "counters", mode = "remove_counters", target = "this", keep = 1 },
    # ... then you gain 1 life for each +1/+1 counter removed this way.
    { type = "life", mode = "gain", amount = "counters_removed_this_way" },
]
```

Every verb is generic; nothing is Lily-Bowen-shaped. The `ponytail:` admission at `counters.rs:118` goes away with the variant it describes — and it prescribed exactly this fix: *"grow a `keep`/`gain_life` rider (or a `kind` axis) on `RemoveAllCountersThenDraw` instead of a new sibling."*

Two notes on what this example does **not** change. `double_counters` stays as it is: it is one coherent oracle clause with five consumers, not a fused pair, so there is nothing to decompose. And `put_counters` keeps its existing spelling rather than being renamed to `add` — 65 card files, no behavior gained. The `condition` line above is Wave 2's `Compare` vocabulary — Wave 1 shipped with the old `source_power_at_most` spelling and Wave 2 migrated it.

### Half 2 — Modifiers: one registry

```rust
struct Modifier {
    source: ObjectId,
    timestamp: u64,       // CR 613.7 same-layer ordering
    layer: Layer,         // CR 613 — data, always, even if `effect` becomes a closure
    duration: Duration,   // EndOfTurn | WhileSourceOnBattlefield | UntilNextUntap | Indefinite
    effect: ModifierEffect,
}
```

Registered when the granting effect resolves; stored on `Game`; read by the recompute loop.

What collapses:

- 13 until-EOT / boost fields on `Permanent` → gone
- 6 registry-shaped fields on `Game` → become `Modifier`s with the appropriate `Duration`
- the hand-maintained reset in `Event::TempBoostsEnded` → `modifiers.retain(|m| m.duration != Duration::EndOfTurn)`
- `Box::leak` in `apply.rs` → gone; those fields no longer need `Permanent: Copy`
- 20 `ContinuousEffect` reconstruct sites → gone; modifiers are stored, not rebuilt per query
- `Game::modifier_provenance` → gone, and **improves**: every modifier carries its own `source`, so Alt-inspect stops being *"additive attribution only — not CR 613 layers"* (`characteristics.rs:66`) and becomes the real layer stack

Determinism is preserved: modifiers are registered by events, so replay from the same intents rebuilds the same list.

**Known gap:** CR 613.8 dependency ordering needs to know what a modifier *reads*. A closure hides that. Either declare reads as data alongside the transform, or leave dependency unimplemented — it is not implemented today either. Record whichever choice a wave makes in `engine-core-and-event-model` and, if deferred, in the relevant `docs/fidelity/` increments.

---

## Wave plan

Each wave is a separate PR leaving `just check` green.

| Wave | Deliverable | Specs to update |
|---|---|---|
| **1** | `Amount::CountersRemovedThisWay` + its `ResolutionFrame` tally; generic `CountersEffect::RemoveCounters { target, all_kinds, keep }` resolving on the `Game::run` path; delete `RemoveAllCountersThenDraw` and `RemoveAllButOnePlusOneCounterThenGainLife` | `card-dsl-and-card-pool`, `choices-actions-and-resolution` (+ regenerate `DSL_REFERENCE.md` / card schema) |
| **2** | `Condition::Compare { left, op, right }` over two `&'static Amount` operands; `TriggerContext::source`/`target` so one `condition_holds` serves trigger placement, resolution, and the conditional-keyword recompute; migrate the scalar-comparison members of the 47-variant enum — including the board counts, via `Amount::PerPermanentMatching` over an ordinary `PermanentFilter` — and delete the bespoke `Effect::Conditional` arms they forced. What stays named is what a scalar comparison can't say: existentials over players, two-subject comparisons, and non-count board facts | `card-dsl-and-card-pool`, **`DSL_REFERENCE.md`** |
| **3** | `Modifier` registry on `Game`. Start with the 13 `Permanent` EOT fields — `TempBoostsEnded` and `Box::leak` both die on the first pass and prove the shape. Then the 6 `Game` fields. **This is program Wave E.** | `engine-core-and-event-model`, `card-inspect` (provenance becomes layer-accurate) |
| **4** *(optional)* | `ModifierEffect` as `Arc<dyn Fn(&Game, &mut Characteristics) + Send + Sync>` if the enum proves too narrow after Wave 3. Reversible; changes no card file. | `engine-core-and-event-model` |

Waves 1–2 are DSL-surface changes and are independent of 3–4. Wave 3 does not depend on 1–2.

**Wave 1 card migration is exactly 2 files** — the fused variants have one consumer each
(`nexus_mentality.toml` for `remove_all_counters_then_draw`, `lily_bowen_raging_grandma.toml` for
`remove_all_but_one_plus_one_counter_then_gain_life`). Each migrated card keeps a test asserting
the same outcome as before migration. Net: two variants out, one in.

**No rename waves.** `double_counters` and `put_counters` keep their spellings. 65 card files use
the counters effect, and churning them to prove a naming point is not worth one PR's risk; neither
verb is fused, so neither is what this design is about. `ponytail:` the DSL carries `put_counters`
where the design's prose says `add` — collapse them when something else already needs that diff.

**A step that produces a `*_this_way` value resolves via `Game::run`, not via minting.**
`Game::execute_effect` and `Game::mint_counters` take `&self` and cannot write the frame. Every
existing producer (`DestroyAll`, `ExileAll`, `MillSelf`) already lives on the `&mut self` `Game::run`
path for exactly this reason; `resolve_mill_self` (`crates/engine/src/resolution/mill.rs:118`) is the
template. New producers follow it, and go in `mint.rs`'s composite group, not its mint group.

**Every wave in this design is net deletion.** A wave that adds more lines than it removes has gone wrong; stop and re-scope.

---

## Testing

TDD per `AGENTS.md`. Red → green → review, at the lowest layer that catches the failure.

**Wave 1**
- Engine unit: the tally crosses the step boundary — remove N counters, gain exactly N life.
- Engine unit: `all_kinds` sweeps both counter stores (`plus_counters` and `kind_counters`) and the tally counts both.
- Engine unit: the tally reads 0, not stale, when the producer removed nothing.
- Engine unit: a tally written inside `then` does not leak into `otherwise`.
- Engine unit: `keep` exceeding the counters present removes nothing rather than underflowing.
- Card: Lily Bowen upkeep both branches — under 16 power doubles, over 16 keeps exactly one counter and gains the removed count as life.
- Schema: the JSON schema check (commit `d9203f09a`) still accepts every file in `crates/cards/data/`.

**Wave 2**
- Engine unit: one test per `op` at its boundary (`at_most` at exactly N passes, N+1 fails).
- Card regression: every card migrated off a bespoke `Condition` variant keeps a test asserting the same outcome as before migration.

**Wave 3**
- Engine unit: `EndOfTurn` modifiers are gone after cleanup and `WhileSourceOnBattlefield` ones survive it.
- Engine unit: a `WhileSourceOnBattlefield` modifier disappears when its source leaves the battlefield.
- Engine unit: CR 613.7 — two same-layer modifiers apply in timestamp order, and reversing registration order reverses the result.
- Engine unit: CR 613 layer order — a layer-7b base set registered *after* a 7c delta still applies before it.
- Determinism: same seed + same intents → same board, exercised through a `Game::clone()` fork.
- Inspect: `modifier_sources` attributes a P/T change to the correct source card after migration.
- Existing `characteristics` suite is the regression net and must stay green with no assertion edits.

**Wave 4**
- The entire Wave 3 suite must pass unchanged. If a test needs editing, the closure conversion changed behavior and is wrong.

---

## Global constraints

- Pure engine: no I/O, no networking, no wall-clock, injected RNG only.
- Guard-return-first. Readability over cleverness.
- Every bug fix gets a regression test, in the same change where possible.
- Any TOML-surface change regenerates `.agents/skills/card-dsl/DSL_REFERENCE.md` and the card JSON schema in the **same** change — `just cards-dsl-ref` + `just cards-schema`. Both files are generated; never hand-edit them. `just check` fails on stale output.
- Only `ponytail:` / `approximates` comments mark deliberate rules gaps; silence means faithful.
- Update the living surface specs in the same change as the behavior change. This design doc is design input, not a substitute.
- Angular commit subjects on squash PR titles (`refactor:` / `feat:` as appropriate).
- Verify with `just check` before claiming a wave done.

## Success criteria

- No **counters** effect variant exists solely to weld two steps together; `counters.rs:118`'s `ponytail:` note and its variant are both gone. The six remaining fused variants in `choice.rs` / `zone.rs` are multi-player sequenced effects — evaluate each on its own merits; this design does not commit to decomposing them.
- A card that needs "remove counters, then do X per counter removed" is authorable in TOML with no Rust change.
- `Permanent` carries no until-EOT fields; `Event::TempBoostsEnded` is a one-line retain.
- No `Box::leak` remains for until-EOT state.
- Alt-inspect modifier attribution reflects CR 613 layers rather than additive-only aggregation.
- Engine suite green after every wave. Same seed + intents → same board.

## Out of scope

- ECS. Decided against; do not relitigate without new evidence.
- Closures in the resolution half.
- Trigger normalization (65 variants → generic watch + `CardFilter`) — program Wave C.
- CR 614 replacement registry — program Wave F. It belongs to the modifier half and should reuse `Modifier`'s duration/timestamp machinery once Wave 3 lands.
- Full CR 613 completeness beyond pool-relevant gaps; remainder stays in `docs/fidelity/` increments.
- CR 613.8 dependency ordering, unless a wave explicitly takes it.
- Intent replay / durable game persistence.
