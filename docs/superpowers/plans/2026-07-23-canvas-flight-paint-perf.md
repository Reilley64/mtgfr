# Canvas Flight Paint Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop mid-flight animation frames from rebuilding the full Foldkit view and repainting the resting bitmap layer, so card flights feel smooth even on a sparse board.

**Architecture:** Flight Mount owns rAF: each frame runs pure `stepFlights` locally and paints only the flight canvas. `publishBitmapFrame` merges incoming model flights without stomping live poses, and skips resting-layer paint when resting inputs are unchanged. The board model receives a sync message only when the flying set changes (settle / clear), not every frame.

**Tech Stack:** Foldkit Mount + messages, Vitest, Canvas 2D bitmap layers, pure `motion/flights.ts`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-flights.md`
- Branch: `cursor/foldkit-migration-design-1ef0` (PR #74)
- No WebGL / OffscreenCanvas / Pixi
- No pointer-move throttling in this plan (flights-only pain)
- No density / hover-raise / tap-tween work
- Keep flight easing (τ≈75ms), reduced-motion snap, and `hideCardIds` product semantics
- TDD; Angular commit subjects; focused commits

## File map

| File | Role |
|------|------|
| `client/app/board/bitmap/flight-frame.ts` | Pure helpers: resting paint fingerprint equality + merge live flight poses with model publishes |
| `client/app/board/bitmap/flight-frame.test.ts` | Unit tests for those helpers |
| `client/app/board/bitmap/mount.ts` | Local rAF step/paint; merge on publish; resting paint gate; settle → message |
| `client/app/board/bitmap/mount.test.ts` | Resting paint not called on pose-only ticks; merge/settle behavior |
| `client/app/board/messages.ts` | `FlightsSynced` message (flying list after Mount step) |
| `client/app/board/submodel.ts` | Apply `FlightsSynced`; stop using `TickedFrame` as a per-frame stepper (or no-op mid-flight) |
| `client/app/update.ts` | Wire `FlightsSynced` if messages are re-exported / routed there |
| `docs/client-canvas-map.md` | Note flight-local rAF + resting dirty gate |
| Spec design file | Status → Done when plan complete |

---

### Task 1: Pure flight-frame helpers (TDD)

**Files:**
- Create: `client/app/board/bitmap/flight-frame.ts`
- Create: `client/app/board/bitmap/flight-frame.test.ts`

**Interfaces:**
- Consumes: `BitmapFrame` fields (or a narrow resting/flight slice), `CardFlight` from `../motion/flights`
- Produces:
  - `restingPaintChanged(prev: RestingPaintSnapshot | null, next: RestingPaintSnapshot): boolean`
  - `restingPaintSnapshot(frame: Pick<BitmapFrame, ...>): RestingPaintSnapshot`
  - `mergeFlightPoses(live: readonly CardFlight[], incoming: readonly CardFlight[]): CardFlight[]`

- [ ] **Step 1: Write the failing tests**

```ts
import { describe, expect, it } from "vitest";
import { spawnFlight } from "../motion/flights";
import { mergeFlightPoses, restingPaintChanged, restingPaintSnapshot } from "./flight-frame";

const baseResting = {
  width: 1440,
  height: 900,
  camera: { panX: 0, panY: 0, zoom: 1 },
  cards: [{ id: 1 }],
  viewer: 0,
  players: [],
  priority: 0,
  combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
  stagedAttackers: [],
  stagedBlocks: [],
  hideCardIds: new Set<number>(),
  targetObjects: new Set<number>(),
  targetPlayers: new Set<number>(),
  aimFrom: null,
  cursor: { x: 0, y: 0 },
  combatDragFrom: null,
  combatDragStroke: null,
  paymentPreviewIds: new Set<number>(),
  actions: undefined as undefined,
};

describe("restingPaintChanged", () => {
  it("is false when only flights would differ (snapshot omits flights)", () => {
    const a = restingPaintSnapshot({ ...baseResting, /* snapshot factory ignores flights */ } as never);
    const b = restingPaintSnapshot({ ...baseResting } as never);
    expect(restingPaintChanged(a, b)).toBe(false);
  });

  it("is true when hideCardIds or camera changes", () => {
    const a = restingPaintSnapshot({ ...baseResting, hideCardIds: new Set([1]) } as never);
    const b = restingPaintSnapshot({ ...baseResting, hideCardIds: new Set() } as never);
    expect(restingPaintChanged(a, b)).toBe(true);
  });
});

