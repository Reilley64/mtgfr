# Choose-X Stepper Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the Arena-aligned choose-X stepper (Min/−/value/+/Max + Confirm) with a live mana-cost preview on the Foldkit board prompt.

**Architecture:** Pure helpers in `client/lib/xCost.ts` clamp X and resolve `WireCost` for a chosen value. `XPromptState.draftX` holds the draft; `XDraftSet` updates it; Confirm submits via existing `XSubmitted`. Preview reuses `costPips` from the hand bar.

**Tech Stack:** Foldkit (Html / Messages / Scene), Effect Schema messages, Vitest, TypeScript.

**Spec:** [prompts-and-pending-choices](../specs/2026-07-20-prompts-and-pending-choices.md)

## Global Constraints

- No `.proto` / engine changes — wire `min_x` / `max_x` / `x_cost` / `x_symbols` already exist.
- Guard-return-first; keep imports at top of file.
- Exhaustive `switch` on Message unions (`never` default where applicable).
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages; work on `cursor/choose-x-stepper-restore-b23c`.
- Scene tests assert product outcomes (preview, stepper, confirm), not migration parity.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/lib/xCost.ts` | `clampX`, `costWithChosenX` |
| `client/lib/xCost.test.ts` | Unit tests for helpers |
| `client/app/board/action/execution.ts` | Add `draftX` to `XPromptState` |
| `client/app/board/messages.ts` | Add `XDraftSet` |
| `client/app/board/submodel.ts` | Init `draftX`; fold `XDraftSet` |
| `client/app/board/html/prompts.ts` | Stepper UI + preview |
| `client/app/board/html/surfaces.test.ts` | Scene: stepper + preview exist |
| `client/app/board/html/prompts.test.ts` | Draft clamp + confirm submit |

---

### Task 1: Pure X cost helpers

**Files:**
- Create: `client/lib/xCost.ts`
- Create: `client/lib/xCost.test.ts`

**Interfaces:**
- Produces: `clampX(value: number, min: number, max: number): number`
- Produces: `costWithChosenX(cost: WireCost, x: number): WireCost`

- [ ] **Step 1: Write the failing test**

```ts
// client/lib/xCost.test.ts
import { describe, expect, it } from "vitest";
import { clampX, costWithChosenX } from "./xCost";

describe("clampX", () => {
  it("clamps to max", () => {
    expect(clampX(7, 0, 3)).toBe(3);
  });
  it("clamps to min", () => {
    expect(clampX(-1, 0, 3)).toBe(0);
  });
  it("returns min when max < min", () => {
    expect(clampX(2, 5, 3)).toBe(5);
  });
});

