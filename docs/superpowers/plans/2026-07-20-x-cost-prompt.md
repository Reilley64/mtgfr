# Choose-X Cast Prompt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the choose-X cast/activate prompt so only engine-legal X values are selectable, with Min/−/+/Max controls and a live mana-cost preview (Arena-aligned server min/max).

**Architecture:** Expand wire with `WireCost.x_symbols` and `ActionView.{min_x,max_x,x_cost}`; engine computes `max_x` via the same affordability path as playability; client `XPromptModal` becomes a clamped stepper that previews `cost.with_x(x)` and submits the existing cast intent `x` field.

**Tech Stack:** Rust engine + schema/proto (prost/tonic), Effect-gRPC codegen (`just server-codegen`), SolidStart client (Solid signals), Vitest + cargo-nextest.

**Spec:** [docs/superpowers/specs/2026-07-20-x-cost-prompt-design.md](../specs/2026-07-20-x-cost-prompt-design.md)

## Global Constraints

- Expand-only proto fields ([docs/WIRE_COMPAT.md](../../WIRE_COMPAT.md)) — never reuse/remove field numbers; keep `has_x`.
- Engine stays pure; no I/O in affordability helpers.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages (`feat:`, `fix:`, `test:`, …); PRs squash-merge.
- Work on a feature branch off current `main` (do not commit straight to `main` for implementation).
- Skip v1 denylist of sparse illegal X values (Living Breakthrough).
- Skip oracle-derived “enters as N/N” hint unless trivial; cost preview is required.

---

## File map

| File | Responsibility |
|------|----------------|
| `proto/mtgfr/v1/common.proto` | Add `WireCost.x_symbols` |
| `proto/mtgfr/v1/stream.proto` | Add `ActionView.min_x`, `max_x`, `x_cost` |
| `crates/schema/src/dto.rs` | Mirror new fields on `WireCost` / `ActionView` |
| `crates/schema/src/catalog.rs` | `wire_cost` sets `x_symbols` |
| `crates/engine/src/priority.rs` (or new `x_choice.rs`) | `Game::max_payable_x` / affordability for chosen X |
| `crates/schema/src/snapshot.rs` | Project min/max/`x_cost` onto X-bearing actions |
| `crates/schema/src/snapshot.rs` tests | Hangarback max_x=3 with 7 mana, etc. |
| `client/` codegen | `just server-codegen` |
| `client/src/wire/types.ts` | Hand types if not fully generated |
| `client/src/lib/xCost.ts` | Clamp + preview helpers (pure) |
| `client/src/lib/xCost.test.ts` | Unit tests for helpers |
| `client/src/controllers/prompt-host.tsx` | Redesigned `XPromptModal` |
| `client/src/controllers/actionExecution.ts` | Pass action bounds into X prompt |
| `client/src/controllers/action-chrome.tsx` | Wire new modal props |

---

### Task 1: Wire — `x_symbols` on `WireCost`

**Files:**
- Modify: `proto/mtgfr/v1/common.proto`
- Modify: `crates/schema/src/dto.rs` (`WireCost`)
- Modify: `crates/schema/src/catalog.rs` (`wire_cost`)
- Test: `crates/schema/src/snapshot.rs` (extend existing `cost_of` / has_x tests) or `crates/schema/src/catalog.rs` unit test if present

**Interfaces:**
- Produces: `WireCost { generic, colored, has_x, x_symbols }` where `x_symbols` mirrors `engine::Cost.x`, `has_x == (x_symbols > 0)`

- [ ] **Step 1: Write the failing test**

In `crates/schema/src/snapshot.rs` tests (near existing `cost_of(insight).has_x` assertions), add:

```rust
#[test]
fn wire_cost_carries_x_symbol_count() {
    // Hangarback Walker is {X}{X} — Cost.x == 2.
    let def = cards::lookup("Hangarback Walker").expect("card in pool");
    let w = crate::catalog::wire_cost_for_test(def.cost); // see step 3 if wire_cost is pub(crate)
    assert_eq!(w.x_symbols, 2);
    assert!(w.has_x);
    let shock = cards::lookup("Shock").expect("card in pool");
    let s = crate::catalog::wire_cost_for_test(shock.cost);
    assert_eq!(s.x_symbols, 0);
    assert!(!s.has_x);
}
```

