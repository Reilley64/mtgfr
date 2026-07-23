# Foldkit Remaining Bugs + Board Layers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close remaining Foldkit product bugs on PR #74 — Alt/Option dock inspect, Lobby Bring + Back, hand drag sensitivity, under-card names, declare-arrow / flight stacking — and lock the board layer stack in spec + code.

**Architecture:** Live-triage first (Alt/Option known-broken). Extend shared `cardHoverPreviewView` with `mode: "follow" | "dock"` for board inspect. Reorder board composition and z-index to match the locked layer table in `docs/client-canvas-map.md`. Outcome Scene/unit tests for each user-visible fix; Interaction checklist before claiming done.

**Tech Stack:** Foldkit (Html, Mount, Canvas, Scene tests), Vitest, Effect Schema messages, Tailwind z-index utilities, Markdown specs.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-board-composition.md`, `docs/superpowers/specs/2026-07-20-battlefield.md`, `docs/superpowers/specs/2026-07-20-card-inspect.md`
- Branch: `cursor/foldkit-migration-design-1ef0` (PR #74)
- Outcome tests in product language (no new “Solid parity” test titles)
- Inspect is topmost while pinned (including above concede/result/portrait chrome)
- Two mana surfaces: battlefield in-play mana (under permanents) vs spell/payment mana on the hand layer
- Angular commit subjects; focused commits (`fix:`, `feat:`, `docs:`, `test:`)
- No Playwright CI matrix; no prompt-stub / CardArt ImageCache debt in this plan

## File map

| File | Role |
|---|---|
| `docs/client-canvas-map.md` | Authoritative layer stack (replace stale paint-order invariant) |
| `docs/superpowers/specs/2026-07-20-board-composition.md` | Cross-link layer SoT |
| `client/lib/deck-builder/card-hover-preview.ts` | Add `mode: "follow" \| "dock"`; shared art + oracle + mods |
| `client/lib/deck-builder/card-hover-preview.test.ts` | Create — follow vs dock layout assertions |
| `client/app/board/html/inspect.ts` | Thin wrapper → dock mode; modifier ledger as dock extras |
| `client/app/board/html/keyboard-mount.ts` | AltLeft/AltRight (+ Option) key detection |
| `client/app/board/html/overlays.ts` | Layer order / z-index per stack; inspect last |
| `client/app/board/view.ts` | Composition: felt → bitmap permanents → overlays mid → flight layer → inspect |
| `client/app/board/bitmap/mount.ts` | Split or gate flight paint onto above-hand layer |
| `client/app/board/canvas/scene.ts` | Remove resting-card name `Canvas.Text` |
| `client/app/board/html/hand.ts` | Export `HAND_PLAY_SLACK_PX`; hand/spell-mana z on hand layer |
| `client/app/board/submodel.ts` | Play threshold uses slack |
| `client/app/board/hand-drag.test.ts` / `scene.test.ts` | Threshold outcome tests |
| `client/app/shell/lobby/view.ts` | Pre-pick Bring + Back; no select |
| `client/app/shell/lobby/entry.test.ts` | Bring/Back Scene tests |
| `client/app/board/geometry/layout.ts` | Mat/packing fixes as needed |
| Current board specs | Record landed layer, inspect, and drag behavior |

---

### Task 1: Lock layer stack in docs

**Files:**
- Modify: `docs/client-canvas-map.md`
- Modify: `docs/superpowers/specs/2026-07-20-board-composition.md`, `docs/superpowers/specs/2026-07-20-card-inspect.md`, and `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md` (short cross-link + inspect/drag notes)

**Interfaces:**
- Consumes: layer table from remaining-bugs design spec
- Produces: authoritative stack text implementers must match in later tasks

- [ ] **Step 1: Replace paint-order invariant with the locked stack**

In `docs/client-canvas-map.md`, replace invariant §2 (“Paint order: felt → seats → …”) with the full bottom→top table and rules from the design spec (layers 1–10, avatar paint vs hits, two mana surfaces, flights above hand, inspect topmost). Keep the rest of the map; do not delete module tables.

- [ ] **Step 2: Cross-link from the board feature spec**

Near the dual-surface intro in `docs/superpowers/specs/2026-07-20-board-composition.md`, ensure the canvas-map pointer says the **layer stack** there is authoritative. Update `docs/superpowers/specs/2026-07-20-card-inspect.md` to say board inspect uses shared preview **dock** mode (left art, right oracle/effects, backdrop, topmost). Update `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md` to note play threshold behavior.

- [ ] **Step 3: Commit**

```bash
git add docs/client-canvas-map.md docs/superpowers/specs/2026-07-20-board-composition.md docs/superpowers/specs/2026-07-20-card-inspect.md docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md
git commit -m "docs: lock board layer stack in canvas map"
```

---

### Task 2: Fix Alt/Option key detection (triage start)

**Files:**
- Modify: `client/app/board/html/keyboard-mount.ts`
- Test: `client/app/board/html/keyboard-mount.test.ts` (create if missing; else extend)

**Interfaces:**
- Consumes: `MountBoardKeyboard` → `AltDown` / `AltUp`
- Produces: AltLeft/AltRight (and `e.key === "Alt"`) both emit AltDown/AltUp

- [ ] **Step 1: Write failing tests for code-based Alt**

```ts
import { describe, expect, it } from "vitest";
import { isAltKeyEvent } from "./keyboard-mount"; // export the pure helper