describe("costWithChosenX", () => {
  it("doubles X for Hangarback {X}{X}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 0, 0] as const, has_x: true, x_symbols: 2 };
    expect(costWithChosenX(base, 3)).toEqual({
      generic: 6,
      colored: [0, 0, 0, 0, 0],
      has_x: false,
      x_symbols: 0,
    });
  });
  it("keeps colored pips for {X}{R}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 1, 0] as const, has_x: true, x_symbols: 1 };
    expect(costWithChosenX(base, 4).generic).toBe(4);
    expect(costWithChosenX(base, 4).colored[3]).toBe(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test lib/xCost.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Write minimal implementation**

```ts
// client/lib/xCost.ts
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

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bun test lib/xCost.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/lib/xCost.ts client/lib/xCost.test.ts
git commit -m "feat(client): restore clampX and costWithChosenX helpers"
```

---

### Task 2: Draft X on BoardModel + XDraftSet

**Files:**
- Modify: `client/app/board/action/execution.ts` (`XPromptState`)
- Modify: `client/app/board/messages.ts`
- Modify: `client/app/board/submodel.ts` (`ensureXPrompt`, `updateBoard` case)
- Test: `client/app/board/html/prompts.test.ts`

**Interfaces:**
- Consumes: `clampX` from `~/xCost` (or `~/lib/xCost` — match existing `~/` alias; helpers live at `client/lib/xCost.ts` imported as `~/xCost` if `lib` is root — check: imports use `~/wire/types` so `~/xCost` is correct)
- Produces: `XPromptState.draftX: number`
- Produces: `XDraftSet({ x: number })` message

- [ ] **Step 1: Write the failing test**

Add to `client/app/board/html/prompts.test.ts`:

```ts
import { emptyCostPicks } from "../action/execution";
import { XDraftSet, XSubmitted } from "../messages";
import { clampX } from "~/xCost";
import type { ActionView, WireCost } from "~/wire/types";

function xCost(overrides: Partial<WireCost> = {}): WireCost {
  return {
    generic: 1,
    colored: [0, 0, 0, 0, 0],
    has_x: true,
    x_symbols: 1,
    ...overrides,
  };
}

function xAction(): ActionView {
  return {
    id: 12,
    kind: "cast",
    label: "Comet Storm",
    has_x: true,
    min_x: 0,
    max_x: 3,
    x_cost: xCost(),
    // …match nearby ActionView fixtures in surfaces.test.ts `action()` helper —
    // copy the minimal fields that ActionView requires in this codebase.
  } as ActionView;
}

test("XDraftSet clamps draftX into min/max", () => {
  const prompt = {
    action: xAction(),
    target: null,
    picks: emptyCostPicks(),
    modes: [],
    name: "Comet Storm",
    minX: 0,
    maxX: 3,
    draftX: 3,
    xCost: xCost(),
  };
  const board = { ...initialBoardModel(), xPrompt: prompt };
  const [next] = updateBoard(board, XDraftSet({ x: 99 }), gameFold(state()), "T1");
  expect(next.xPrompt?.draftX).toBe(3);
  const [low] = updateBoard(next, XDraftSet({ x: -5 }), gameFold(state()), "T1");
  expect(low.xPrompt?.draftX).toBe(0);
});

test("XSubmitted confirms draft X on the cast intent", () => {
  const prompt = {
    action: xAction(),
    target: null,
    picks: emptyCostPicks(),
    modes: [],
    name: "Comet Storm",
    minX: 0,
    maxX: 3,
    draftX: 2,
    xCost: xCost(),
  };
  const [, commands] = updateBoard(
    { ...initialBoardModel(), xPrompt: prompt },
    XSubmitted({ x: 2 }),
    gameFold(state()),
    "T1",
  );
  expect(commands.length).toBeGreaterThan(0);
  const intent = intentFromCommand(commands[0]);
  expect(intent).toMatchObject({ x: 2 });
});
```

Adapt `ActionView` construction to the real `action()` helper from `surfaces.test.ts` (copy that helper or import a shared fixture if one exists). Prefer duplicating the minimal fixture in `prompts.test.ts` over inventing fields.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test app/board/html/prompts.test.ts`

Expected: FAIL (`draftX` / `XDraftSet` missing)

- [ ] **Step 3: Write minimal implementation**

1. Add `draftX: number` to `XPromptState` in `execution.ts`.
2. In `messages.ts`:

```ts
export const XDraftSet = m("XDraftSet", { x: S.Number });
// add to Message union export list beside XSubmitted
```

3. In `ensureXPrompt`:

```ts
const minX = action.min_x ?? 0;
const maxX = action.max_x ?? 0;
return {
  // …
  minX,
  maxX,
  draftX: clampX(maxX, minX, maxX),
  xCost,
};
```

4. In `updateBoard` switch:

```ts
case "XDraftSet": {
  if (model.xPrompt == null) return [model, []];
  const { minX, maxX } = model.xPrompt;
  return [
    {
      ...model,
      xPrompt: { ...model.xPrompt, draftX: clampX(message.x, minX, maxX) },
    },
    [],
  ];
}
```

Fix every `XPromptState` literal in tests (`surfaces.test.ts`) to include `draftX`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bun test app/board/html/prompts.test.ts app/board/html/surfaces.test.ts`

Expected: PASS (after draftX added to Scene fixtures)

- [ ] **Step 5: Commit**

```bash
git add client/app/board/action/execution.ts client/app/board/messages.ts \
  client/app/board/submodel.ts client/app/board/html/prompts.test.ts \
  client/app/board/html/surfaces.test.ts
git commit -m "feat(client): draft choose-X value on the board prompt"
```

---

### Task 3: Stepper UI + live cost preview

**Files:**
- Modify: `client/app/board/html/prompts.ts` (`boardXPrompt`)
- Modify: `client/app/board/html/surfaces.test.ts`

**Interfaces:**
- Consumes: `clampX`, `costWithChosenX` from `~/xCost`
- Consumes: `costPips`, `costPipPlate` from `~/costPips`
- Consumes: `XDraftSet`, `XSubmitted`

- [ ] **Step 1: Write the failing Scene test**

Replace/extend the x-prompt test in `surfaces.test.ts`:

```ts
test("x prompt shows stepper controls and a live cost preview", () => {
  const xPrompt: XPromptState = {
    action: action(12, { label: "Comet Storm", has_x: true, max_x: 3, min_x: 0 }),
    target: null,
    picks: emptyCostPicks(),
    modes: [],
    name: "Comet Storm",
    minX: 0,
    maxX: 3,
    draftX: 3,
    xCost: cost({ generic: 1, has_x: true, x_symbols: 1 }),
  };
  overlayScene(
    overlayModel({ ...initialBoardModel(), xPrompt }),
    Scene.expect(Scene.testId("x-prompt")).toExist(),
    Scene.expect(Scene.testId("x-prompt-preview")).toExist(),
    Scene.expect(Scene.testId("x-prompt-min")).toExist(),
    Scene.expect(Scene.testId("x-prompt-dec")).toExist(),
    Scene.expect(Scene.testId("x-prompt-value")).toExist(),
    Scene.expect(Scene.testId("x-prompt-inc")).toExist(),
    Scene.expect(Scene.testId("x-prompt-max")).toExist(),
    Scene.expect(Scene.testId("x-prompt-confirm")).toExist(),
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test app/board/html/surfaces.test.ts -t "x prompt"`

Expected: FAIL (missing testids)

- [ ] **Step 3: Write minimal implementation**

Replace `boardXPrompt` with stepper + preview. Reuse hand pip rendering pattern (inline small `costPipView` or import from hand if exported — if not exported, duplicate a 10-line local helper in `prompts.ts`):

```ts
function boardXPrompt(prompt: NonNullable<BoardModel["xPrompt"]>): Html {
  const { minX, maxX, draftX, xCost, name } = prompt;
  const preview = costWithChosenX(xCost, draftX);
  const pips = costPips(preview, { showZero: true });
  return frame("x-prompt", `Choose X for ${name}`, [
    h.div(
      [h.Class("mb-sm flex items-center justify-center gap-2 text-body text-mist"), h.DataAttribute("testid", "x-prompt-preview")],
      ["Pay ", ...pips.map((pip) => /* cost pip span */)],
    ),
    h.div(
      [h.Class("flex flex-wrap items-center justify-center gap-2")],
      [
        itemButton("Min", "x-prompt-min", XDraftSet({ x: minX })),
        // dec/inc as buttons with Disabled when at bounds
        h.span([h.DataAttribute("testid", "x-prompt-value"), h.Class("min-w-[2ch] text-center text-body text-snow")], [String(draftX)]),
        itemButton("Max", "x-prompt-max", XDraftSet({ x: maxX })),
      ],
    ),
    itemButton("Confirm", "x-prompt-confirm", XSubmitted({ x: draftX })),
    cancelButton(),
  ]);
}
```

Wire `XDraftSet` into the prompts imports. Match existing button/disabled styling (`itemButton` or ghost variants used nearby).

- [ ] **Step 4: Run tests**

Run: `cd client && bun test app/board/html/surfaces.test.ts app/board/html/prompts.test.ts lib/xCost.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/prompts.ts client/app/board/html/surfaces.test.ts
git commit -m "feat(client): restore choose-X stepper with live cost preview"
```

---

### Task 4: Verify + ship

**Files:** none new

- [ ] **Step 1: Format / typecheck / focused tests**

Run:

```bash
cd client && bun run format && bun run lint && bun run typecheck
cd client && bun test lib/xCost.test.ts app/board/html/prompts.test.ts app/board/html/surfaces.test.ts
```

Expected: all green

- [ ] **Step 2: Push and update PR**

```bash
git push -u origin cursor/choose-x-stepper-restore-b23c
```

- [ ] **Step 3: Mark plan checkboxes done in this file and commit if desired**

Optional docs-only commit; skip if noisy.

---

## Spec coverage self-review

| Spec requirement | Task |
|------------------|------|
| `clampX` / `costWithChosenX` | Task 1 |
| `draftX` + `XDraftSet` | Task 2 |
| Stepper UI + Pay preview via `costPips` | Task 3 |
| Scene + unit tests | Tasks 1–3 |
| No proto/engine changes | All tasks |
| Feature spec already updated | Prior docs commit |