describe("mergeFlightPoses", () => {
  it("keeps live x/y/scale when id and targets match", () => {
    const incoming = [
      spawnFlight({
        id: 7,
        print: "p",
        name: "Bolt",
        x: 0,
        y: 0,
        scale: 1,
        targetX: 100,
        targetY: 200,
        targetScale: 1,
        kind: "battlefield",
      }),
    ];
    const live = [{ ...incoming[0], x: 40, y: 80, scale: 1, phase: "flying" as const }];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({ id: 7, x: 40, y: 80, targetX: 100, targetY: 200 });
  });

  it("adopts incoming when target retargets or id is new", () => {
    const live = [
      spawnFlight({
        id: 7,
        print: "p",
        name: "Bolt",
        x: 40,
        y: 80,
        scale: 1,
        targetX: 100,
        targetY: 200,
        targetScale: 1,
        kind: "battlefield",
      }),
    ];
    const incoming = [{ ...live[0], targetX: 300, targetY: 400, x: 0, y: 0 }];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({ targetX: 300, targetY: 400, x: 0, y: 0 });
  });
});
```

Adapt `restingPaintSnapshot` to take a real narrow type — include every `BitmapFrame` field that `paintBitmapLayer` depends on **except** `flights`. Compare with structural equality (camera fields, set membership for hide/target/payment, card id list + layout x/y/w/h/tapped/print, cursor only if aim/combat drag active, etc.). Prefer a simple serialized key string if clearer.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run app/board/bitmap/flight-frame.test.ts`

Expected: FAIL (module / exports missing)

- [ ] **Step 3: Implement helpers**

```ts
// client/app/board/bitmap/flight-frame.ts
import type { CardFlight } from "../motion/flights";
import type { BitmapFrame } from "./mount";

export type RestingPaintSnapshot = string; // or a struct — pick one and stick to it in tests

export function restingPaintSnapshot(frame: Omit<BitmapFrame, "flights">): RestingPaintSnapshot {
  // Deterministic key from resting paint inputs only
}

export function restingPaintChanged(prev: RestingPaintSnapshot | null, next: RestingPaintSnapshot): boolean {
  if (prev == null) return true;
  return prev !== next;
}

export function mergeFlightPoses(live: readonly CardFlight[], incoming: readonly CardFlight[]): CardFlight[] {
  const liveById = new Map(live.map((f) => [f.id, f]));
  return incoming.map((inc) => {
    const prev = liveById.get(inc.id);
    if (prev == null) return inc;
    const retargeted =
      prev.targetX !== inc.targetX || prev.targetY !== inc.targetY || prev.targetScale !== inc.targetScale;
    if (retargeted) return inc;
    // Keep live pose; refresh static fields from incoming (print/name/kind)
    return {
      ...inc,
      x: prev.x,
      y: prev.y,
      scale: prev.scale,
      phase: prev.phase,
    };
  });
}
```

If `BitmapFrame` is not exported from `mount.ts`, export it (or define the resting pick type in `flight-frame.ts` and cast at the call site).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/board/bitmap/flight-frame.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/bitmap/flight-frame.ts client/app/board/bitmap/flight-frame.test.ts client/app/board/bitmap/mount.ts
git commit -m "feat(client): pure helpers for flight pose merge and resting paint gate"
```

---

### Task 2: `FlightsSynced` message + submodel apply (no per-frame step)

**Files:**
- Modify: `client/app/board/messages.ts`
- Modify: `client/app/board/submodel.ts` (`tickedFrameModel` / new case)
- Modify: `client/app/update.ts` if Message union / routing needs the new tag
- Test: `client/app/board/bitmap/mount.test.ts` or a small `submodel` / scene test file that already exercises flights — add focused tests beside existing flight tests if present

**Interfaces:**
- Consumes: `CardFlight` shape (id, print, name, x, y, scale, targets, phase, kind, fromCardId?)
- Produces: `FlightsSynced({ flights: CardFlight[], now: number })` applied to `BoardModel`

- [ ] **Step 1: Write the failing test**

Find an existing board update test that uses `TickedFrame`, or add:

```ts
import { FlightsSynced } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";
import { spawnFlight } from "./motion/flights";

test("FlightsSynced replaces flights and clears hide when empty", () => {
  const flying = spawnFlight({
    id: 9,
    print: "p",
    name: "Shock",
    x: 10,
    y: 10,
    scale: 1,
    targetX: 10,
    targetY: 10,
    targetScale: 1,
    kind: "battlefield",
  });
  let model = {
    ...initialBoardModel(),
    flights: new Map([[9, { ...flying, phase: "flying" as const }]]),
    hideCardIds: new Set([9]),
    ownedIds: new Set([9]),
    lastFlightFrame: 100,
  };
  [model] = updateBoard(model, FlightsSynced({ flights: [], now: 200 }), /* fold */, "table-1");
  expect(model.flights.size).toBe(0);
  expect(model.hideCardIds.size).toBe(0);
  expect(model.ownedIds.size).toBe(0);
  expect(model.lastFlightFrame).toBeNull();
});
```

Use the real `gameFold` helper from nearby tests. Also assert: applying `FlightsSynced` with one still-flying card keeps `hideCardIds` containing that id and stores the provided pose.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/board/<test-file>.ts -t "FlightsSynced"`