describe("isAltKeyEvent", () => {
  it("matches AltLeft and AltRight codes", () => {
    expect(isAltKeyEvent({ key: "Alt", code: "AltLeft" } as KeyboardEvent)).toBe(true);
    expect(isAltKeyEvent({ key: "Alt", code: "AltRight" } as KeyboardEvent)).toBe(true);
  });

  it("matches key Alt even when code is empty", () => {
    expect(isAltKeyEvent({ key: "Alt", code: "" } as KeyboardEvent)).toBe(true);
  });

  it("ignores unrelated keys", () => {
    expect(isAltKeyEvent({ key: "a", code: "KeyA" } as KeyboardEvent)).toBe(false);
  });
});
```

- [ ] **Step 2: Run — expect FAIL (helper missing)**

```bash
cd client && bunx vitest run app/board/html/keyboard-mount.test.ts
```

Expected: FAIL — `isAltKeyEvent` not exported / file missing.

- [ ] **Step 3: Implement helper and wire mount**

```ts
export function isAltKeyEvent(e: KeyboardEvent): boolean {
  if (e.code === "AltLeft" || e.code === "AltRight") return true;
  return e.key === "Alt";
}
```

In `onKeyDown` / `onKeyUp`, replace `e.key === "Alt"` with `isAltKeyEvent(e)`.

- [ ] **Step 4: Run — expect PASS**

```bash
cd client && bunx vitest run app/board/html/keyboard-mount.test.ts app/board/inspect-pile-concede.test.ts
```

- [ ] **Step 5: Live smoke (manual in this task)**

With `just dev` (or running client), open a table, hover a face-up permanent, hold Option/Alt — note whether dock appears. Record result in the commit message body or task report (`works` / `still broken — dock wiring next`). Do not skip Task 3–4 even if key detection alone fixes pin state.

- [ ] **Step 6: Commit**

```bash
git add client/app/board/html/keyboard-mount.ts client/app/board/html/keyboard-mount.test.ts
git commit -m "fix(client): detect AltLeft/AltRight for inspect hold"
```

---

### Task 3: Shared card preview `dock` mode

**Files:**
- Modify: `client/lib/deck-builder/card-hover-preview.ts`
- Create: `client/lib/deck-builder/card-hover-preview.test.ts`
- Modify: builder/list call sites only if the API gains a required `mode` (default `"follow"`)

**Interfaces:**
- Consumes: existing `cardHoverPreviewView`, `HoverPreviewCard`, `cardArt`, `splitOracleText`
- Produces:

```ts
export type CardPreviewMode = "follow" | "dock";

export type DockPreviewArgs<M> = {
  mode: "dock";
  print: string;
  name: string;
  oracle?: string | null;
  approximates?: string | null;
  face?: "front" | "back";
  /** Extra right-column nodes (modifier ledger Html). */
  extras?: ReadonlyArray<Html>;
  onDismiss?: M; // optional; board wires backdrop click via wrapper
  testId?: string;
};

// follow keeps today's hover: { mode?: "follow"; hover; card; testId? }
```

Prefer a single function with a discriminated `mode` field.

- [ ] **Step 1: Failing tests for dock layout**

```ts
import { html } from "foldkit/html";
import { Scene } from "foldkit/scene"; // or project's Scene import used elsewhere
import { describe, expect, it } from "vitest";
import { cardHoverPreviewView } from "./card-hover-preview";

const h = html<never>();

