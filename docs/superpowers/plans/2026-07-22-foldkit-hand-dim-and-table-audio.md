# Foldkit Hand Dim Retirement + Table Audio Unlock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire unplayable hand/command brightness veils and make synthesized table audio audible after lobby Ready without requiring the Sound toggle.

**Architecture:** Hand tiles stop using `brightness-[0.55]` for “no legal action”; castability stays on Arena playable borders. `dimmed`/`slotInert` only gates interaction for staged/in-flight slots. Audio keeps the existing six synth cues; `unlockTableAudio()` runs synchronously on Ready click and on Sound-on (recovery + unmute confirmation tick).

**Tech Stack:** Foldkit HTML views, Vitest, Web Audio `AudioContext`, Effect Match lobby update, Bun.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`, `docs/superpowers/specs/2026-07-20-table-audio.md`, `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md`
- Branch: `cursor/foldkit-migration-design-1ef0` (PR #74)
- No sample/Howler assets; no board-wide pointerdown unlock; no reintroducing unplayable hand darkening
- Happy-path audio must work without pressing Sound
- TDD: red → green → commit per task; Angular commit subjects
- Interaction / UI PR — run verify Interaction checklist items for hand + sound before claiming done

## File map

| File | Role |
|---|---|
| `client/app/board/html/hand.ts` | Drop unplayable brightness; split inert vs unplayable |
| `client/app/board/html/hand.test.ts` | Assert no `brightness-[0.55]` on unplayable tiles; keep playable borders; drag fade |
| `client/lib/tableAudio.ts` | Add `playUnmuteTick()`; keep unlock/`canPlay` |
| `client/lib/tableAudio.test.ts` | Unlock resume + unmute tick + mute gating |
| `client/app/board/submodel.ts` | `SoundToggled` → unlock + unmute tick when turning on |
| `client/app/board/sound.test.ts` | Board update spies unlock/tick on Sound-on |
| `client/app/shell/lobby/update.ts` | Sync `unlockTableAudio()` in `RequestedLobbyReady` |
| `client/app/shell/lobby/update.test.ts` | Ready update calls unlock before enqueueing `ReadyLobby` |
| Spec design file | Status → Done at end |

---

### Task 1: Retire unplayable hand/command brightness veil

**Files:**
- Modify: `client/app/board/html/hand.ts`
- Modify: `client/app/board/html/hand.test.ts`

**Interfaces:**
- Consumes: `handView(...)`, existing `barZoneAura`, `slotDimmed` staging/flight helpers
- Produces: Unplayable hand/command tiles at full brightness; `playable` still false when no action or slot is staged/in-flight; drag-source `opacity-25` unchanged

- [ ] **Step 1: Write the failing tests**

Append to `client/app/board/html/hand.test.ts`:

```ts
function treeHasClass(node: unknown, token: string): boolean {
  if (className(node).split(/\s+/).includes(token)) return true;
  if (node == null || typeof node !== "object") return false;
  const n = node as { children?: unknown[] };
  return (n.children ?? []).some((child) => treeHasClass(child, token));
}

describe("handView unplayable brightness", () => {
  it("does not darken unplayable hand tiles (borders carry castability)", () => {
    const castable = object(42, { name: "Lightning Bolt" });
    const uncastable = object(43, { name: "Cancel" });
    const tree = renderHand(state({ objects: [castable, uncastable], actions: [action(7, { object: 42 })] }));

    const unplayableFace = findTestId(tree, "hand-card-face-43");
    expect(unplayableFace).not.toBeNull();
    expect(treeHasClass(unplayableFace, "brightness-[0.55]")).toBe(false);
    expect(className(unplayableFace)).not.toContain("ring-playable-border");
  });

  it("does not darken unplayable command tiles", () => {
    const commander = object(9, {
      name: "Atraxa",
      zone: ZONE.Command,
      is_commander: true,
      kind: { kind: "creature" },
    });
    const tree = renderHand(state({ objects: [commander], actions: [] }));
    const face = findTestId(tree, "hand-card-face-9");
    expect(face).not.toBeNull();
    expect(treeHasClass(face, "brightness-[0.55]")).toBe(false);
  });

  it("still fades the drag-source hand tile", () => {
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
        x: 10,
        y: 10,
      },
    });
    const face = findTestId(tree, "hand-card-face-42");
    expect(face).not.toBeNull();
    expect(treeHasClass(face, "opacity-25")).toBe(true);
  });
});
```

`HandDragState` fields: `action`, `name`, `print`, `manaCost`, optional `kind`, `x`, `y` (`client/app/board/submodel.ts`).

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd client && bun run test app/board/html/hand.test.ts
```

