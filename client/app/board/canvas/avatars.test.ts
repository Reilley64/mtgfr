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
    const cmdText = withDmg.find((s) => s._tag === "Text" && s.content === "Cmd 14");
    expect(cmdText?._tag === "Text" ? cmdText.fill : null).toBe("#db8664");
  });
});

describe("avatarShapes poison and rad clocks", () => {
  const positions = { 0: { x: 100, y: 100 } };

  it("omits both when the player has neither", () => {
    const texts = textContents(avatarShapes([player()], positions, 0, 1));
    expect(texts.some((t) => t.startsWith("Poison ") || t.startsWith("Rad "))).toBe(false);
  });

  it("paints the poison and rad totals", () => {
    const texts = textContents(avatarShapes([player({ poison: 3, rad: 2 })], positions, 0, 1));
    expect(texts).toContain("Poison 3");
    expect(texts).toContain("Rad 2");
  });

  // CR 704.5c — the last two poison counters before elimination read as a warning.
  it("turns the poison chip red inside lethal range", () => {
    const fillAt = (poison: number) => {
      const shapes = avatarShapes([player({ poison })], positions, 0, 1);
      const chip = shapes.find((s) => s._tag === "Text" && s.content === `Poison ${poison}`);
      return chip?._tag === "Text" ? chip.fill : null;
    };
    expect(fillAt(7)).toBe("#8fd14f");
    expect(fillAt(9)).toBe("#e0574f");
  });

  // Cmd, Poison, and Rad stack rather than overprint each other.
  it("stacks the chips on distinct rows", () => {
    const shapes = avatarShapes(
      [player({ commander_damage: [{ from: 1, amount: 14 }], poison: 4, rad: 1 })],
      positions,
      0,
      1,
    );
    const ys = shapes
      .filter((s) => s._tag === "Text" && /^(Cmd|Poison|Rad) /.test(s.content))
      .map((s) => (s._tag === "Text" ? s.y : 0));
    expect(ys).toEqual([142, 156, 170]);
  });
});