it("dock mode renders backdrop and left-docked preview", () => {
  const node = cardHoverPreviewView(h, {
    mode: "dock",
    print: "abc",
    name: "Sol Ring",
    oracle: "{T}: Add {C}.",
    testId: "inspect-overlay",
  });
  // Assert via Scene.render or string/html probe used in this repo for pure Html —
  // match patterns in client/lib/ui/card-art.test.ts / builder story tests:
  // expect testid inspect-overlay, backdrop class bg-black/55, flex row with art + oracle text.
});
```

Use the same Scene/`render` helper other lib tests use. Assert:

- `data-testid="inspect-overlay"` (or args.testId)
- backdrop / `bg-black/55` (or equivalent)
- oracle text present
- not using `fixed top-(--y) left-(--x)` cursor positioning for dock

- [ ] **Step 2: Run — expect FAIL**

```bash
cd client && bunx vitest run lib/deck-builder/card-hover-preview.test.ts
```

- [ ] **Step 3: Implement `mode: "dock"`**

- Extract shared `textPanel` / art column.
- `follow`: keep current cursor-follow behavior (default when `mode` omitted or `"follow"`).
- `dock`: `fixed inset-0 z-[100] flex items-start bg-black/55` (or design-token equivalent), inner row `m-lg` left-aligned: art then text panel; `pointer-events-auto` on content; include `extras` after oracle.

Do not wire board yet.

- [ ] **Step 4: Run — expect PASS**

```bash
cd client && bunx vitest run lib/deck-builder/card-hover-preview.test.ts app/shell/decks/builder/story.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/lib/deck-builder/card-hover-preview.ts client/lib/deck-builder/card-hover-preview.test.ts
git commit -m "feat(client): add dock mode to shared card hover preview"
```

---

### Task 4: Board inspect uses dock mode (topmost)

**Files:**
- Modify: `client/app/board/html/inspect.ts`
- Modify: `client/app/board/html/overlays.ts`
- Test: `client/app/board/scene.test.ts` and/or `inspect-pile-concede.test.ts` (Scene for overlay)

**Interfaces:**
- Consumes: `cardHoverPreviewView` dock mode from Task 3; existing `InspectPin` / modifiers
- Produces: `inspectView` returns dock preview; z above concede/result

- [ ] **Step 1: Failing Scene test — dock chrome when pinned**

Add to `client/app/board/scene.test.ts` (or inspect suite with Scene harness used for overlays):

```ts
test("inspect overlay docks left with backdrop when pinned", () => {
  const model = {
    ...initialBoardModel(),
    inspectPin: { name: "Sol Ring", prepared: false },
    inspectCard: {
      /* minimal CatalogCard with oracle */
      id: "sol",
      name: "Sol Ring",
      oracle: "{T}: Add {C}.",
      default_print: "p1",
    },
  };
  // render board overlays / inspectView via existing overlayScene helper
  overlayScene(
    model,
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.text("{T}: Add {C}.")).toExist(), // or split oracle visible text
  );
});
```

Adapt card fixture to real `CatalogCard` shape in repo.

- [ ] **Step 2: Run — expect FAIL if layout still centered-only / missing oracle path**

```bash
cd client && bunx vitest run app/board/scene.test.ts
```

- [ ] **Step 3: Refactor `inspectView` to dock wrapper**

- Build modifier ledger Html as today.
- Call shared dock preview with print/name/oracle/approximates/face/extras/flip+close controls.
- Keep `InspectDismissed` / `InspectFlipFace` message wiring.
- Ensure overlay z-index is highest (e.g. `z-[100]` on dock root). In `overlays.ts`, render `inspectView` **after** `concedeDialogView` and `resultOverlayView`. Portrait gate if board-scoped must sit under inspect when pin active (per spec).

- [ ] **Step 4: Run board inspect + scene suites — PASS**

```bash
cd client && bunx vitest run app/board/scene.test.ts app/board/inspect-pile-concede.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/inspect.ts client/app/board/html/overlays.ts client/app/board/scene.test.ts
git commit -m "fix(client): dock board inspect via shared card preview"
```

---

### Task 5: Hand drag-to-play sensitivity

**Files:**
- Modify: `client/app/board/html/hand.ts` — export slack constant
- Modify: `client/app/board/submodel.ts` — threshold math
- Modify: `client/app/board/hand-drag.test.ts`
- Modify: `client/app/board/scene.test.ts` — comments/assertions for new threshold

**Interfaces:**
- Consumes: `HAND_BAR_H`, `planHandDrop(action, card, y, threshold)`
- Produces: `HAND_PLAY_SLACK_PX = 96` and  
  `threshold = viewport.height - HAND_BAR_H + HAND_PLAY_SLACK_PX`

Semantics: release inside the top 96px of the hand bar still commits play (shorter lift).

- [ ] **Step 1: Failing test — release in slack band plays**

In `hand-drag.test.ts`, with default `BOARD_VIEWPORT` height `H`:

```ts
it("plays when drag ends in the hand-bar slack band", () => {
  // threshold_old = H - HAND_BAR_H
  // yJustInsideOldBar = threshold_old + 40  (would IGNORE before slack)
  // with slack 96, yJustInsideOldBar < threshold_new → PLAY
  const y = /* BOARD_VIEWPORT.height - HAND_BAR_H + 40 */;
  // ... HandDragStarted then HandDragEnded({ x: 400, y })
  expect(commands).toHaveLength(1);
});
```

Keep a test that `y` near the bottom of the bar still ignores (e.g. `H - 20`).

- [ ] **Step 2: Run — expect FAIL**

```bash
cd client && bunx vitest run app/board/hand-drag.test.ts
```

- [ ] **Step 3: Implement slack**

```ts
// hand.ts
/** How far into the hand bar a release may still count as play (px). */
export const HAND_PLAY_SLACK_PX = 96;
```

```ts
// submodel.ts
import { HAND_BAR_H, HAND_PLAY_SLACK_PX } from "./html/hand";
const threshold = model.viewport.height - HAND_BAR_H + HAND_PLAY_SLACK_PX;
```

Update `scene.test.ts` comments that hard-code `viewport - HAND_BAR_H`.

- [ ] **Step 4: Run — PASS**

```bash
cd client && bunx vitest run app/board/hand-drag.test.ts app/board/scene.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/hand.ts client/app/board/submodel.ts client/app/board/hand-drag.test.ts client/app/board/scene.test.ts
git commit -m "fix(client): shorten hand drag-to-play lift threshold"
```

---

### Task 6: Remove resting-card name paint

**Files:**
- Modify: `client/app/board/canvas/scene.ts` — drop `Canvas.Text` `card.name` on resting cards
- Modify: `client/app/board/canvas/scene.test.ts` — assert names not in shapes
- Modify: `client/app/board/bitmap/paint-cards.ts` only if names render outside the card rect (keep in-face placeholder text for missing art **inside** the clipped rect, or remove if tests show under-card bleed)

**Interfaces:**
- Consumes: `sceneShapes` / `paintCardArt`
- Produces: no free-floating / under-card name labels on resting permanents

- [ ] **Step 1: Failing test**

```ts
it("does not paint resting card names as canvas text", () => {
  const shapes = sceneShapes(stateWithNamedCard, { /* camera etc */ });
  const texts = shapes.filter((s) => s._tag === "Text" && s.content === "Swamp");
  expect(texts).toHaveLength(0);
});
```

Adapt to actual shape walk (nested Groups).

- [ ] **Step 2: Run — FAIL**

```bash
cd client && bunx vitest run app/board/canvas/scene.test.ts
```

- [ ] **Step 3: Remove name Text from vector card chrome; fix any under-card bitmap bleed**

Keep P/T text. Do not remove stack/pile/inspect names.

- [ ] **Step 4: Run — PASS**

```bash
cd client && bunx vitest run app/board/canvas/scene.test.ts app/board/bitmap/mount.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/board/canvas/scene.ts client/app/board/canvas/scene.test.ts client/app/board/bitmap/paint-cards.ts
git commit -m "fix(client): remove resting battlefield card name labels"
```

---

### Task 7: Implement layer stack (arrows, flights, z-index)

**Files:**
- Modify: `client/app/board/view.ts` — DOM composition order
- Modify: `client/app/board/bitmap/mount.ts` — permanents vs flights paint split
- Modify: `client/app/board/html/overlays.ts` — z-index + child order
- Modify: `client/app/board/html/mana-tray.ts` — battlefield mana under permanents (`z` ≤ permanents)
- Modify: `client/app/board/html/hand.ts` — hand layer z
- Modify: `client/app/board/bitmap/mount.test.ts` / `canvas/scene.test.ts` — arrow above cards; flight layer assertions

**Interfaces:**
- Consumes: locked stack from Task 1
- Produces: composition matching layers 1–10

**Target composition (`view.ts` children, bottom→top):**

1. `Canvas.view` — felt, seats, avatars paint, arrows (including combat drag + aim)  
2. Bitmap canvas — **resting permanents only** (no flights)  
3. Overlays slice A — battlefield mana tray, GY/exile/command pile chrome if DOM, stack **below** flights…  
4. Hand + spell/payment mana (hand layer)  
5. **Flight bitmap canvas** (new Mount or second publish path) — flights only  
6. Life-orb hit targets  
7. Prompts  
8. Turn HUD / priority / discoverability  
9. Concede/result (under inspect)  
10. Inspect dock  

If splitting overlays is cleaner than splitting `boardOverlays`, do that — keep one function with documented layer comments.

- [ ] **Step 1: Failing tests**

```ts
// mount.test.ts or arrows test: combat drag arrow paint occurs after card blit in the frame path
it("paints combat drag arrow after resting cards", () => { /* spy paint order or read mount.ts order */ });