Expected: FAIL — unplayable faces still include `brightness-[0.55]`.

- [ ] **Step 3: Minimal implementation in `hand.ts`**

1. Rename the tile arg conceptually from visual-dim to inert gating. Prefer renaming `dimmed` → `slotInert` on `tile` / `HandSlot` (or keep the name but stop using it for CSS brightness).
2. Set slot inert **only** from staging/flight:

```ts
const slotInert = (id: number) => id === hiddenId || flyingIds.has(id);

// command:
slotInert: slotInert(c.id),
// hand:
slotInert: slotInert(c.id),
// extras / GY / exile: slotInert: false
```

3. Playable rule:

```ts
const playable = action != null && !slotInert;
```

4. Remove brightness from art classes and the no-print fallback:

```ts
const artClass = [
  "pointer-events-none block touch-none rounded-game object-cover shadow-hand transition-[filter,opacity] duration-[80ms] ease-state",
  dragSource ? "opacity-25" : "",
  playable && !dragSource ? "group-hover/hand-tile:brightness-110" : "",
]
  .filter((v) => v !== "")
  .join(" ");

// fallback face (no print):
h.Class(
  "flex items-center justify-center rounded-game bg-forest-shadow p-1 text-center text-caption text-snow shadow-hand",
),
```

5. Delete any remaining `dimmed ? "brightness-[0.55]" : ""` branches in this file.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd client && bun run test app/board/html/hand.test.ts
```

Expected: PASS (including existing playable-outline tests).

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/hand.ts client/app/board/html/hand.test.ts
git commit -m "fix(client): stop dimming unplayable hand and command tiles"
```

---

### Task 2: Unmute confirmation tick + AudioContext unlock tests

**Files:**
- Modify: `client/lib/tableAudio.ts`
- Create: `client/lib/tableAudio.test.ts`

**Interfaces:**
- Consumes: existing `unlockTableAudio`, `canPlay`, `tone`, `setSoundEnabledForTests`, `resetTableAudioForTests`
- Produces: `playUnmuteTick(): void` — short soft confirmation when sound is on and context is running

- [ ] **Step 1: Write the failing tests**

Create `client/lib/tableAudio.test.ts`:

```ts
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  audioContextForTests,
  playUnmuteTick,
  resetTableAudioForTests,
  setSoundEnabledForTests,
  unlockTableAudio,
} from "./tableAudio";

class FakeAudioContext {
  state: AudioContextState = "suspended";
  currentTime = 0;
  resume = vi.fn(async () => {
    this.state = "running";
  });
  createOscillator() {
    return {
      type: "sine",
      frequency: { value: 0 },
      connect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
    };
  }
  createGain() {
    return {
      gain: {
        setValueAtTime: vi.fn(),
        linearRampToValueAtTime: vi.fn(),
        exponentialRampToValueAtTime: vi.fn(),
      },
      connect: vi.fn(),
    };
  }
}

describe("tableAudio unlock", () => {
  beforeEach(() => {
    resetTableAudioForTests();
    setSoundEnabledForTests(true);
    vi.stubGlobal("AudioContext", FakeAudioContext);
  });
  afterEach(() => {
    resetTableAudioForTests();
    setSoundEnabledForTests(null);
    vi.unstubAllGlobals();
  });

  it("resume()s a suspended context on unlock", () => {
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    expect(ac).not.toBeNull();
    expect(ac.resume).toHaveBeenCalled();
    expect(ac.state).toBe("running");
  });

  it("playUnmuteTick no-ops when muted", () => {
    setSoundEnabledForTests(false);
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    const createOscillator = vi.spyOn(ac, "createOscillator");
    playUnmuteTick();
    expect(createOscillator).not.toHaveBeenCalled();
  });

  it("playUnmuteTick plays when unlocked and enabled", () => {
    unlockTableAudio();
    const ac = audioContextForTests() as unknown as FakeAudioContext;
    // Fake resume flips state synchronously inside the mock; if canPlay still
    // sees "suspended", set `ac.state = "running"` before the tick.
    ac.state = "running";
    const createOscillator = vi.spyOn(ac, "createOscillator");
    playUnmuteTick();
    expect(createOscillator).toHaveBeenCalled();
  });
});
```

Also import `audioContextForTests` once it exists. Note: `resume` is async on real browsers; the fake must set `state = "running"` inside the `resume` mock body so sync tests can proceed.

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd client && bun run test lib/tableAudio.test.ts
```

Expected: FAIL — `playUnmuteTick` / `audioContextForTests` missing.

- [ ] **Step 3: Minimal implementation**

In `client/lib/tableAudio.ts`:

```ts
/** Test-only: current shared context, if any. */
export function audioContextForTests(): AudioContext | null {
  return sharedCtx;
}

