# Commander Damage Avatars Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Paint `Cmd N` on life orbs when a seat’s max per-commander damage is > 0.

**Architecture:** Pure `maxCommanderDamage` helper + one extra `Canvas.Text` in `avatarShapes`. Wire already projects `commander_damage`.

**Tech Stack:** Foldkit Canvas, Vitest, TypeScript.

**Spec:** [2026-07-23-commander-damage-avatars-design.md](../specs/2026-07-23-commander-damage-avatars-design.md)

## Global Constraints

- No `.proto` / engine changes.
- Canvas hex `#db8664` for the label; sync battlefield spec (already updated).
- TDD; Angular commits on `cursor/commander-damage-avatars-b23c`.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/app/board/canvas/avatars.ts` | Helper + paint |
| `client/app/board/canvas/avatars.test.ts` | Unit / shape tests |

---

### Task 1: maxCommanderDamage + Cmd paint

**Files:**
- Create: `client/app/board/canvas/avatars.test.ts`
- Modify: `client/app/board/canvas/avatars.ts`

- [ ] **Step 1: Failing tests**

```ts
import { describe, expect, it } from "vitest";
import type { PlayerView } from "~/wire/types";
import { avatarShapes, maxCommanderDamage } from "./avatars";

function player(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...overrides,
  };
}

describe("maxCommanderDamage", () => {
  it("returns 0 when absent or empty", () => {
    expect(maxCommanderDamage(player())).toBe(0);
    expect(maxCommanderDamage(player({ commander_damage: [] }))).toBe(0);
  });
  it("returns the highest single-source amount", () => {
    expect(
      maxCommanderDamage(
        player({
          commander_damage: [
            { from: 1, amount: 7 },
            { from: 2, amount: 14 },
          ],
        }),
      ),
    ).toBe(14);
  });
});

describe("avatarShapes commander damage", () => {
  it("paints Cmd N when damage > 0 and omits it at 0", () => {
    const positions = { 0: { x: 100, y: 100 } };
    const withDmg = avatarShapes(
      [player({ commander_damage: [{ from: 1, amount: 14 }] })],
      positions,
      0,
      1,
    );
    const without = avatarShapes([player()], positions, 0, 1);
    const texts = (shapes: typeof withDmg) =>
      shapes.filter((s) => s._tag === "Text").map((s) => (s._tag === "Text" ? s.content : ""));
    expect(texts(withDmg)).toContain("Cmd 14");
    expect(texts(without).some((t) => t.startsWith("Cmd "))).toBe(false);
  });
});
```

- [ ] **Step 2: Run — expect FAIL (export missing)**

`cd client && bun run test -- app/board/canvas/avatars.test.ts`

- [ ] **Step 3: Implement**

```ts
export function maxCommanderDamage(player: PlayerView): number {
  const rows = player.commander_damage;
  if (rows == null || rows.length === 0) return 0;
  let max = 0;
  for (const row of rows) {
    if (row.amount > max) max = row.amount;
  }
  return max;
}

// inside avatarShapes loop, after username text:
const cmd = maxCommanderDamage(player);
if (cmd > 0) {
  shapes.push(
    Canvas.Text({
      x: pos.x,
      y: pos.y + 42 * zoom,
      content: `Cmd ${cmd}`,
      font: `${Math.max(1, Math.round(12 * zoom))}px system-ui, sans-serif`,
      fill: "#db8664",
      align: "Center",
      baseline: "Middle",
    }),
  );
}
```

- [ ] **Step 4: Run — PASS**

- [ ] **Step 5: Commit**

`feat(client): show max commander damage on life orbs`

---

### Task 2: Verify + ship

- [ ] `bun run test -- app/board/canvas/avatars.test.ts app/board/canvas/scene.test.ts`
- [ ] `bun run typecheck` / lint
- [ ] Push + open PR
