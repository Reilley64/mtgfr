import { describe, expect, it } from "vitest";
import { emptyManaPool, manaPipChips, manaTrayChips } from "~/lib/manaPips";

describe("manaPipChips", () => {
  it("hides zero rows", () => {
    expect(manaPipChips(emptyManaPool())).toEqual([]);
  });

  it("lists WUBRG, C, any, either, and of_colors", () => {
    expect(
      manaPipChips({
        colored: [1, 0, 2, 0, 0],
        colorless: 3,
        any: 1,
        either: [{ a: 0, b: 1, amount: 1 }],
        of_colors: [{ mask: 0b00110, amount: 2 }],
      }),
    ).toEqual([
      { symbol: "W", amount: 1 },
      { symbol: "B", amount: 2 },
      { symbol: "C", amount: 3 },
      { symbol: "★", amount: 1 },
      { symbol: "W/U", amount: 1 },
      { symbol: "UB", amount: 2 },
    ]);
  });
});

describe("manaTrayChips", () => {
  it("hides an empty pool", () => {
    expect(manaTrayChips(emptyManaPool())).toEqual([]);
  });

  it("uses cost pips, split hybrids, multicolor duo for any, and color indicators for of_colors", () => {
    expect(
      manaTrayChips({
        colored: [1, 0, 2, 0, 0],
        colorless: 3,
        any: 1,
        either: [{ a: 0, b: 1, amount: 1 }],
        of_colors: [{ mask: 0b00110, amount: 2 }],
      }),
    ).toEqual([
      { kind: "glyph", ms: "w", code: "W", amount: 1 },
      { kind: "glyph", ms: "b", code: "B", amount: 2 },
      { kind: "glyph", ms: "c", code: "C", amount: 3 },
      { kind: "any", amount: 1 },
      { kind: "glyph", ms: "wu", code: "W/U", amount: 1 },
      { kind: "ci", n: 2, suffix: "ub", code: "UB", amount: 2 },
    ]);
  });

  it("keeps colorless credits as {C} at any amount (not a generic number pip)", () => {
    expect(manaTrayChips({ ...emptyManaPool(), colorless: 1 })).toEqual([
      { kind: "glyph", ms: "c", code: "C", amount: 1 },
    ]);
    expect(manaTrayChips({ ...emptyManaPool(), colorless: 3 })).toEqual([
      { kind: "glyph", ms: "c", code: "C", amount: 3 },
    ]);
  });

  it("maps five-color of_colors to the ci-5 / wubrg indicator", () => {
    expect(manaTrayChips({ ...emptyManaPool(), of_colors: [{ mask: 0b11111, amount: 1 }] })).toEqual([
      { kind: "ci", n: 5, suffix: "wubrg", code: "WUBRG", amount: 1 },
    ]);
  });

  it("maps wire-order either pairs (W/R) to mana-font split glyphs", () => {
    expect(manaTrayChips({ ...emptyManaPool(), either: [{ a: 0, b: 3, amount: 2 }] })).toEqual([
      { kind: "glyph", ms: "rw", code: "W/R", amount: 2 },
    ]);
  });
});