If `wire_cost` stays `pub(crate)`, put the test in `catalog.rs` `#[cfg(test)]` module instead and call `wire_cost` directly — prefer that to avoid a test-only export:

```rust
// crates/schema/src/catalog.rs
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_cost_carries_x_symbol_count() {
        let hangar = cards::lookup("Hangarback Walker").expect("in pool");
        let w = wire_cost(hangar.cost);
        assert_eq!(w.x_symbols, 2);
        assert!(w.has_x);
    }
}
```

(Use whatever card lookup this crate already uses in nearby tests — match existing `cards::` / `load` patterns in `snapshot.rs` tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --profile ci wire_cost_carries_x_symbol_count`

Expected: FAIL (field missing / compile error, or assertion on missing field)

- [ ] **Step 3: Proto + DTO + `wire_cost`**

`proto/mtgfr/v1/common.proto` — add field 4:

```protobuf
message WireCost {
  uint32 generic = 1;
  repeated uint32 colored = 2;
  bool has_x = 3;
  // Number of `{X}` symbols in the cost (0 = none). CR 107.3: each symbol is paid once.
  uint32 x_symbols = 4;
}
```

`crates/schema/src/dto.rs` — extend `WireCost`:

```rust
pub struct WireCost {
    pub generic: u8,
    pub colored: [u8; 5],
    #[serde(default)]
    pub has_x: bool,
    /// Number of `{X}` symbols (`engine::Cost.x`).
    #[serde(default)]
    pub x_symbols: u8,
}
```

`catalog.rs`:

```rust
pub(crate) fn wire_cost(cost: engine::Cost) -> WireCost {
    WireCost {
        generic: cost.generic,
        colored: cost.colored,
        has_x: cost.x > 0,
        x_symbols: cost.x,
    }
}
```

Update every `WireCost { ... }` literal in schema tests to include `x_symbols: 0` (or rely on `Default` if you add `#[derive(Default)]` — prefer explicit in tests that construct structs).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --profile ci wire_cost_carries_x_symbol_count`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add proto/mtgfr/v1/common.proto crates/schema/src/dto.rs crates/schema/src/catalog.rs
git commit -m "$(cat <<'EOF'
feat(schema): add WireCost.x_symbols for {X} count

EOF
)"
```

---

### Task 2: Engine — `max_payable_x` helper

**Files:**
- Create or modify: `crates/engine/src/priority.rs` (next to `affordable_from` / `available_mana`) — prefer a focused `pub(crate) fn max_payable_x_for_cost` on `Game`
- Test: `crates/engine/tests/game.rs` or a unit test module in `priority.rs`

**Interfaces:**
- Consumes: `Game::available_mana`, `Game::affordable_from`, `Cost::with_x`, `cast_cost` (callers supply a closure or prebuilt base)
- Produces:

```rust
impl Game {
    /// Largest X such that `available_mana` can pay `cost_at(x)`, or `0` when even X=0 fails
    /// only if caller still lists the action — callers use this for projection.
    pub(crate) fn max_payable_x(
        &self,
        player: PlayerId,
        spell: Option<SpellCharacteristics>,
        mut cost_at: impl FnMut(u32) -> Cost,
    ) -> u32 { ... }
}
```

Semantics (must match spec):

1. If `cost_at(0)` is free mana (`Cost` with all mana zeros) **and** `cost_at(1)` equals `cost_at(0)` (X does not increase mana — free-cast / `pay_life_x` mana fold), then:
   - If base has `pay_life_x`: return `self.players[player].life.max(0) as u32` (life is the X payment).
   - Else (true free cast): return `0`.
2. Otherwise binary-search / exponential probe the largest `x` where `affordable_from(available_mana(player), cost_at(x), spell)`.
3. Upper bound probe: start from `1` and double until unaffordable or cap at `255` (generic saturates at u8); then binary search. For `pay_life_x`, also clamp to life.

- [ ] **Step 1: Write the failing test**

Add in `crates/engine/tests/game.rs` (follow local Game harness patterns — seed lands/mana, put Hangarback in hand):

```rust
#[test]
fn max_payable_x_for_hangarback_with_seven_mana_is_three() {
    // Arrange: P0 has 7 generic available (pool or seven Forests + green filter — use the
    // harness helper this file already uses for "fund N mana", e.g. fund_mana / add to pool).
    let mut game = /* existing two-player setup */;
    // Put Hangarback Walker in hand; ensure it's cast_listable.
    // Act
    let def = game.def_of(hangarback_id);
    let max = game.max_payable_x(PlayerId(0), Some(def.spell_characteristics()), |x| {
        game.cast_cost(PlayerId(0), hangarback_id, def, None, x, Zone::Hand, 0, false, false, false, 0, 0)
    });
    assert_eq!(max, 3);
}
```

Also add:

```rust
#[test]
fn max_payable_x_for_fireball_respects_red_pip() {
    // 5 generic + 0 red → max 0 if {X}{R} and no red; 5 generic + 1 red → max 5.
}
```

Use real card names from the pool (`Blaze` / `Hangarback Walker` / whatever the harness already loads).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --profile ci max_payable_x_for_hangarback`

