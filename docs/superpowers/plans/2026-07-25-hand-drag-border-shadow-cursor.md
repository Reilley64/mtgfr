# Hand Drag Border, Flight Shadow, and Cursors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While dragging from the action bar, move the playable border onto the ghost; deepen Arena-like lift shadows for ghost + canvas flights; show not-allowed / grab / grabbing cursors.

**Architecture:** View/CSS + mount side effects — no new cursor enum on the board model. Source tile drops playable aura while `dragSource`; ghost uses `barZoneAura(zone, true)` with the dragged zone. Hand-drag mount sets/clears `document.documentElement` grabbing cursor. Token + `paintFlightCard` constants deepen the lift shadow.

**Tech Stack:** TypeScript, Vitest, Foldkit Html/Mount, DTCG tokens (`design.tokens.json` → `bun run gen:tokens`).

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-25-hand-drag-border-shadow-cursor-design.md`
- Border: source loses playable aura while dragging; ghost gets `barZoneAura(zone, true)`; no ring on canvas flights
- Cursors: unplayable `cursor-not-allowed`, playable `cursor-grab`, active drag `cursor-grabbing` on `document.documentElement`, cleared on end/cancel
- Shadows: deepen `--drop-shadow-drag` to `0 16px 36px rgb(0 0 0 / 0.72)`; flights use blur `28`, offsetY `12`, color `rgba(0,0,0,0.55)` (exported constants)
- No engine/wire; no flight playable rings; no drag-threshold changes
- TDD; Angular commits (`feat(client):`, `fix(client):`, `test(client):`, `docs:`, `style:`)
- Branch: `cursor/hand-drag-chrome-b23c`

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/messages.ts` | Optional `zone` on `HandDragStarted` |
| `client/app/board/submodel.ts` | `HandDragState.zone`; copy from message |
| `client/app/board/html/hand-drag-mount.ts` | Read `data-bar-zone`; set/clear grabbing cursor |
| `client/app/board/html/hand.ts` | Source aura off while drag; ghost aura + zone; hit cursors |
| `client/app/board/html/hand.test.ts` | Border + cursor class contracts |
| `client/app/board/hand-drag.test.ts` | Payload / state zone if needed |
| `design.tokens.json` | Deeper `drop-shadow.drag` |
| `client/styles/tokens.generated.css` | Regenerated |
| `client/lib/design-tokens.test.ts` | Expected CSS string |
| `client/app/board/bitmap/paint-flights.ts` | Stronger flight shadow constants |
| `client/app/board/bitmap/paint-flights.test.ts` | Create — lock shadow constants |
| `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md` | Behavior truth |
| `docs/superpowers/specs/2026-07-20-flights.md` | Flight shadow + out-of-scope tweak |

---

### Task 1: Playable border follows ghost + idle cursors

**Files:**
- Modify: `client/app/board/messages.ts`
- Modify: `client/app/board/submodel.ts`
- Modify: `client/app/board/html/hand-drag-mount.ts` (zone on payload only; grabbing is Task 2)
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/html/hand.test.ts`
- Modify: `client/app/board/hand-drag.test.ts` / `surfaces.test.ts` fixtures if they construct `HandDragStarted` / `handDrag`

**Interfaces:**
- Consumes: `data-bar-zone` on hit strips; `barZoneAura`
- Produces: `HandDragStarted.zone?: "hand" | "command" | "graveyard" | "exile"`; `HandDragState.zone` same

- [ ] **Step 1: Write failing tests**

In `hand.test.ts`:

```ts
describe("handView drag chrome", () => {
  it("moves playable border from source to ghost while dragging", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const cast = action(7, { object: 42 });
    const tree = handView({
      state: state({ objects: [castable], actions: [cast] }),
      hiddenId: null,
      flyingIds: new Set(),
      hiddenIds: new Set(),
      handDrag: {
        action: cast,
        name: "Lightning Bolt",
        print: "",
        manaCost: cost(),
        zone: "hand",
        x: 10,
        y: 10,
      },
    });
    const source = findTestId(tree, "hand-card-face-42");
    expect(className(source)).not.toContain("ring-playable-border");
    expect(treeHasClass(source, "opacity-25")).toBe(true);
    const ghost = findTestId(tree, "hand-drag-ghost");
    expect(ghost).not.toBeNull();
    expect(treeHasClass(ghost, "ring-playable-border")).toBe(true);
  });

  it("uses not-allowed on unplayable and grab on playable hit strips", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [castable, uncastable], actions: [action(7, { object: 42 })] }));
    const playableHit = findTestId(tree, "hand-card-42");
    const unplayableHit = findTestId(tree, "hand-card-43");
    expect(className(playableHit)).toContain("cursor-grab");
    expect(className(playableHit)).not.toContain("cursor-not-allowed");
    expect(className(unplayableHit)).toContain("cursor-not-allowed");
    expect(className(unplayableHit)).not.toContain("cursor-grab");
  });
});
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/hand.test.ts -t "drag chrome|not-allowed"`

Expected: FAIL — ghost lacks playable ring and/or source still has it; unplayable still `cursor-default`.

- [ ] **Step 3: Implement**

1. `messages.ts` — add optional zone:

```ts
export const HandDragStarted = m("HandDragStarted", {
  action: ActionView,
  name: S.String,
  print: S.String,
  manaCost: S.Any,
  kind: S.optional(S.String),
  zone: S.optional(S.Literal("hand", "command", "graveyard", "exile")),
  x: S.Number,
  y: S.Number,
});
```

2. `HandDragState` — add `zone?: "hand" | "command" | "graveyard" | "exile"`.

3. `HandDragStarted` case in `submodel.ts` — copy `zone: message.zone`.

4. `readHandDragPayload` — set zone from `hit.dataset.barZone` when it is one of the four bar zones; else from `action.section` when it is a bar zone; else `"hand"`.

5. `hand.ts` face chrome — when `dragSource`, use empty aura (not `barZoneAura(zone, playable)`):

```ts
const faceChromeClass = [
  "relative origin-bottom rounded-game",
  discardSelected
    ? "ring-2 ring-llanowar shadow-[0_0_12px_rgba(47,125,70,0.55)]"
    : discardSelectable
      ? "ring-2 ring-island-blue shadow-[0_0_12px_rgba(74,158,255,0.45)]"
      : dragSource
        ? ""
        : barZoneAura(zone, playable),
]
  .filter((v) => v !== "")
  .join(" ");