// scene / view structure test if available: flight layer testid exists above hand z
```

Also add/adjust: declare-attackers drag uses same `paintArrow` / `combatDrag` path already after cards on bitmap — if DOM hand was covering arrows, fix by ensuring arrows stay on canvas under HTML but **above bitmap cards**, and hand does not cover the battlefield mat (hand is bottom strip only). Primary bug “arrows under cards” = arrow pass before card blit → reorder `paintCurrentFrame` to cards then arrows (today arrows are already after cards in `mount.ts` — then fix **vector** `sceneShapes` order / DOM cards). Audit both canvas scene shape order and bitmap order; fix the failing surface.

- [ ] **Step 2: Run — document which assertion fails**

```bash
cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/canvas/scene.test.ts app/board/canvas/arrows.test.ts
```

- [ ] **Step 3: Implement reorder**

- Canvas scene: felt → seats → avatars → **arrows last among vector** (cards should not be vector faces if bitmap owns them; remove duplicate vector card faces if they paint above arrows).  
- Bitmap: resting cards → (no arrows here if arrows are vector) OR cards then arrows if arrows stay bitmap.  
- Flights on separate canvas above hand.  
- z-index tokens: mana-tray `z-[5]` (under), hand `z-20`, flights layer `z-30`, prompts `z-40`, HUD `z-50`, inspect `z-[100]`.

- [ ] **Step 4: Run focused suites — PASS**

```bash
cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/canvas/scene.test.ts app/board/html/surfaces.test.ts app/board/scene.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/board/view.ts client/app/board/bitmap/mount.ts client/app/board/html/overlays.ts client/app/board/html/mana-tray.ts client/app/board/html/hand.ts client/app/board/bitmap/mount.test.ts client/app/board/canvas/scene.ts
git commit -m "fix(client): align board paint and z-index with layer stack"
```

---

### Task 8: Lobby Bring + Back

**Files:**
- Modify: `client/app/shell/lobby/view.ts`
- Modify: `client/app/shell/lobby/entry.test.ts`
- Modify: `client/app/shell/surfaces.test.ts` if lobby surfaces assert select

**Interfaces:**
- Consumes: `selectedDeckId`, `pickedDeckName`, `routePath(HomeRoute())`
- Produces: pre-pick entry/claim UI with `lobby-bring` + `lobby-back`; no `#lobby-deck`

