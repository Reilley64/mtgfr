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

function textContents(shapes: ReturnType<typeof avatarShapes>): string[] {
  return shapes.filter((s) => s._tag === "Text").map((s) => (s._tag === "Text" ? s.content : ""));
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
    const withDmg = avatarShapes([player({ commander_damage: [{ from: 1, amount: 14 }] })], positions, 0, 1);
    const without = avatarShapes([player()], positions, 0, 1);
    expect(textContents(withDmg)).toContain("Cmd 14");
    expect(textContents(without).some((t) => t.startsWith("Cmd "))).toBe(false);
  });
});
