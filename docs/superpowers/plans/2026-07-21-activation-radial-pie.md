# Activation Radial Pie Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the activation radial’s floating chip buttons with a continuous SVG donut of angular wedges that commit on pointer-up, so the first deliberate press reliably activates.

**Architecture:** Pure geometry + pointer-press helpers in `lib/radial.ts`; `ActivationRadial` renders one fixed-position SVG donut over the existing dismiss scrim; `board.tsx` freezes the screen anchor when selection opens and hides empty radials. No engine/wire changes.

**Tech Stack:** SolidStart client (Solid signals), Vitest (`just client-test` / `cd client && bun run test`), DESIGN.md HUD tokens via Tailwind classes.

**Spec:** [docs/superpowers/specs/2026-07-21-activation-radial-pie-design.md](../specs/2026-07-21-activation-radial-pie-design.md)

## Global Constraints

- Client-only; no proto/engine/schema changes.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages (`feat:`, `fix:`, `test:`, `docs:`); PRs squash-merge.
- Work on a feature branch off `main` (continue `cursor/activation-radial-pie-design-6c98` or a sibling `cursor/…-6c98` if splitting).
- Keep dual-surface invariant: radial stays DOM overlay, not canvas paint.
- No press `scale`/`translate` on wedges (that shrinks the hit target under the cursor).
- Preserve `radialOptions` semantics and `session.play` / `tap_for_mana` wiring.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/src/lib/radial.ts` | Options (existing) + ring radii, wedge geometry, option keys, pointer-press reducer |
| `client/src/lib/radial.test.ts` | Unit tests for geometry + press reducer |
| `client/src/components/molecules/activation-radial.tsx` | SVG continuous-ring UI + pointer-up commit |
| `client/src/components/organisms/board.tsx` | Freeze anchor on open; empty → clear selection; pass frozen x/y/zoom |
| `docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md` | Note pie radial + pointer-up (short behavior update) |

---

### Task 1: Ring geometry helpers

**Files:**
- Modify: `client/src/lib/radial.ts`
- Modify: `client/src/lib/radial.test.ts`

**Interfaces:**
- Consumes: existing `activationRadialRadius(zoom: number): number`, `CARD_W` / `CARD_H` from `~/layout`
- Produces:
  - `activationRadialInnerRadius(zoom: number): number`
  - `activationRadialOuterRadius(zoom: number): number` (may grow past today’s outer so the ring clears card corners and keeps min thickness)
  - `wedgeIndex(angleRad: number, count: number): number` — `angleRad` in screen space with `0` = east, increasing clockwise **or** document the convention you pick and stick to it (recommend: `atan2(dy, dx)` raw, first wedge centered at `-π/2`)
  - `wedgePath(i: number, count: number, inner: number, outer: number): string` — SVG path `d` for wedge `i`
  - `wedgeLabelPoint(i: number, count: number, inner: number, outer: number): { x: number; y: number }` — center of wedge mid-radius, relative to ring center
  - `radialOptionKey(opt: RadialOption): string`

- [ ] **Step 1: Write the failing tests**

Append to `client/src/lib/radial.test.ts` (keep existing `activationRadialRadius` / `radialOptions` tests):

```ts
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  activationRadialRadius,
  radialOptionKey,
  wedgeIndex,
  wedgeLabelPoint,
  wedgePath,
} from "~/lib/radial";
import { CARD_H, CARD_W } from "~/layout";

describe("activationRadialInnerRadius / outer", () => {
  it("clears the upright card corners and keeps a usable ring thickness", () => {
    const zoom = 1;
    const inner = activationRadialInnerRadius(zoom);
    const outer = activationRadialOuterRadius(zoom);
    const corner = Math.hypot(CARD_W / 2, CARD_H / 2) * zoom;
    expect(inner).toBeGreaterThan(corner);
    expect(outer - inner).toBeGreaterThanOrEqual(36);
    expect(outer).toBeGreaterThanOrEqual(activationRadialRadius(zoom));
  });

  it("scales with zoom", () => {
    expect(activationRadialInnerRadius(2)).toBeGreaterThan(activationRadialInnerRadius(1));
    expect(activationRadialOuterRadius(2)).toBeGreaterThan(activationRadialOuterRadius(1));
  });
});