- [ ] **Step 1: Replace obsolete select-binding test; add Bring/Back tests**

```ts
test("entry with pre-picked deck shows Bring text and Back, not a select", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck, other] },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-bring")).toExist(),
    Scene.expect(Scene.text("Tokens")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.text("Back")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
  );
});

test("claim seat pre-pick includes Back to decks", () => {
  // tableLobbyModel with selectedDeckId: 7
  Scene.expect(Scene.testId("lobby-back")).toExist();
});
```

Remove or rewrite `entry select shows the pre-picked deck among multiple decks` — select must be absent when pre-picked.

- [ ] **Step 2: Run — FAIL**

```bash
cd client && bunx vitest run app/shell/lobby/entry.test.ts
```

- [ ] **Step 3: Implement view**

In `entry()` when `model.selectedDeckId != null`:

```ts
h.div(
  [h.Class("flex flex-wrap items-center gap-sm")],
  [
    h.span(
      [h.Class("text-label text-lichen"), h.DataAttribute("testid", "lobby-bring")],
      ["Bring: ", h.b([], [pickedDeckName(model, decks)])],
    ),
    h.a(
      [
        h.Href(routePath(HomeRoute())),
        h.DataAttribute("testid", "lobby-back"),
        h.Class(buttonClass("quiet")), // match existing secondary button styles
      ],
      ["Back"],
    ),
  ],
);
```