Expected: FAIL (method missing)

- [ ] **Step 3: Implement `max_payable_x`**

```rust
pub(crate) fn max_payable_x(
    &self,
    player: PlayerId,
    spell: Option<SpellCharacteristics>,
    mut cost_at: impl FnMut(u32) -> Cost,
) -> u32 {
    let available = self.available_mana(player);
    let at0 = cost_at(0);
    let at1 = cost_at(1);
    let x_affects_mana = at0.generic != at1.generic
        || at0.colored != at1.colored
        || at0.colorless != at1.colorless;
    if !x_affects_mana {
        if at0.additional.pay_life_x {
            return self.players[player.0 as usize].life.max(0) as u32;
        }
        return 0;
    }
    if !Self::affordable_from(available, at0, spell) {
        return 0;
    }
    let mut hi = 1u32;
    while hi < 255 {
        let next = hi.saturating_mul(2).min(255);
        if !Self::affordable_from(available, cost_at(next), spell) {
            break;
        }
        if next == hi {
            break;
        }
        hi = next;
    }
    // binary search in [0, hi] for largest affordable
    let mut lo = 0u32;
    let mut best = 0u32;
    let mut bound = hi;
    while lo <= bound {
        let mid = lo + (bound - lo) / 2;
        if Self::affordable_from(available, cost_at(mid), spell) {
            best = mid;
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            bound = mid - 1;
        }
    }
    best
}
```

Refine against the failing tests (probe logic may need tweaks when `hi` itself is affordable).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run --profile ci max_payable_x`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine/src/priority.rs crates/engine/tests/game.rs
git commit -m "$(cat <<'EOF'
feat(engine): compute max payable X for cast costs

EOF
)"
```

---

### Task 3: Schema — project `min_x` / `max_x` / `x_cost` on actions

**Files:**
- Modify: `proto/mtgfr/v1/stream.proto` (`ActionView`)
- Modify: `crates/schema/src/dto.rs` (`ActionView`)
- Modify: `crates/schema/src/snapshot.rs` (`action_view` and every `ActionView { ... }` arm)
- Test: `crates/schema/src/snapshot.rs` tests

**Interfaces:**
- Consumes: `Game::max_payable_x`, `wire_cost`
- Produces: on X-bearing actions:

```rust
has_x: true,
min_x: 0, // v1 always 0 unless free-cast forces 0..=0
max_x: game.max_payable_x(...),
x_cost: Some(wire_cost(paid_base_cost_without_folding_x)),
```

Proto (new field numbers after `required_attacks = 18`):

```protobuf
  uint32 min_x = 19;
  uint32 max_x = 20;
  // Mana cost being paid for preview (printed / flashback / back face / activation).
  // Absent when has_x is false.
  optional WireCost x_cost = 21;
```

DTO:

```rust
#[serde(default)]
pub min_x: u32,
#[serde(default)]
pub max_x: u32,
#[serde(default)]
pub x_cost: Option<WireCost>,
```

Default for non-X actions: `min_x: 0`, `max_x: 0`, `x_cost: None`.

Helper inside `snapshot.rs`:

```rust
fn x_choice_fields(
    game: &engine::Game,
    player: engine::PlayerId,
    paid: engine::Cost,
    spell: Option<engine::SpellCharacteristics>,
    cost_at: impl FnMut(u32) -> engine::Cost,
) -> (bool, u32, u32, Option<WireCost>) {
    if paid.x == 0 {
        return (false, 0, 0, None);
    }
    let max_x = game.max_payable_x(player, spell, cost_at);
    (true, 0, max_x, Some(wire_cost(paid)))
}
```

For `Cast`: `paid` = base cost before `with_x` (same source as today’s `has_x` — printed / flashback / escape). `cost_at` closes over `cast_cost(..., x, ...)`.

For `CastPrepared`: `paid = back_def.cost`.

For `Activate` with X in activation mana: `paid = ability.cost` (mana portion); `cost_at = |x| ability.cost.with_x(x)` (plus any activate-specific reducers if they exist — match payment path).

Free cast: `cast_cost` returns `FREE` → `max_payable_x` returns `0` → `min_x == max_x == 0`.

- [ ] **Step 1: Write the failing projection test**

```rust
#[test]
fn hangarback_cast_action_projects_max_x_from_available_mana() {
    // Build a VisibleState / action list where P0 can produce 7 mana and holds Hangarback.
    // Assert the cast ActionView: has_x, x_cost.x_symbols == 2, max_x == 3, min_x == 0.
}
```