Expected: FAIL (`FlightsSynced` missing)

- [ ] **Step 3: Add message + handler**

In `messages.ts`:

```ts
export const FlightsSynced = m("FlightsSynced", {
  now: S.Number,
  // Use Schema for the flight fields the Mount sends — mirror CardFlight
  flights: S.Array(
    S.Struct({
      id: S.Number,
      print: S.String,
      name: S.String,
      x: S.Number,
      y: S.Number,
      scale: S.Number,
      targetX: S.Number,
      targetY: S.Number,
      targetScale: S.Number,
      phase: S.Literals(["flying", "settled"]),
      kind: S.Literals(["battlefield", "stack", "from-stack"]),
      fromCardId: S.optional(S.Number),
    }),
  ),
});
```

Register in the board `Message` union.

In `submodel.ts`, implement apply (reuse the settle bookkeeping from `tickedFrameModel`):

```ts
function applyFlightsSynced(model: BoardModel, flightsIn: readonly CardFlight[], now: number): BoardModel {
  const flights = new Map<number, CardFlight>();
  const handHidden = new Set(model.handHidden);
  for (const flight of flightsIn) {
    if (flight.phase === "settled") {
      if (flight.fromCardId != null) handHidden.delete(flight.fromCardId);
      continue;
    }
    flights.set(flight.id, flight);
  }
  // Also drop handHidden for ids that disappeared vs previous flying set
  return {
    ...model,
    flights,
    handHidden,
    hideCardIds: flyingCardIds(flights),
    lastFlightFrame: flights.size === 0 ? null : now,
    ownedIds: new Set(flights.keys()),
  };
}
```

Change `TickedFrame` handling: either remove stepping (Mount no longer sends it every frame) or make it a no-op when `flights.size > 0` so accidental ticks cannot thrash. Prefer: **keep `TickedFrame` for cleanup when size===0** if still useful, but **do not call `stepFlights` on every tick**. Primary path is `FlightsSynced`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/board/<test-file>.ts -t "FlightsSynced"`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/messages.ts client/app/board/submodel.ts client/app/update.ts client/app/board/**/*.test.ts
git commit -m "feat(client): sync flight model on settle via FlightsSynced"
```

---

### Task 3: Mount-local rAF + resting paint gate

**Files:**
- Modify: `client/app/board/bitmap/mount.ts`
- Modify: `client/app/board/bitmap/mount.test.ts`
- Modify: `client/app/board/view.ts` only if `publishBitmapFrame` call site needs a note (usually unchanged)

**Interfaces:**
- Consumes: `stepFlights`, `mergeFlightPoses`, `restingPaintSnapshot`, `restingPaintChanged`, `FlightsSynced`
- Produces: Flight layer paints every rAF; resting layer paints only when resting snapshot changes; `FlightsSynced` when flying membership / all-settled changes

- [ ] **Step 1: Write the failing Mount-level test**

Extend `mount.test.ts` with a testable seam. Export a small driver used by Mount (preferred over poking rAF):

```ts
// In mount.ts (exported for tests)
export type FlightClockState = {
  liveFlights: CardFlight[];
  lastRestingSnapshot: ReturnType<typeof restingPaintSnapshot> | null;
};

export function applyPublishedFrame(
  state: FlightClockState,
  frame: BitmapFrame,
): { state: FlightClockState; paintResting: boolean; paintFlight: boolean; frame: BitmapFrame } {
  // merge flights into frame; decide paintResting via restingPaintChanged
}

export function tickFlightClock(
  state: FlightClockState,
  frame: BitmapFrame,
  now: number,
  dtMs: number,
  reducedMotion: boolean,
): {
  state: FlightClockState;
  frame: BitmapFrame;
  paintFlight: boolean;
  sync: { flights: CardFlight[]; now: number } | null;
} {
  // stepFlights on live map; paintFlight true; sync non-null when flying set changes or all settled
}
```

Test:

```ts
it("pose-only flight tick does not request resting paint", () => {
  const flight = spawnFlight({
    id: 3,
    print: "p",
    name: "Bolt",
    x: 0,
    y: 0,
    scale: 1,
    targetX: 100,
    targetY: 0,
    targetScale: 1,
    kind: "battlefield",
  });
  const frame = { /* minimal BitmapFrame with flights: [flight], cards: [card()] */ };
  let state = { liveFlights: [flight], lastRestingSnapshot: null };
  const published = applyPublishedFrame(state, frame);
  expect(published.paintResting).toBe(true);
  state = published.state;

  const tick = tickFlightClock(state, published.frame, 16, 16, false);
  expect(tick.paintFlight).toBe(true);
  expect(tick.sync).toBeNull(); // still flying, membership unchanged

  const republish = applyPublishedFrame(tick.state, { ...frame, flights: frame.flights });
  expect(republish.paintResting).toBe(false);
  expect(republish.frame.flights[0]?.x).not.toBe(0); // live pose preserved
});
```

Fill in a real minimal `BitmapFrame` using the same helpers as existing mount tests.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/board/bitmap/mount.test.ts -t "pose-only flight tick"`

Expected: FAIL (helpers missing)

- [ ] **Step 3: Wire Mount to the clock helpers**

Refactor `publishBitmapFrame` / `registerLayer` roughly as:

1. Keep `currentFrame` as the paint source.
2. On publish: `applyPublishedFrame` → update `currentFrame`; paint resting only if `paintResting`; paint flight if flight list structural change or first publish.
3. Flight layer `frame()` rAF callback:
   - `tickFlightClock(...)` with `now` / dt from `lastFlightTick`
   - `paintFlightLayer` only (not resting)
   - if `sync != null`, `Queue.offerUnsafe(queue, FlightsSynced(sync))`
   - `kickRaf` while live flights remain in `flying` phase
4. Stop offering `TickedFrame` every rAF (switch Mount stream messages to `ArtLoaded` + `FlightsSynced`).

`MountFlightLayer` / `defineLayerMount` signature must list `FlightsSynced` instead of (or in addition to) `TickedFrame`.

- [ ] **Step 4: Run Mount + flight-frame + FlightsSynced tests**

Run:

```bash
cd client && bunx vitest run app/board/bitmap/flight-frame.test.ts app/board/bitmap/mount.test.ts
```

Also run any board test that previously depended on `TickedFrame` stepping:

```bash
cd client && bunx vitest run app/board --grep Flight
```

Expected: PASS (fix any broken TickedFrame tests to use `FlightsSynced` or a single settle tick)

- [ ] **Step 5: Manual sanity checklist (document in commit body)**

- Spawn one flight: resting layer does not clear every frame (DevTools Performance or a temporary paint counter in tests is enough for agents).
- Settle: resting card reappears; no double draw during flight.

- [ ] **Step 6: Commit**

```bash
git add client/app/board/bitmap/mount.ts client/app/board/bitmap/mount.test.ts client/app/board/messages.ts client/app/board/submodel.ts
git commit -m "perf(client): step and paint flights without full-board frames"
```

---

### Task 4: Docs + spec status

**Files:**
- Modify: `docs/client-canvas-map.md` (flight layer / rAF note)
- Modify: `docs/superpowers/specs/2026-07-20-flights.md` (current paint-gating behavior)

- [ ] **Step 1: Update canvas map**

Add a short note under the flight / bitmap layer section:

```markdown
Flight animation is Mount-local rAF: mid-flight ticks paint only the flight
canvas. Resting bitmap republishes when layout/chrome/hide sets change, not on
every pose tick. Model receives `FlightsSynced` when the flying set changes.
```

- [ ] **Step 2: Mark design Done**

Set spec status to `Done` and one-line evidence (tests that prove resting paint gated + FlightsSynced settle).

- [ ] **Step 3: Commit**

```bash
git add docs/client-canvas-map.md docs/superpowers/specs/2026-07-20-flights.md
git commit -m "docs: record flight-local canvas paint loop"
```

- [ ] **Step 4: Verification**

```bash
cd client && bunx vitest run app/board/bitmap/flight-frame.test.ts app/board/bitmap/mount.test.ts
```

Expected: PASS

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Flight Mount owns rAF; flight layer only per frame | Task 3 |
| Mid-flight must not full updateBoard → view → dual publish | Tasks 2–3 |
| Resting bitmap gated on non-flight inputs | Tasks 1, 3 |
| Vector view not rebuilt on flight ticks | Task 3 (no TickedFrame storm) |
| Spawn/retarget from model; Mount merges poses | Tasks 1, 3 |
| hideCardIds / settle semantics | Task 2 |
| Reduced motion parity via `stepFlights` | Task 3 |
| Tests: resting paint count / pose ticks | Tasks 1, 3 |
| Out of scope items not implemented | All tasks |

## Placeholder / consistency self-review

- No TBD steps; helpers named `mergeFlightPoses`, `restingPaintSnapshot`, `restingPaintChanged`, `applyPublishedFrame`, `tickFlightClock`, message `FlightsSynced`.
- `TickedFrame` is demoted; do not leave a second per-frame stepper alive in Mount.
