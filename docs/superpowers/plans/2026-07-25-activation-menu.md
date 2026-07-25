# Activation Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the battlefield activation SVG donut with a single card-anchored HUD list menu (readable labels, optional cost chips, arm/commit pointer model, edge-aware placement).

**Architecture:** Keep option building + press reducer in `client/app/board/geometry/radial.ts`; add pure placement + cost-chip helpers there. Replace `activation-radial.ts` with `activation-menu.ts` (DOM list + scrim). `BoardModel` armed/hover indices and `RadialWedge*` / `RadialOptionPicked` messages stay index-based (no dual menu). Living feature spec becomes the menu; wedge geometry is deleted.

**Tech Stack:** Foldkit HTML (`foldkit/html`), Vitest Scene tests, Tailwind HUD tokens (`bg-forest-hud`, `border-vine`, `rounded-hud`), existing `costPips` / mana-font for chips.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-25-activation-menu-design.md`
- Client-only; no proto/schema activation-cost field
- Never invent costs by parsing labels/oracle
- One menu surface only (no hybrid radial)
- TDD: failing test → implement → pass → commit per task
- Angular commits (`feat(client):`, `test(client):`, `docs:`)
- Branch: `cursor/activation-menu-redesign-a366`
- No press `scale`/`translate` that shrinks hit targets
- Preserve `radialOptions` / `commitRadialIndex` / auto-tap hover semantics

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/geometry/radial.ts` | Options, press reducer, screen center; **add** menu placement + cost chip; **delete** wedge path/index/radii once unused |
| `client/app/board/geometry/radial.test.ts` | Unit tests for placement, cost chip, options, press; drop wedge tests |
| `client/app/board/html/activation-menu.ts` | **Create** — list menu view + `selectedRadialOptions` (move from radial file) |
| `client/app/board/html/activation-radial.ts` | **Delete** after menu lands |
| `client/app/board/html/overlays.ts` | Compose `activationMenuView` |
| `client/app/board/submodel.ts` | Import `selectedRadialOptions` from new path |
| `client/app/board/scene.test.ts` | Scene/outcome tests for menu testids + placement |
| `client/app/board/html/surfaces.test.ts` | Add/extend Scene coverage if overlays suite owns surface presence |
| `docs/superpowers/specs/2026-07-21-activation-radial.md` | Rewrite as current activation-menu surface (or replace + retarget README) |
| `docs/superpowers/specs/README.md` | Point living row at menu; mark design absorbed |
| `docs/client-canvas-map.md` | `activation-menu.ts` in HTML chrome list |
| `docs/superpowers/specs/2026-07-20-board-composition.md` | Update if it names the radial file |

---

### Task 1: Menu placement helpers

**Files:**
- Modify: `client/app/board/geometry/radial.ts`
- Modify: `client/app/board/geometry/radial.test.ts`

**Interfaces:**
- Consumes: `radialScreenCenter`, viewport size, card screen size (`card.w * zoom`, `card.h * zoom` or pass explicit `{ w, h }`)
- Produces:
  - `ACTIVATION_MENU_WIDTH_PX = 240`
  - `ACTIVATION_MENU_MAX_HEIGHT_PX = 280`
  - `ACTIVATION_MENU_GAP_PX = 8`
  - `activationMenuEstimatedHeight(optionCount: number, rowPx?: number): number`
  - `activationMenuPlacement(center, cardScreen: { w: number; h: number }, menu: { width: number; height: number }, viewport: { width: number; height: number }, gap?: number): { left: string; top: string; width: string; maxHeight: string }` — CSS `%` of viewport (same stretch invariant as `radialOverlayPlacement`)

- [ ] **Step 1: Write the failing tests**

Append to `client/app/board/geometry/radial.test.ts`:

```ts
import {
  ACTIVATION_MENU_GAP_PX,
  ACTIVATION_MENU_MAX_HEIGHT_PX,
  ACTIVATION_MENU_WIDTH_PX,
  activationMenuEstimatedHeight,
  activationMenuPlacement,
} from "./radial";

describe("activationMenuEstimatedHeight", () => {
  it("grows with option count and caps at max height", () => {
    expect(activationMenuEstimatedHeight(1)).toBeLessThan(activationMenuEstimatedHeight(4));
    expect(activationMenuEstimatedHeight(50)).toBe(ACTIVATION_MENU_MAX_HEIGHT_PX);
  });
});

describe("activationMenuPlacement", () => {
  const menu = { width: ACTIVATION_MENU_WIDTH_PX, height: 120 };
  const card = { w: 96, h: 134 };
  const vp = { width: 1440, height: 900 };

  it("prefers the right of the card when there is room", () => {
    const center = { x: 400, y: 450 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    const expected = center.x + card.w / 2 + ACTIVATION_MENU_GAP_PX;
    expect(leftPx).toBeCloseTo(expected, 1);
    expect(place.width).toBe(`${(menu.width / vp.width) * 100}%`);
  });

  it("flips to the left when the right side overflows", () => {
    const center = { x: 1400, y: 450 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    expect(leftPx + menu.width).toBeLessThanOrEqual(vp.width + 0.5);
    expect(leftPx).toBeLessThan(center.x);
  });

  it("flips above when horizontal sides overflow", () => {
    // Narrow viewport: menu cannot fit left or right of a centered card.
    const narrow = { width: 300, height: 900 };
    const center = { x: 150, y: 450 };
    const wideMenu = { width: 240, height: 80 };
    const place = activationMenuPlacement(center, card, wideMenu, narrow);
    const topPx = (Number.parseFloat(place.top) / 100) * narrow.height;
    expect(topPx + wideMenu.height).toBeLessThanOrEqual(center.y - card.h / 2 + 0.5);
  });

  it("clamps so the panel stays fully on-screen", () => {
    const center = { x: 10, y: 10 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    const topPx = (Number.parseFloat(place.top) / 100) * vp.height;
    expect(leftPx).toBeGreaterThanOrEqual(0);
    expect(topPx).toBeGreaterThanOrEqual(0);
    expect(leftPx + menu.width).toBeLessThanOrEqual(vp.width + 0.5);
    expect(topPx + menu.height).toBeLessThanOrEqual(vp.height + 0.5);
  });

  it("returns zero box when viewport is invalid", () => {
    expect(activationMenuPlacement({ x: 1, y: 1 }, card, menu, { width: 0, height: 0 })).toEqual({
      left: "0%",
      top: "0%",
      width: "0%",
      maxHeight: "0%",
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run app/board/geometry/radial.test.ts -t "activationMenu"`

Expected: FAIL — exports missing.

- [ ] **Step 3: Minimal implementation**

In `client/app/board/geometry/radial.ts`, add:

```ts
export const ACTIVATION_MENU_WIDTH_PX = 240;
export const ACTIVATION_MENU_MAX_HEIGHT_PX = 280;
export const ACTIVATION_MENU_GAP_PX = 8;
const ACTIVATION_MENU_ROW_PX = 36;
const ACTIVATION_MENU_PAD_PX = 16;

export function activationMenuEstimatedHeight(optionCount: number, rowPx = ACTIVATION_MENU_ROW_PX): number {
  const n = Math.max(0, optionCount);
  return Math.min(ACTIVATION_MENU_MAX_HEIGHT_PX, n * rowPx + ACTIVATION_MENU_PAD_PX);
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

/**
 * Card-anchored menu box in % of the board viewport (CSS-stretch safe).
 * Prefer right → left → above → below; then clamp fully on-screen.
 */
export function activationMenuPlacement(
  center: { x: number; y: number },
  cardScreen: { w: number; h: number },
  menu: { width: number; height: number },
  viewport: { width: number; height: number },
  gap = ACTIVATION_MENU_GAP_PX,
): { left: string; top: string; width: string; maxHeight: string } {
  if (viewport.width <= 0 || viewport.height <= 0) {
    return { left: "0%", top: "0%", width: "0%", maxHeight: "0%" };
  }
  const halfW = cardScreen.w / 2;
  const halfH = cardScreen.h / 2;
  const candidates = [
    { x: center.x + halfW + gap, y: center.y - menu.height / 2 },
    { x: center.x - halfW - gap - menu.width, y: center.y - menu.height / 2 },
    { x: center.x - menu.width / 2, y: center.y - halfH - gap - menu.height },
    { x: center.x - menu.width / 2, y: center.y + halfH + gap },
  ];
  const fits = (p: { x: number; y: number }) =>
    p.x >= 0 && p.y >= 0 && p.x + menu.width <= viewport.width && p.y + menu.height <= viewport.height;
  const raw = candidates.find(fits) ?? candidates[0]!;
  const x = clamp(raw.x, 0, Math.max(0, viewport.width - menu.width));
  const y = clamp(raw.y, 0, Math.max(0, viewport.height - menu.height));
  return {
    left: `${(x / viewport.width) * 100}%`,
    top: `${(y / viewport.height) * 100}%`,
    width: `${(menu.width / viewport.width) * 100}%`,
    maxHeight: `${(Math.min(menu.height, ACTIVATION_MENU_MAX_HEIGHT_PX) / viewport.height) * 100}%`,
  };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/board/geometry/radial.test.ts -t "activationMenu"`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/geometry/radial.ts client/app/board/geometry/radial.test.ts
git commit -m "feat(client): add activation menu placement helpers"
```

---

### Task 2: Cost chip from structured option data

**Files:**
- Modify: `client/app/board/geometry/radial.ts`
- Modify: `client/app/board/geometry/radial.test.ts`

**Interfaces:**
- Consumes: `RadialOption`, `ActionView.taps_self`, `ActionView.x_cost`
- Produces:
  - `export type ActivationCostChip = { kind: "tap" } | { kind: "mana"; cost: WireCost } | { kind: "tap_and_mana"; cost: WireCost }`
  - `activationCostChip(opt: RadialOption): ActivationCostChip | null`

Rules (client-only, no label parsing):
- `tap_for_mana` → `{ kind: "tap" }`
- action with `x_cost != null` and `taps_self` → `{ kind: "tap_and_mana", cost }`
- action with `x_cost != null` → `{ kind: "mana", cost }`
- action with `taps_self === true` only → `{ kind: "tap" }`
- else → `null`

- [ ] **Step 1: Write the failing tests**

```ts
import { activationCostChip } from "./radial";