/** Short confirmation when the player unmutes (Sound toggle on). */
export function playUnmuteTick(): void {
  const ac = canPlay();
  if (!ac) return;
  const t = ac.currentTime;
  tone(ac, 660, t, 0.05, 0.04, "sine");
}
```

Ensure `unlockTableAudio` still creates + `resume()`s; with the fake, sync `resume` mock that sets `state = "running"` so `canPlay()` works immediately in tests. If the real `resume` is async, the Fake’s sync state flip in the mock body is enough for unit tests.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd client && bun run test lib/tableAudio.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add client/lib/tableAudio.ts client/lib/tableAudio.test.ts
git commit -m "feat(client): add unmute tick and tableAudio unlock tests"
```

---

### Task 3: Sound toggle recovers AudioContext

**Files:**
- Modify: `client/app/board/submodel.ts` (`SoundToggled` case)
- Create: `client/app/board/sound.test.ts`

**Interfaces:**
- Consumes: `SoundToggled`, `setSoundEnabled`, `unlockTableAudio`, `playUnmuteTick`, `updateBoard`, `initialBoardModel`
- Produces: Turning sound on unlocks + confirmation tick; turning off only mutes

- [ ] **Step 1: Write the failing test**

Create `client/app/board/sound.test.ts` (inline the same `fold(objects, actions)` helper shape used in `hand-drag.test.ts`):

```ts
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ActionView, ObjectView } from "~/wire/types";
import * as tableAudio from "../../lib/tableAudio";
import type { GameFoldState } from "../game/fold";
import { SoundToggled } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function fold(objects: ObjectView[] = [], actions: ActionView[] = []): GameFoldState {
  return {
    seq: 1,
    state: {
      active_player: 0,
      can_act: true,
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
      objects,
      pending_choice: null,
      players: [
        {
          commander_tax: 0,
          hand_count: 0,
          library_count: 80,
          life: 40,
          lost: false,
          mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
          player: 0,
          username: "Alice",
        },
      ],
      priority: 0,
      stack: [],
      step: 3,
      viewer: 0,
      actions,
    },
    log: [],
    reject: null,
    provenance: {
      zoneMoves: new Map(),
      resolvedFromStack: new Set(),
      leftStackToPile: new Set(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false },
  };
}

describe("SoundToggled", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    tableAudio.resetTableAudioForTests();
    tableAudio.setSoundEnabledForTests(null);
  });

  it("unlocks and plays unmute tick when turning sound on", () => {
    tableAudio.setSoundEnabledForTests(false);
    const board = { ...initialBoardModel(), soundOn: false };
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const tick = vi.spyOn(tableAudio, "playUnmuteTick");

    const [next] = updateBoard(board, SoundToggled(), fold(), "T1");
    expect(next.soundOn).toBe(true);
    expect(unlock).toHaveBeenCalledTimes(1);
    expect(tick).toHaveBeenCalledTimes(1);
  });

  it("does not unlock when turning sound off", () => {
    tableAudio.setSoundEnabledForTests(true);
    const board = { ...initialBoardModel(), soundOn: true };
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const tick = vi.spyOn(tableAudio, "playUnmuteTick");

    const [next] = updateBoard(board, SoundToggled(), fold(), "T1");
    expect(next.soundOn).toBe(false);
    expect(unlock).not.toHaveBeenCalled();
    expect(tick).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd client && bun run test app/board/sound.test.ts
```

Expected: FAIL — unlock/tick not called from `SoundToggled`.

- [ ] **Step 3: Minimal implementation**

In `client/app/board/submodel.ts`:

```ts
import { isSoundEnabled, playUnmuteTick, setSoundEnabled, unlockTableAudio } from "../../lib/tableAudio";
```

Replace the `SoundToggled` case:

```ts
case "SoundToggled": {
  const next = !model.soundOn;
  setSoundEnabled(next);
  if (next) {
    unlockTableAudio();
    playUnmuteTick();
  }
  return [{ ...model, soundOn: next }, []];
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd client && bun run test app/board/sound.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add client/app/board/submodel.ts client/app/board/sound.test.ts
git commit -m "fix(client): unlock table audio when unmuting"
```

---

### Task 4: Gesture-safe unlock on lobby Ready

**Files:**
- Modify: `client/app/shell/lobby/update.ts`
- Create: `client/app/shell/lobby/update.test.ts`