```

6. `handDragGhost` — use zone:

```ts
const zone = drag.zone ?? "hand";
const aura = barZoneAura(zone, true);
// artClass / placeholder: `... drop-shadow-drag shadow-hand ${aura}`
```

7. Hit cursor:

```ts
playable ? "cursor-grab" : "cursor-not-allowed",
```

Update fixtures that build `handDrag` / `HandDragStarted` to include `zone: "hand"` where TypeScript requires it (optional field — only if tests need ghost aura zone).

- [ ] **Step 4: Run — expect PASS**

Run: `cd client && bunx vitest run app/board/html/hand.test.ts app/board/hand-drag.test.ts app/board/html/surfaces.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/messages.ts client/app/board/submodel.ts \
  client/app/board/html/hand-drag-mount.ts client/app/board/html/hand.ts \
  client/app/board/html/hand.test.ts client/app/board/hand-drag.test.ts \
  client/app/board/html/surfaces.test.ts
git commit -m "feat(client): move playable border to hand drag ghost"
```

---

### Task 2: Grabbing cursor during hand drag

**Files:**
- Modify: `client/app/board/html/hand-drag-mount.ts`
- Create or modify: `client/app/board/html/hand-drag-mount.test.ts` (create if missing)

**Interfaces:**
- Consumes: pointerdown start / teardown paths already in the mount
- Produces: `document.documentElement.style.cursor = "grabbing"` while drag listeners are armed; cleared in `teardown` and acquireRelease cleanup

- [ ] **Step 1: Write failing test**

Prefer a focused unit that exercises the mount’s cursor helpers if you extract them; otherwise test pure helpers:

```ts
// In hand-drag-mount.ts
export function setHandDragGrabbingCursor(active: boolean): void {
  if (typeof document === "undefined") return;
  document.documentElement.style.cursor = active ? "grabbing" : "";
}
```

```ts
// hand-drag-mount.test.ts
import { afterEach, describe, expect, it } from "vitest";
import { setHandDragGrabbingCursor } from "./hand-drag-mount";

describe("setHandDragGrabbingCursor", () => {
  afterEach(() => {
    document.documentElement.style.cursor = "";
  });

  it("sets grabbing and clears", () => {
    setHandDragGrabbingCursor(true);
    expect(document.documentElement.style.cursor).toBe("grabbing");
    setHandDragGrabbingCursor(false);
    expect(document.documentElement.style.cursor).toBe("");
  });
});
```

Wire `setHandDragGrabbingCursor(true)` immediately after a successful `HandDragStarted` offer in `onPointerDown`, and `setHandDragGrabbingCursor(false)` inside `teardown` and the acquireRelease cleanup (after `handle.teardown()`).

- [ ] **Step 2: Run — expect FAIL**

Run: `cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts`

Expected: FAIL until helper + wiring exist.

- [ ] **Step 3: Implement** as above. Do not leave grabbing cursor set if payload is null after pointerdown.

- [ ] **Step 4: Run — expect PASS**

Run: `cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/hand-drag.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/hand-drag-mount.ts client/app/board/html/hand-drag-mount.test.ts
git commit -m "feat(client): use grabbing cursor during hand drag"
```

---

### Task 3: Lift shadows + module specs + verify

**Files:**
- Modify: `design.tokens.json`
- Regenerate: `client/styles/tokens.generated.css` (+ any sibling generated token files `gen:tokens` writes)
- Modify: `client/lib/design-tokens.test.ts`
- Modify: `client/app/board/bitmap/paint-flights.ts`
- Create: `client/app/board/bitmap/paint-flights.test.ts`
- Modify: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`
- Modify: `docs/superpowers/specs/2026-07-20-flights.md`