describe("activationCostChip", () => {
  it("shows tap for tap_for_mana", () => {
    expect(activationCostChip({ kind: "tap_for_mana", label: "Tap for mana", disabled: false })).toEqual({
      kind: "tap",
    });
  });

  it("shows tap when action.taps_self is true", () => {
    expect(
      activationCostChip({
        kind: "action",
        label: "Scry 1",
        disabled: false,
        action: activate({ taps_self: true }),
      }),
    ).toEqual({ kind: "tap" });
  });

  it("shows mana from x_cost when present", () => {
    const cost = { generic: 1, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 1 };
    expect(
      activationCostChip({
        kind: "action",
        label: "X pump",
        disabled: false,
        action: activate({ has_x: true, x_cost: cost }),
      }),
    ).toEqual({ kind: "mana", cost });
  });

  it("combines tap and mana when both apply", () => {
    const cost = { generic: 0, colored: [0, 1, 0, 0, 0], has_x: false };
    expect(
      activationCostChip({
        kind: "action",
        label: "Pay U, tap",
        disabled: false,
        action: activate({ taps_self: true, x_cost: cost }),
      }),
    ).toEqual({ kind: "tap_and_mana", cost });
  });

  it("returns null when no structured cost exists", () => {
    expect(
      activationCostChip({
        kind: "action",
        label: "Add {U}{R}",
        disabled: false,
        action: activate({ label: "Add {U}{R}" }),
      }),
    ).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/board/geometry/radial.test.ts -t "activationCostChip"`

Expected: FAIL — `activationCostChip` missing.

- [ ] **Step 3: Minimal implementation**

```ts
import type { ActionView, WireCost } from "~/wire/types";

export type ActivationCostChip =
  | { kind: "tap" }
  | { kind: "mana"; cost: WireCost }
  | { kind: "tap_and_mana"; cost: WireCost };

export function activationCostChip(opt: RadialOption): ActivationCostChip | null {
  if (opt.kind === "tap_for_mana") return { kind: "tap" };
  const taps = opt.action.taps_self === true;
  const mana = opt.action.x_cost ?? null;
  if (mana != null && taps) return { kind: "tap_and_mana", cost: mana };
  if (mana != null) return { kind: "mana", cost: mana };
  if (taps) return { kind: "tap" };
  return null;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/board/geometry/radial.test.ts -t "activationCostChip"`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/board/geometry/radial.ts client/app/board/geometry/radial.test.ts
git commit -m "feat(client): derive activation menu cost chips from option data"
```

---

### Task 3: Activation menu view + Scene tests

**Files:**
- Create: `client/app/board/html/activation-menu.ts`
- Modify: `client/app/board/html/overlays.ts`
- Modify: `client/app/board/submodel.ts` (import path for `selectedRadialOptions`)
- Modify: `client/app/board/scene.test.ts`
- Delete: `client/app/board/html/activation-radial.ts` (end of this task, after move)

**Interfaces:**
- Consumes: placement + cost chip helpers; `RadialWedgeArmed` / `Released` / `Hovered` / `RadialOptionPicked`; `BoardModel.radialPress` / `radialHover`
- Produces: `activationMenuView(board, state): Html | null`, `selectedRadialOptions(board, state)`
- Testids: `activation-menu`, `activation-menu-row-${radialOptionKey(opt)}`, optional `activation-menu-cost` on chip container

- [ ] **Step 1: Write failing Scene tests (replace radial assertions)**

In `client/app/board/scene.test.ts`, replace the three radial presence/placement tests with:

```ts
test("selected permanent with tap-for-mana shows activation menu row", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const base = viewModel(fold(state({ objects: [land], can_act: true })));
  const selected: ViewModel = { ...base, board: { ...base.board, selectedId: 5 } };
  overlayScene(
    selected,
    Scene.expect(Scene.testId("activation-menu")).toExist(),
    Scene.expect(Scene.testId("activation-menu-row-tap_for_mana")).toExist(),
    Scene.expect(Scene.testId("activation-menu-cost")).toExist(),
  );
});

test("selected tapped mana source keeps its disabled tap-for-mana row visible", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    tapped: true,
    toughness: 0,
  });
  const base = viewModel(fold(state({ objects: [land], can_act: true })));
  const selected: ViewModel = { ...base, board: { ...base.board, selectedId: 5 } };
  overlayScene(
    selected,
    Scene.expect(Scene.testId("activation-menu")).toExist(),
    Scene.expect(Scene.testId("activation-menu-row-tap_for_mana")).toExist(),
    Scene.expect(Scene.testId("activation-menu-row-tap_for_mana")).toHaveAttribute("aria-disabled", "true"),
  );
});

test("activation menu is placed beside the selected card screen center", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const gameFold = fold(state({ objects: [land], can_act: true }));
  const board: BoardModel = { ...initialBoardModel(), selectedId: 5 };
  const visible = gameFold.state;
  expect(visible).not.toBeNull();
  if (visible == null) return;
  const card = layout(visible, visible.viewer).find((c) => c.id === land.id);
  expect(card).toBeDefined();
  if (card == null) return;
  const center = worldToScreen(board.camera, card.x + card.w / 2, card.y + card.h / 2);
  const zoom = board.camera.zoom;
  const place = activationMenuPlacement(
    center,
    { w: card.w * zoom, h: card.h * zoom },
    { width: ACTIVATION_MENU_WIDTH_PX, height: activationMenuEstimatedHeight(1) },
    board.viewport,
  );
  const selected: ViewModel = { board, fold: gameFold, tableId: "T1" };

  overlayScene(
    selected,
    Scene.expect(Scene.testId("activation-menu-panel")).toHaveStyle("left", place.left),
    Scene.expect(Scene.testId("activation-menu-panel")).toHaveStyle("top", place.top),
    Scene.expect(Scene.testId("activation-menu-panel")).toHaveStyle("width", place.width),
  );
});
```

Update imports: drop `activationRadialOuterRadius` / `radialOverlayPlacement`; add `ACTIVATION_MENU_WIDTH_PX`, `activationMenuEstimatedHeight`, `activationMenuPlacement`.

Keep existing `RadialOptionPicked` outcome tests unchanged (they do not assert DOM).

- [ ] **Step 2: Run Scene tests to verify they fail**

Run: `cd client && bunx vitest run app/board/scene.test.ts -t "activation menu|tap-for-mana shows|disabled tap-for-mana"`

Expected: FAIL — `activation-menu` absent (radial still renders).

- [ ] **Step 3: Implement `activation-menu.ts` and wire overlays**

Create `client/app/board/html/activation-menu.ts`:

- Move `selectedRadialOptions` from `activation-radial.ts` unchanged.
- `activationMenuView`:
  - Same guards (selected battlefield object, options non-empty).
  - Scrim button → `RadialWedgeReleased({ index: null })` on pointer-up.
  - Panel `data-testid="activation-menu-panel"` with `Style` from `activationMenuPlacement` (`left`/`top`/`width`/`maxHeight`), classes:
    `pointer-events-auto absolute z-[31] flex max-h-full flex-col overflow-y-auto rounded-hud border border-vine/50 bg-forest-hud p-sm text-chip text-snow shadow-hud`
  - Outer wrapper: `data-testid="activation-menu"`, `pointer-events-none fixed inset-0 z-30`.
  - Each row: `button` or focusable element with `data-testid={`activation-menu-row-${key}`}`, `data-wedge={i}` (keep attribute name so any `radialWedgeFromElement` callers still work if present), `role="button"`, `aria-disabled`, arm/release/hover/keydown handlers matching the old radial (no scale transforms).
  - Label: full `opt.label` with `line-clamp-2` / `text-left` (no 18-char truncate).
  - Cost chip: if `activationCostChip(opt)` non-null, render a span `data-testid="activation-menu-cost"`:
    - tap → mana-font tap glyph (`ms ms-tap` or project’s existing tap class from mana font)
    - mana → `costPips(cost)` disks like `hand.ts`
    - tap_and_mana → tap + pips
  - Armed/hover row: stronger border (`border-priority-gold` or `bg-llanowar-deep`); disabled: `opacity-60 cursor-not-allowed`.

Wire `overlays.ts`: `activationMenuView` instead of `activationRadialView`.

Point `submodel.ts` import of `selectedRadialOptions` at `./html/activation-menu`.

Delete `activation-radial.ts`.

- [ ] **Step 4: Run Scene + geometry tests**

Run: `cd client && bunx vitest run app/board/scene.test.ts app/board/geometry/radial.test.ts`

Expected: Scene menu tests PASS. Geometry may still have wedge tests green until Task 4.

- [ ] **Step 5: Commit**

```bash
git add client/app/board/html/activation-menu.ts client/app/board/html/overlays.ts client/app/board/submodel.ts client/app/board/scene.test.ts
git rm client/app/board/html/activation-radial.ts
git commit -m "feat(client): replace activation radial with card-anchored menu"
```

---

### Task 4: Retire wedge geometry

**Files:**
- Modify: `client/app/board/geometry/radial.ts`
- Modify: `client/app/board/geometry/radial.test.ts`
- Grep: ensure no remaining imports of `wedgePath`, `wedgeIndex`, `wedgeLabelPoint`, `activationRadialInnerRadius`, `activationRadialOuterRadius`, `activationRadialRadius`, `radialOverlayPlacement`

**Interfaces:**
- Keep: `RadialOption`, `radialOptions`, `radialOptionKey`, `radialScreenCenter`, `RadialPress`, `radialPressDown`, `radialPressUp`, `radialWedgeFromElement`, `radialWedgeAtPoint`, menu placement + cost chip
- Delete: wedge path/index/label + donut radii + `radialOverlayPlacement` if unused

- [ ] **Step 1: Confirm nothing imports wedge helpers**

Run: `rg "wedgePath|wedgeIndex|wedgeLabelPoint|activationRadialInnerRadius|activationRadialOuterRadius|activationRadialRadius|radialOverlayPlacement" client --glob '*.ts'`

Expected: only `radial.ts` / `radial.test.ts` (and maybe stale scene imports — fix those).

- [ ] **Step 2: Delete wedge tests and dead exports**

Remove describes for radii, `wedgeIndex`, `wedgePath` / `wedgeLabelPoint`, and `radialOverlayPlacement` from `radial.test.ts`. Remove the corresponding functions/constants (`INNER_GAP_PX`, `MIN_RING_PX`, etc.) from `radial.ts`.

- [ ] **Step 3: Run full geometry + board scene suite**

Run: `cd client && bunx vitest run app/board/geometry/radial.test.ts app/board/scene.test.ts app/board/inspect-pile-concede.test.ts`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add client/app/board/geometry/radial.ts client/app/board/geometry/radial.test.ts client/app/board/scene.test.ts
git commit -m "refactor(client): remove unused activation wedge geometry"
```

---

### Task 5: Living feature spec + nav docs

**Files:**
- Rewrite: `docs/superpowers/specs/2026-07-21-activation-radial.md` → current menu behavior (same authoring template: Problem → Solution → Stories → Behavior → Implementation → Testing → Out of Scope). Prefer renaming file to `2026-07-25-activation-menu.md` **only if** you update every link in the same change; otherwise rewrite in place and retitle to “Activation Menu”.
- Modify: `docs/superpowers/specs/README.md` — living row points at menu; design row notes absorbed / superseded
- Modify: `docs/client-canvas-map.md` — `activation-menu.ts`
- Modify: `docs/superpowers/specs/2026-07-20-board-composition.md` if it cites the radial module
- Modify: `docs/superpowers/specs/2026-07-25-activation-menu-design.md` — Status: `superseded by living activation-menu feature spec`

- [ ] **Step 1: Rewrite living surface spec for the menu (no wedges, no migration history)**

Document today’s intended behavior after Tasks 1–4: card-anchored list, arm/commit, cost chip rules, placement order, testids.

- [ ] **Step 2: Update index + canvas map + design status**

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs docs/client-canvas-map.md
git commit -m "docs: record activation menu as living feature surface"
```

---

## Spec coverage checklist (self-review)

| Spec requirement | Task |
|------------------|------|
| Drop donut; single list menu | 3 |
| Card-anchored; prefer right → left → above → below; clamp | 1, 3 |
| Label + optional cost chip; no label parsing | 2, 3 |
| Arm / same-row commit / slide-off / scrim | 3 (reuse press reducer) |
| Keyboard Enter/Space | 3 |
| Hover → auto-tap preview | 3 (existing hover message) |
| Disabled visible, non-committing | 3 |
| Scroll when many options | 3 (`overflow-y-auto` + max height) |
| Forest HUD chrome / motion | 3 |
| Scene + placement + press tests | 1–4 |
| Living feature spec rewrite | 5 |
| No proto cost field | Global / Task 2 |

## Placeholder / consistency notes

- Message names stay `RadialWedge*` / `RadialOptionPicked` (index-based); do not introduce a second press channel.
- `data-wedge` attribute retained on rows for any element-from-point helpers; testids use `activation-menu-row-*`.
- Tap glyph: use the same mana-font class the client already uses for `{T}` elsewhere (`rg "ms-tap|ms-untap" client`); if none exists, render a small text `{T}` chip rather than inventing a new font pipeline in this plan.