**Interfaces:**
- Consumes: `RequestedLobbyReady`, `update(model, message, deckIds)`, `unlockTableAudio`
- Produces: Ready click path calls `unlockTableAudio()` synchronously before returning the `ReadyLobby` command

- [ ] **Step 1: Write the failing test**

Create `client/app/shell/lobby/update.test.ts`:

```ts
import { afterEach, describe, expect, it, vi } from "vitest";
import * as tableAudio from "../../../lib/tableAudio";
import { RequestedLobbyReady } from "./messages";
import { initialLobbySlice } from "./submodel";
import { ReadyLobby, update } from "./update";

describe("RequestedLobbyReady audio unlock", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    tableAudio.resetTableAudioForTests();
  });

  it("unlocks table audio synchronously on Ready", () => {
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const model = { ...initialLobbySlice(), tableId: "ABC123" };
    const [next, commands] = update(model, RequestedLobbyReady({ ready: true }), [1]);

    expect(unlock).toHaveBeenCalledTimes(1);
    expect(next.submitting).toBe(true);
    expect(commands).toHaveLength(1);
    expect(commands[0]?.name).toBe(ReadyLobby.name);
  });

  it("does not unlock when tableId is missing", () => {
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    update(initialLobbySlice(), RequestedLobbyReady({ ready: true }), [1]);
    expect(unlock).not.toHaveBeenCalled();
  });
});
```

`commands[0]?.name` matches existing board tests (`scene.test.ts` uses `SubmitIntent.name`).

- [ ] **Step 2: Run test to verify it fails**

```bash
cd client && bun run test app/shell/lobby/update.test.ts
```

Expected: FAIL — unlock only happens inside the Effect command today, not in the sync update handler.

- [ ] **Step 3: Minimal implementation**

In `client/app/shell/lobby/update.ts`, change `RequestedLobbyReady`:

```ts
RequestedLobbyReady: ({ ready }) => {
  if (model.tableId == null) return [model, []];
  unlockTableAudio();
  return [{ ...model, error: null, submitting: true }, [ReadyLobby({ tableId: model.tableId, ready })]];
},
```

Keep `unlockTableAudio()` inside `ReadyLobby` as defense in depth.

- [ ] **Step 4: Run test to verify it passes**

```bash
cd client && bun run test app/shell/lobby/update.test.ts
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/update.ts client/app/shell/lobby/update.test.ts
git commit -m "fix(client): unlock table audio on Ready click path"
```

---

### Task 5: Spec Done + focused verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`, `docs/superpowers/specs/2026-07-20-table-audio.md`, `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` (current behavior notes)
- Optional note in `DESIGN.md` only if hand dim language still claims a veil — align to “borders only”

**Interfaces:**
- Consumes: Tasks 1–4 green
- Produces: Spec marked Done; test suite evidence

- [ ] **Step 1: Run the targeted client tests**

```bash
cd client && bun run test app/board/html/hand.test.ts lib/tableAudio.test.ts app/board/sound.test.ts app/shell/lobby/update.test.ts
```

Expected: all PASS.

- [ ] **Step 2: Mark spec Done**

In the design doc header:

```markdown
**Status:** Done
**Plan:** [`docs/superpowers/plans/2026-07-22-foldkit-hand-dim-and-table-audio.md`](../plans/2026-07-22-foldkit-hand-dim-and-table-audio.md)
```

- [ ] **Step 3: Manual Interaction checks (when `just dev` is available)**

- Ready → start → land drop / gain priority: hear cues **without** pressing Sound.
- Unplayable hand cards: full brightness; playable cards keep playable border.
- Mute / unmute: unmute plays confirmation tick.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md docs/superpowers/specs/2026-07-20-table-audio.md docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md DESIGN.md
git commit -m "docs: mark hand-dim and table-audio unlock spec done"
```

---

## Spec coverage checklist

| Spec requirement | Task |
|---|---|
| No unplayable hand/command brightness veil | Task 1 |
| Keep playable borders + drag/staging fades | Task 1 |
| Split inert vs unplayable | Task 1 |
| Keep six synth cues + MountBoardAudio / feel flags | (unchanged; Tasks 2–4 only unlock) |
| Sync Ready unlock (happy path) | Task 4 |
| Sound-on unlock + confirmation tick (recovery) | Tasks 2–3 |
| Mute-off silent | Task 3 |
| Automated tests listed in spec | Tasks 1–4 |
| Manual Ready-without-Sound | Task 5 |
| Out of scope battlefield dim API cleanup | skipped (explicit) |