**Interfaces:**
- Produces: exported flight shadow constants for tests

- [ ] **Step 1: Token + flight constant tests (failing)**

In `design.tokens.json`:

```json
"drag": { "$type": "css", "$value": "0 16px 36px rgb(0 0 0 / 0.72)" }
```

Update `design-tokens.test.ts` expected string to match **after** regen (write the expectation first so check fails until regen).

In `paint-flights.ts` export:

```ts
export const FLIGHT_SHADOW_BLUR = 28;
export const FLIGHT_SHADOW_OFFSET_Y = 12;
export const FLIGHT_SHADOW_COLOR = "rgba(0,0,0,0.55)";
```

Use them in `paintFlightCard` (`shadowBlur`, `shadowOffsetY`, `shadowColor`). Add `paint-flights.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { FLIGHT_SHADOW_BLUR, FLIGHT_SHADOW_COLOR, FLIGHT_SHADOW_OFFSET_Y } from "./paint-flights";

describe("flight lift shadow", () => {
  it("locks Arena-forward lift constants", () => {
    expect(FLIGHT_SHADOW_BLUR).toBe(28);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(12);
    expect(FLIGHT_SHADOW_COLOR).toBe("rgba(0,0,0,0.55)");
  });
});
```

- [ ] **Step 2: Run token check / tests — expect FAIL**

```bash
cd client && bun run gen:tokens:check
bunx vitest run lib/design-tokens.test.ts app/board/bitmap/paint-flights.test.ts
```

Expected: FAIL stale tokens / old shadow values.

- [ ] **Step 3: Implement**

```bash
cd client && bun run gen:tokens
```

Apply flight constants in `paintFlightCard` (set `shadowOffsetY` before fill; reset blur/offset after).

Update specs:

`hand-and-zone-bar.md` Behavior — replace the drag-source fade bullet with:

```markdown
- The drag source fades with `opacity-25` and loses playable aura; the drag ghost carries the face, playable `barZoneAura`, and deepened `drop-shadow-drag`. Idle hits use `cursor-grab` when playable and `cursor-not-allowed` otherwise; an active drag sets `cursor-grabbing` on the document element. See [hand-drag-border-shadow-cursor design](2026-07-25-hand-drag-border-shadow-cursor-design.md).
```

`flights.md` Paint layer — add:

```markdown
- In-flight cards use a stronger lift shadow (`FLIGHT_SHADOW_*` in `paint-flights.ts`) matched to the hand-drag ghost drop shadow. See [hand-drag-border-shadow-cursor design](2026-07-25-hand-drag-border-shadow-cursor-design.md).
```

In `flights.md` Out of Scope, change or remove “Changing flight visual design beyond timing and pose parity” so it no longer forbids this shadow polish (e.g. “Changing flight visual design beyond lift-shadow polish and timing/pose parity”).

- [ ] **Step 4: Focused verify**

```bash
cd client && bun run gen:tokens:check
bunx vitest run \
  app/board/html/hand.test.ts \
  app/board/html/hand-drag-mount.test.ts \
  app/board/hand-drag.test.ts \
  app/board/bitmap/paint-flights.test.ts \
  lib/design-tokens.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write \
  app/board/html/hand.ts \
  app/board/html/hand-drag-mount.ts \
  app/board/bitmap/paint-flights.ts \
  app/board/bitmap/paint-flights.test.ts
```

Expected: PASS / clean.

- [ ] **Step 5: Commit + push**

```bash
git add design.tokens.json client/styles/tokens.generated.css \
  client/lib/design-tokens.generated.ts client/lib/design-tokens.test.ts \
  client/app/board/bitmap/paint-flights.ts client/app/board/bitmap/paint-flights.test.ts \
  docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md \
  docs/superpowers/specs/2026-07-20-flights.md
# include any other files gen:tokens writes
git commit -m "feat(client): deepen hand drag and flight lift shadows"
git push -u origin cursor/hand-drag-chrome-b23c
```

PR title: `feat(client): hand drag border, flight shadow, and cursors`

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| Source loses playable aura while dragging | Task 1 |
| Ghost gets `barZoneAura(zone, true)` | Task 1 |
| Idle not-allowed / grab cursors | Task 1 |
| Grabbing cursor while drag, cleared on end | Task 2 |
| Deeper `--drop-shadow-drag` | Task 3 |
| Stronger flight canvas shadow | Task 3 |
| Module spec updates | Task 3 |
| No flight playable rings / no wire | All (out of scope) |

## Placeholder scan

None. Shadow numbers are fixed in Global Constraints; tweak only if visual review demands (same PR).