describe("wedgeIndex", () => {
  it("puts the top of the ring in wedge 0 when count is 4", () => {
    // atan2(-1, 0) === -π/2 — straight up from center
    expect(wedgeIndex(-Math.PI / 2, 4)).toBe(0);
  });

  it("wraps angles into [0, count)", () => {
    expect(wedgeIndex(Math.PI, 4)).toBeGreaterThanOrEqual(0);
    expect(wedgeIndex(Math.PI, 4)).toBeLessThan(4);
  });

  it("returns 0 for a single wedge at any angle", () => {
    expect(wedgeIndex(0, 1)).toBe(0);
    expect(wedgeIndex(2, 1)).toBe(0);
  });
});

describe("wedgePath / wedgeLabelPoint", () => {
  it("returns a non-empty path for each of 6 wedges", () => {
    for (let i = 0; i < 6; i++) {
      expect(wedgePath(i, 6, 50, 90).length).toBeGreaterThan(10);
    }
  });

  it("places the single-wedge label at the top", () => {
    const p = wedgeLabelPoint(0, 1, 50, 90);
    expect(p.x).toBeCloseTo(0, 5);
    expect(p.y).toBeLessThan(0);
  });
});

describe("radialOptionKey", () => {
  it("keys tap-for-mana and actions stably", () => {
    expect(radialOptionKey({ kind: "tap_for_mana", label: "Tap for mana" })).toBe("tap_for_mana");
    expect(
      radialOptionKey({
        kind: "action",
        label: "Pump",
        action: activate({ id: 42 }),
      }),
    ).toBe("action:42");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun run test src/lib/radial.test.ts`

Expected: FAIL (exports missing / not a function)

- [ ] **Step 3: Implement geometry in `radial.ts`**

Add (exact constants may match tests above):

```ts
import { CARD_H, CARD_W } from "~/layout";

const INNER_GAP_PX = 4;
const MIN_RING_PX = 36;

/** Keep existing activationRadialRadius for callers / tests. */

export function activationRadialInnerRadius(zoom: number): number {
  return Math.hypot(CARD_W / 2, CARD_H / 2) * zoom + INNER_GAP_PX;
}

export function activationRadialOuterRadius(zoom: number): number {
  const inner = activationRadialInnerRadius(zoom);
  return Math.max(activationRadialRadius(zoom), inner + MIN_RING_PX);
}

export function radialOptionKey(opt: RadialOption): string {
  if (opt.kind === "tap_for_mana") return "tap_for_mana";
  return `action:${opt.action.id}`;
}

/** Normalize atan2 angle so 0 is the start of wedge 0 (top-centered). */
export function wedgeIndex(angleRad: number, count: number): number {
  if (count <= 1) return 0;
  const slice = (2 * Math.PI) / count;
  // Shift so wedge 0 is centered on -π/2 (top).
  let a = angleRad + Math.PI / 2 + slice / 2;
  a = ((a % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return Math.min(count - 1, Math.floor(a / slice));
}

export function wedgePath(i: number, count: number, inner: number, outer: number): string {
  const slice = (2 * Math.PI) / count;
  const a0 = -Math.PI / 2 - slice / 2 + i * slice;
  const a1 = a0 + slice;
  const large = slice > Math.PI ? 1 : 0;
  const x = (r: number, a: number) => Math.cos(a) * r;
  const y = (r: number, a: number) => Math.sin(a) * r;
  return [
    `M ${x(outer, a0)} ${y(outer, a0)}`,
    `A ${outer} ${outer} 0 ${large} 1 ${x(outer, a1)} ${y(outer, a1)}`,
    `L ${x(inner, a1)} ${y(inner, a1)}`,
    `A ${inner} ${inner} 0 ${large} 0 ${x(inner, a0)} ${y(inner, a0)}`,
    "Z",
  ].join(" ");
}

export function wedgeLabelPoint(
  i: number,
  count: number,
  inner: number,
  outer: number,
): { x: number; y: number } {
  const slice = (2 * Math.PI) / count;
  const mid = -Math.PI / 2 + i * slice;
  const r = (inner + outer) / 2;
  return { x: Math.cos(mid) * r, y: Math.sin(mid) * r };
}
```

Adjust `wedgeIndex` if Step 4 shows the “top → 0” assertion off by one — fix the shift math, do not weaken the test.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun run test src/lib/radial.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/src/lib/radial.ts client/src/lib/radial.test.ts
git commit -m "feat(client): add continuous-ring radial geometry helpers"
```

---

### Task 2: Pointer-press reducer

**Files:**
- Modify: `client/src/lib/radial.ts`
- Modify: `client/src/lib/radial.test.ts`

**Interfaces:**
- Consumes: none beyond Task 1
- Produces:
  - `type RadialPress = { armed: number | null }`
  - `radialPressDown(state, wedgeIndex: number): RadialPress`
  - `radialPressUp(state, wedgeIndex: number | null): { state: RadialPress; commit: number | null; dismiss: boolean }`
  - Semantics:
    - Down on wedge `i` → `{ armed: i }`
    - Up on same `i` → commit `i`, clear armed, `dismiss: false`
    - Up on other wedge / `null` while armed → cancel (clear armed), `dismiss: false`
    - Up on `null` while not armed → `dismiss: true` (scrim)
    - Up on a wedge while not armed → treat as commit that wedge (defensive) **or** ignore; prefer commit that wedge for keyboard-parity simplicity only if you also fire from key handlers separately — **prefer: if `armed == null` and up on wedge, commit that wedge** so a lost down still works once

Lock the preferred semantics in tests as below (down→up same commits; armed + up elsewhere cancels; idle + up null dismisses; idle + up wedge commits).

- [ ] **Step 1: Write the failing tests**

```ts
import { radialPressDown, radialPressUp, type RadialPress } from "~/lib/radial";

const idle: RadialPress = { armed: null };

describe("radialPress", () => {
  it("commits when down and up on the same wedge", () => {
    const armed = radialPressDown(idle, 2);
    expect(armed).toEqual({ armed: 2 });
    const up = radialPressUp(armed, 2);
    expect(up.commit).toBe(2);
    expect(up.dismiss).toBe(false);
    expect(up.state.armed).toBeNull();
  });

  it("cancels when sliding off before release", () => {
    const armed = radialPressDown(idle, 1);
    const up = radialPressUp(armed, null);
    expect(up.commit).toBeNull();
    expect(up.dismiss).toBe(false);
    expect(up.state.armed).toBeNull();
  });

  it("dismisses on scrim up when nothing was armed", () => {
    const up = radialPressUp(idle, null);
    expect(up.commit).toBeNull();
    expect(up.dismiss).toBe(true);
  });

  it("commits an idle up on a wedge (no prior down)", () => {
    const up = radialPressUp(idle, 0);
    expect(up.commit).toBe(0);
    expect(up.dismiss).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun run test src/lib/radial.test.ts`

Expected: FAIL (missing exports)

- [ ] **Step 3: Implement reducer**

```ts
export type RadialPress = { armed: number | null };

export function radialPressDown(_state: RadialPress, wedgeIndex: number): RadialPress {
  return { armed: wedgeIndex };
}

export function radialPressUp(
  state: RadialPress,
  wedgeIndex: number | null,
): { state: RadialPress; commit: number | null; dismiss: boolean } {
  const clear = { armed: null as number | null };
  if (state.armed != null) {
    const commit = wedgeIndex === state.armed ? state.armed : null;
    return { state: clear, commit, dismiss: false };
  }
  if (wedgeIndex == null) return { state: clear, commit: null, dismiss: true };
  return { state: clear, commit: wedgeIndex, dismiss: false };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun run test src/lib/radial.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/src/lib/radial.ts client/src/lib/radial.test.ts
git commit -m "feat(client): add radial pointer-press commit reducer"
```

---

### Task 3: SVG continuous-ring `ActivationRadial`

**Files:**
- Modify: `client/src/components/molecules/activation-radial.tsx`

**Interfaces:**
- Consumes: Task 1 geometry + Task 2 reducer; existing props `{ x, y, zoom, options, onPick, onDismiss, onHoverAction? }`
- Produces: same public props; internal SVG donut; no `Button` chips; no `game-quiet` press scale

- [ ] **Step 1: Replace the component body**

Rewrite `activation-radial.tsx` to approximately:

```tsx
// Activation radial around a selected permanent: continuous SVG donut of legal options.

import { For, createSignal } from "solid-js";
import { Button } from "~/components/atoms";
import { cn } from "~/lib/cn";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  radialOptionKey,
  radialPressDown,
  radialPressUp,
  type RadialOption,
  type RadialPress,
  wedgeLabelPoint,
  wedgePath,
} from "~/lib/radial";
import type { ActionView } from "~/wire/types";

export default function ActivationRadial(props: {
  x: number;
  y: number;
  zoom: number;
  options: RadialOption[];
  onPick: (opt: RadialOption) => void;
  onDismiss: () => void;
  onHoverAction?: (action: ActionView | null) => void;
}) {
  const [press, setPress] = createSignal<RadialPress>({ armed: null });
  const [hover, setHover] = createSignal<number | null>(null);

  const n = () => props.options.length;
  const inner = () => activationRadialInnerRadius(props.zoom);
  const outer = () => activationRadialOuterRadius(props.zoom);
  const size = () => outer() * 2 + 8;
  const origin = () => size() / 2;

  /** Resolve wedge index from the element under the pointer (`data-wedge` on the path’s `<g>`). */
  const wedgeAttr = (el: EventTarget | null): number | null => {
    if (!(el instanceof Element)) return null;
    const node = el.closest("[data-wedge]");
    if (!node) return null;
    const v = node.getAttribute("data-wedge");
    if (v == null) return null;
    const i = Number(v);
    return Number.isFinite(i) ? i : null;
  };

  const applyUp = (wedge: number | null) => {
    const result = radialPressUp(press(), wedge);
    setPress(result.state);
    if (result.dismiss) {
      props.onHoverAction?.(null);
      props.onDismiss();
      return;
    }
    if (result.commit != null) {
      const opt = props.options[result.commit];
      if (!opt) return;
      props.onHoverAction?.(null);
      props.onPick(opt);
    }
  };

  return (
    <div class="pointer-events-none fixed inset-0 z-30">
      <Button
        type="button"
        aria-label="Close"
        variant="ghost"
        hitQuiet
        class="pointer-events-auto absolute inset-0 cursor-default rounded-none border-0 bg-transparent hover:bg-transparent"
        onPointerUp={(e) => {
          e.preventDefault();
          applyUp(null);
        }}
      />
      <svg
        class="pointer-events-none absolute z-[31]"
        width={size()}
        height={size()}
        style={{
          left: `${props.x}px`,
          top: `${props.y}px`,
          transform: "translate(-50%, -50%)",
        }}
      >
        <g transform={`translate(${origin()}, ${origin()})`} class="pointer-events-auto">
          <For each={props.options}>
            {(opt, i) => {
              const d = () => wedgePath(i(), n(), inner(), outer());
              const label = () => wedgeLabelPoint(i(), n(), inner(), outer());
              const active = () => hover() === i() || press().armed === i();
              return (
                <g data-wedge={i()} data-testid={`radial-wedge-${radialOptionKey(opt)}`}>
                  <path
                    d={d()}
                    tabindex="0"
                    role="button"
                    aria-label={opt.label}
                    class={cn(
                      "cursor-pointer outline-none",
                      active()
                        ? "fill-llanowar-deep stroke-priority-gold stroke-2"
                        : "fill-forest-hud stroke-priority-gold/70 stroke-1",
                      "focus-visible:stroke-priority-gold focus-visible:stroke-2",
                    )}
                    onPointerDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
                      setPress(radialPressDown(press(), i()));
                    }}
                    onPointerUp={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      applyUp(wedgeAttr(e.target) ?? i());
                    }}
                    onPointerEnter={() => {
                      setHover(i());
                      props.onHoverAction?.(opt.kind === "action" ? opt.action : null);
                    }}
                    onPointerLeave={() => {
                      setHover((h) => (h === i() ? null : h));
                      props.onHoverAction?.(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key !== "Enter" && e.key !== " ") return;
                      e.preventDefault();
                      props.onHoverAction?.(null);
                      props.onPick(opt);
                    }}
                  />
                  <text
                    x={label().x}
                    y={label().y}
                    text-anchor="middle"
                    dominant-baseline="middle"
                    class="pointer-events-none fill-snow text-[11px] font-semibold"
                  >
                    {opt.label.length > 18 ? `${opt.label.slice(0, 16)}…` : opt.label}
                  </text>
                </g>
              );
            }}
          </For>
        </g>
      </svg>
    </div>
  );
}
```

**Implementer notes:**

1. Hit-test wedges via `data-wedge` + `closest` (as above). Angle/`wedgeIndex` hit-testing is optional fallback only if path events prove flaky.
2. On scrim `pointerup`, if a wedge was armed and the pointer released off-wedge, `applyUp(null)` cancels without dismiss (reducer). Scrim `pointerdown` must not dismiss by itself.
3. Do not use `variant="game-quiet"` (press scale). Use flat SVG fills.
4. If Tailwind `fill-*` / `stroke-*` utilities are unreliable on SVG in this project, use explicit `style={{ fill: "...", stroke: "..." }}` with DESIGN.md hexes (`#0C1412EB`, `#FFD76A`, `#276B3C`). Prefer tokens/classes when they already work on SVG.
5. Truncation at 18 chars is fine for v1; full string stays on `aria-label`.

- [ ] **Step 2: Typecheck**

Run: `cd client && bun run typecheck` (or `just typecheck`)

Expected: PASS (fix any Solid SVG attribute typing issues — use `attr:tabindex` / `attr:role` if the JSX types require it)

- [ ] **Step 3: Run radial unit tests still green**

Run: `cd client && bun run test src/lib/radial.test.ts`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add client/src/components/molecules/activation-radial.tsx
git commit -m "feat(client): render activation radial as continuous SVG pie"
```

---

### Task 4: Board freeze + empty radial

**Files:**
- Modify: `client/src/components/organisms/board.tsx`

**Interfaces:**
- Consumes: `ActivationRadial` props unchanged; `selectedRadial()`, `selectedScreen()`, `camera()`, `selectedId`
- Produces: frozen `{ x, y, zoom }` while a selection with options is open; no hollow ring when `selectedRadial()` is empty

- [ ] **Step 1: Add anchor signal + effects**

Near `selectedId` / `selectedRadial` (around the existing selection effects ~L116–L608):

```tsx
const [radialAnchor, setRadialAnchor] = createSignal<{ x: number; y: number; zoom: number } | null>(
  null,
);

// Freeze screen position when selection opens; clear when it closes.
createEffect((prev: number | null | undefined) => {
  const id = selectedId();
  if (id == null) {
    setRadialAnchor(null);
    return null;
  }
  if (prev !== id) {
    const s = selectedScreen();
    if (s) setRadialAnchor({ x: s.x, y: s.y, zoom: camera().zoom });
  }
  return id;
});

// No hollow pie — drop selection when nothing is legal.
createEffect(() => {
  if (selectedId() == null) return;
  if (selectedRadial().length === 0) setSelectedId(null);
});
```

Replace the render site:

```tsx
<Show when={radialAnchor() && selectedRadial().length > 0 ? radialAnchor() : null}>
  {(pos) => (
    <ActivationRadial
      x={pos().x}
      y={pos().y}
      zoom={pos().zoom}
      options={selectedRadial()}
      onPick={onRadialPick}
      onDismiss={() => {
        setHoverActionId(null);
        setSelectedId(null);
      }}
      onHoverAction={setHoverAction}
    />
  )}
</Show>
```

Remove the old `<Show when={selectedScreen()}>` radial block.

- [ ] **Step 2: Typecheck + focused tests**

Run:

```bash
cd client && bun run typecheck
cd client && bun run test src/lib/radial.test.ts
```

Expected: PASS

- [ ] **Step 3: Manual smoke (if board runnable)**

Select an untapped mana source → continuous ring appears → press-and-release “Tap for mana” once → mana added, radial closes. Press on a wedge, slide off, release → radial stays open. Click outside → dismisses.

- [ ] **Step 4: Commit**

```bash
git add client/src/components/organisms/board.tsx
git commit -m "fix(client): freeze radial anchor and hide empty pies"
```

---

### Task 5: Spec note + verify

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md` (activation radial behavior blurb)
- Optionally touch: `CONTEXT.md` activation-radial glossary line if it still says “pie of legal activates” only — keep wording aligned (“continuous SVG donut; pointer-up commit”)

- [ ] **Step 1: Update board interaction spec**

Find the user story / behavior mentioning the radial (right-click / long-press / activation radial). Add a short note:

```markdown
### Activation radial

Selecting your battlefield permanent opens a **continuous SVG donut** of legal options
(`radialOptions`: tap-for-mana + battlefield activates). Wedges commit on **pointer-up**
on the same wedge (slide-off cancels; outside/hole dismisses). Screen center + zoom are
frozen while open. Empty option lists do not show a hollow ring.
```

If an older bullet still says only “right-click (or long-press)”, leave open behavior as implemented (click-to-select) and don’t reintroduce right-click in this change.

- [ ] **Step 2: Format + lint + client tests**

Run:

```bash
just client-format
just client-lint
just typecheck
just client-test
```

Expected: all green

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md CONTEXT.md
git commit -m "docs: note continuous-ring activation radial behavior"
```

---

## Self-review (author)

| Spec requirement | Task |
|------------------|------|
| Pointer-up commit / slide-off cancel | Task 2 + 3 |
| Continuous ring wedges | Task 1 + 3 |
| Preserve options / play path | Task 3–4 (no session changes) |
| Hover auto_tap preview | Task 3 `onPointerEnter` |
| Freeze position while open | Task 4 |
| Empty → no hollow ring | Task 4 |
| a11y focus + Enter/Space | Task 3 |
| Geometry + press tests | Task 1–2 |
| No engine/wire | Global constraints |

No TBD placeholders. Inner/outer formula locked to corner clearance + `MIN_RING_PX = 36`.
