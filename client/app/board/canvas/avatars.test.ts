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
  it("places life below the circle and keeps hand above", () => {
    const positions = { 0: { x: 100, y: 100 } };
    const shapes = avatarShapes([player()], positions, 0, 1);
    const life = shapes.find((s) => s._tag === "Text" && s.content === "40");
    const hand = shapes.find((s) => s._tag === "Text" && s.content === "Hand 7");

    expect(life?._tag === "Text" ? life.y : null).toBe(148);
    expect(hand?._tag === "Text" ? hand.y : null).toBe(71);
  });

  it("paints monogram when gravatar_hash empty", () => {
    const shapes = avatarShapes([player({ username: "Alice", gravatar_hash: "" })], { 0: { x: 0, y: 0 } }, 0, 1);

    expect(textContents(shapes)).toContain("A");
  });

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