Follow existing snapshot tests that call `action_view` / `visible` with a seeded `Game` (see `a_cast_action_lists_auto_tap_permanents`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --profile ci hangarback_cast_action_projects_max_x`

Expected: FAIL

- [ ] **Step 3: Proto, DTO, fill every `ActionView` literal, implement projection**

Update all ~11 `ActionView { ... }` arms with the new fields (non-X → zeros/`None`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --profile ci hangarback_cast_action_projects_max_x`

Expected: PASS

Also run: `cargo nextest run --profile ci an_activate_action_with_x_in_its_cost_carries_has_x`

Expected: PASS (extend assertion to `max_x` if easy)

- [ ] **Step 5: Commit**

```bash
git add proto/mtgfr/v1/stream.proto crates/schema/src/dto.rs crates/schema/src/snapshot.rs
git commit -m "$(cat <<'EOF'
feat(schema): project min_x/max_x/x_cost on X actions

EOF
)"
```

---

### Task 4: Client codegen + pure X helpers

**Files:**
- Run: `just server-codegen`
- Modify: `client/src/wire/types.ts` if hand-maintained types need `x_symbols` / `min_x` / `max_x` / `x_cost`
- Create: `client/src/lib/xCost.ts`
- Create: `client/src/lib/xCost.test.ts`
- Modify: `client/src/api/generated.ts` only if codegen does not own it — prefer codegen output

**Interfaces:**
- Produces:

```ts
export function clampX(value: number, min: number, max: number): number;
export function costWithChosenX(cost: WireCost, x: number): WireCost;
// costWithChosenX: generic = cost.generic + x * (cost.x_symbols ?? 0), has_x false, x_symbols 0
```

- [ ] **Step 1: Write the failing test**

```ts
// client/src/lib/xCost.test.ts
import { describe, expect, it } from "vitest";
import { clampX, costWithChosenX } from "~/lib/xCost";

describe("clampX", () => {
  it("clamps to max", () => {
    expect(clampX(7, 0, 3)).toBe(3);
  });
  it("clamps to min", () => {
    expect(clampX(-1, 0, 3)).toBe(0);
  });
});

describe("costWithChosenX", () => {
  it("doubles X for Hangarback {X}{X}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 2 };
    expect(costWithChosenX(base, 3)).toEqual({
      generic: 6,
      colored: [0, 0, 0, 0, 0],
      has_x: false,
      x_symbols: 0,
    });
  });
  it("keeps colored pips for {X}{R}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 1, 0], has_x: true, x_symbols: 1 };
    expect(costWithChosenX(base, 4).generic).toBe(4);
    expect(costWithChosenX(base, 4).colored[3]).toBe(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test src/lib/xCost.test.ts`

Expected: FAIL (module missing)

- [ ] **Step 3: Implement helpers + codegen**

```ts
// client/src/lib/xCost.ts
import type { WireCost } from "~/wire/types";

export function clampX(value: number, min: number, max: number): number {
  if (max < min) return min;
  const n = Math.floor(Number.isFinite(value) ? value : min);
  return Math.min(max, Math.max(min, n));
}

export function costWithChosenX(cost: WireCost, x: number): WireCost {
  const symbols = cost.x_symbols ?? (cost.has_x ? 1 : 0);
  return {
    generic: cost.generic + clampX(x, 0, Number.MAX_SAFE_INTEGER) * symbols,
    colored: [...cost.colored] as WireCost["colored"],
    has_x: false,
    x_symbols: 0,
  };
}
```

Update `WireCost` / `ActionView` in `client/src/wire/types.ts` to match proto. Run `just server-codegen`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bun test src/lib/xCost.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/src/lib/xCost.ts client/src/lib/xCost.test.ts client/src/wire/types.ts
# plus codegen outputs if they are committed in this repo — they are gitignored; do not force-add
git commit -m "$(cat <<'EOF'
feat(client): add clampX and costWithChosenX helpers

EOF
)"
```

---

### Task 5: Redesign `XPromptModal` stepper UI

**Files:**
- Modify: `client/src/controllers/prompt-host.tsx`
- Modify: `client/src/controllers/actionExecution.ts` (xPrompt signal shape)
- Modify: `client/src/controllers/action-chrome.tsx`
- Test: extend `client/src/controllers/action-session.test.ts` if it asserts xPrompt shape; add a small solid-test or unit-test the clamp-on-input via exporting a `normalizeXInput` if needed

**Interfaces:**
- Consumes: `clampX`, `costWithChosenX`, `costText` from `prompt-forms.tsx`
- Produces: updated prompt API:

```ts
export function XPromptModal(props: {
  name: string;
  minX: number;
  maxX: number;
  xCost: WireCost; // paid face; x_symbols set
  onSubmit: (x: number) => void;
  onCancel: () => void;
}): JSX.Element;
```

`actionExecution` xPrompt signal:

```ts
{
  name: string;
  minX: number;
  maxX: number;
  xCost: WireCost;
  submit: (x: number) => void;
}
```

When opening:

```ts
if (action.has_x && x === undefined) {
  const xCost = action.x_cost ?? { generic: 0, colored: [0,0,0,0,0], has_x: true, x_symbols: 1 };
  setXPrompt({
    name: ...,
    minX: action.min_x ?? 0,
    maxX: action.max_x ?? 0,
    xCost,
    submit: (chosen) => takeCastAction(action, target, chosen, modes, picks),
  });
  return;
}
```

UI structure (reuse `Modal`, `Button`, `Field`, `PROMPT_TITLE`, `PROMPT_ROW`):

```tsx
export function XPromptModal(props: { ... }) {
  const min = () => props.minX;
  const max = () => props.maxX;
  const [x, setX] = createSignal(clampX(props.maxX, min(), max())); // default max
  const setClamped = (n: number) => setX(clampX(n, min(), max()));
  const preview = () => costText(costWithChosenX(props.xCost, x()));
  return (
    <Modal class="fixed top-[45%] left-1/2 z-30 -translate-x-1/2 -translate-y-1/2">
      <div class={PROMPT_TITLE}>Choose X for {props.name}</div>
      <div class="mb-sm text-[var(--mist)]">Pay {preview()}</div>
      <div class={PROMPT_ROW}>
        <Button type="button" variant="ghost" onClick={() => setClamped(min())}>Min</Button>
        <Button type="button" onClick={() => setClamped(x() - 1)} disabled={x() <= min()}>−</Button>
        <Field
          ref={(el) => setTimeout(() => el.select())}
          type="number"
          min={String(min())}
          max={String(max())}
          value={x()}
          onInput={(e) => setClamped(Number(e.currentTarget.value))}
          onKeyDown={(e) => e.key === "Enter" && props.onSubmit(x())}
          class="w-[70px]"
        />
        <Button type="button" onClick={() => setClamped(x() + 1)} disabled={x() >= max()}>+</Button>
        <Button type="button" variant="ghost" onClick={() => setClamped(max())}>Max</Button>
        <Button type="button" onClick={() => props.onSubmit(x())}>Cast</Button>
        <Button type="button" variant="ghost" onClick={props.onCancel}>Cancel</Button>
      </div>
    </Modal>
  );
}
```

Esc → cancel: if `Modal`/dialog already handles it, reuse; else `onKeyDown` on container for `Escape`.

- [ ] **Step 1: Write / update failing client test**

Update `action-session.test.ts` (or add `prompt-host` test) so setting xPrompt requires the new fields and chrome still renders Cancel. Prefer a pure assertion that `clampX(7,0,3)===3` already covered; for UI, assert `actionExecution` stores `maxX` from the action:

```ts
it("opens X prompt with action max_x", async () => {
  // stub action { has_x: true, min_x: 0, max_x: 3, x_cost: { generic: 0, colored: [...], x_symbols: 2 } }
  // call takeCastAction without x
  // expect execution.xPrompt()?.maxX).toBe(3)
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test src/controllers/action-session.test.ts`

Expected: FAIL on new expectations

- [ ] **Step 3: Implement modal + wire through chrome/execution**

- [ ] **Step 4: Run tests**

Run: `cd client && bun test src/controllers/action-session.test.ts src/lib/xCost.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/src/controllers/prompt-host.tsx client/src/controllers/actionExecution.ts client/src/controllers/action-chrome.tsx client/src/controllers/action-session.test.ts
git commit -m "$(cat <<'EOF'
feat(client): clamped Min/−/+/Max choose-X prompt with cost preview

EOF
)"
```

---

### Task 6: End-to-end verification

**Files:** none new — run suites and fix any fallout from struct literals / proto maps

- [ ] **Step 1: Server tests**

Run: `cargo nextest run --profile ci max_payable_x hangarback_cast_action_projects wire_cost_carries`

Expected: PASS

- [ ] **Step 2: Broader schema/engine smoke**

Run: `cargo nextest run --profile ci has_x`

Expected: PASS

- [ ] **Step 3: Client check**

Run: `just server-codegen && cd client && bun test src/lib/xCost.test.ts src/controllers/action-session.test.ts`

Expected: PASS

- [ ] **Step 4: Manual success criteria (optional if live game available)**

With Hangarback and 7 mana: Max = 3, preview `{6}`, typing 7 snaps to 3.

- [ ] **Step 5: Final commit only if fixes were needed**

```bash
git commit -m "$(cat <<'EOF'
fix: tidy X-prompt fallout from wire expansion

EOF
)"
```

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| `WireCost.x_symbols` | 1 |
| Keep `has_x` | 1, 3 |
| `min_x` / `max_x` on actions | 3 |
| Server-authoritative max via affordability | 2, 3 |
| Cost preview | 4, 5 |
| Min/−/+/Max stepper, default max | 5 |
| Clamp typing | 4, 5 |
| Free-cast → 0..=0 | 2, 3 |
| Hangarback 7 mana → max 3 | 2, 3, 6 |
| `{X}{R}` colored preview | 4, 5 |
| No denylist v1 | (non-goal) |
| Arena-aligned contract | 2, 3 |

## Placeholder / consistency self-review

- No TBD steps; card lookup / harness setup in Task 2 must follow **existing** `game.rs` helpers in that file (copy the nearest `fund_mana` / hand-seed pattern rather than inventing new ones).
- Field names consistent: `x_symbols`, `min_x`, `max_x`, `x_cost` across proto, DTO, TS.
- `costWithChosenX` uses `x_symbols`, not a guessed `has_x ? 1 : 0` except as fallback when old snapshots omit the field mid-roll.