Same Bring + Back on claim-seat pre-pick branch. When `selectedDeckId == null` and decks exist, keep `deckPicker`.

Import `HomeRoute`, `routePath` from `../../routes` (adjust relative path).

- [ ] **Step 4: Run — PASS**

```bash
cd client && bunx vitest run app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/view.ts client/app/shell/lobby/entry.test.ts client/app/shell/surfaces.test.ts
git commit -m "fix(client): lobby Bring text and Back when deck pre-picked"
```

---

### Task 9: Board layout collisions

**Files:**
- Modify: `client/app/board/geometry/layout.ts` (and density/camera as needed)
- Test: `client/app/board/geometry/layout.test.ts` (extend)

**Interfaces:**
- Consumes: seat band / zone column / command zone geometry
- Produces: mats and labels that do not overlap card names/command art; avatar clear bands honored

- [ ] **Step 1: Capture failing layout assertions from current bugs**

Add tests for regressions visible in artifacts (`board-A.png` class of bugs):

- Command zone label does not share the card art AABB (or label suppressed — prefer no duplicate text label if art+name on card).
- Seat band aspect: assert band height/width ratios stay within documented bounds already in layout tests; tighten if mats are “too tall/narrow”.

If a precise numeric invariant is unclear, write the smallest characterization test that fails on current `layout()` output for a 2-player fixture (e.g. command label anchor outside card rect).

- [ ] **Step 2: Run — FAIL**

```bash
cd client && bunx vitest run app/board/geometry/layout.test.ts
```

- [ ] **Step 3: Fix layout/camera packing**

Minimal geometry changes only. Respect layer 2 clear bands for avatars.

- [ ] **Step 4: Run — PASS**

```bash
cd client && bunx vitest run app/board/geometry/layout.test.ts app/board/scene.test.ts
```

- [ ] **Step 5: Commit**

```bash
git add client/app/board/geometry/layout.ts client/app/board/geometry/layout.test.ts
git commit -m "fix(client): ease board seat and command zone layout collisions"
```

---

### Task 10: Live triage remainder + mark design Done

**Files:**
- Modify: current board specs — record landed behavior
- Possibly small fix commits if Host / hand-hide / builder hover / session gate still fail live

**Interfaces:**
- Consumes: Interaction checklist in `.agents/skills/verify/SKILL.md`
- Produces: verified checklist notes; design Status Done

- [ ] **Step 1: Run unit suites for this plan**

```bash
cd client && bunx vitest run \
  app/board/html/keyboard-mount.test.ts \
  lib/deck-builder/card-hover-preview.test.ts \
  app/board/inspect-pile-concede.test.ts \
  app/board/hand-drag.test.ts \
  app/board/scene.test.ts \
  app/board/canvas/scene.test.ts \
  app/board/bitmap/mount.test.ts \
  app/shell/lobby/entry.test.ts
```

Expected: all PASS.

- [ ] **Step 2: Live Interaction checklist**

Exercise: Host table; Alt/Option dock inspect; drag-play (short lift + hide); builder hover; pre-pick Bring + Back; declare-attackers arrow above cards. Fix only what still fails (separate focused commits).

- [ ] **Step 3: Mark design Done**

```markdown
**Status:** Done
```

- [ ] **Step 4: Commit + push**

```bash
git add docs/superpowers/specs/2026-07-20-board-composition.md docs/superpowers/specs/2026-07-20-battlefield.md docs/superpowers/specs/2026-07-20-card-inspect.md
git commit -m "docs: mark foldkit remaining bugs and board layers design done"
git push -u origin cursor/foldkit-migration-design-1ef0
```

---

## Self-review (author)

1. **Spec coverage:** Alt/Option + dock preview (Tasks 2–4); triage others (Task 10); drag slack (5); Bring+Back (8); layout (9); under-card names (6); arrows/flights/layers (1, 7); inspect topmost (4, 7); two mana surfaces (7).
2. **Placeholders:** None intentional; Task 9 allows characterization tests when numeric bounds unclear.
3. **Types:** `CardPreviewMode` / dock args defined in Task 3; `HAND_PLAY_SLACK_PX` in Task 5; layer z tokens in Task 7.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-22-foldkit-remaining-bugs-and-board-layers.md`. Two execution options:

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration  

**2. Inline Execution** — execute tasks in this session using executing-plans, batch with checkpoints  

Which approach?
