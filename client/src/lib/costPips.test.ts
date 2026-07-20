import { describe, expect, it } from "vitest";
import { costPipPlate, costPips } from "~/lib/costPips";

describe("costPips", () => {
  it("emits X, then generic, then WUBRG in order", () => {
    expect(costPips({ has_x: true, generic: 2, colored: [1, 0, 1, 0, 0] }).map((p) => p.code)).toEqual([
      "X",
      "2",
      "W",
      "B",
    ]);
  });

  it("expands colored counts into one pip each", () => {
    expect(costPips({ generic: 0, colored: [0, 0, 3, 0, 0] }).map((p) => p.code)).toEqual(["B", "B", "B"]);
  });

  it("returns no pips for an empty cost (lands)", () => {
    expect(costPips({ generic: 0, colored: [0, 0, 0, 0, 0] })).toEqual([]);
  });

  it("shows {0} when showZero is set and the cost is otherwise empty", () => {
    expect(costPips({ generic: 0, colored: [0, 0, 0, 0, 0] }, { showZero: true }).map((p) => p.code)).toEqual(["0"]);
  });

  it("maps codes to mana-font class names", () => {
    expect(costPips({ generic: 1, colored: [0, 1, 0, 0, 0] })).toEqual([
      { ms: "1", code: "1" },
      { ms: "u", code: "U" },
    ]);
  });
});

describe("costPipPlate", () => {
  it("uses coloured disks for WUBRG and generic for numbers/X", () => {
    expect(costPipPlate("W")).toBe("#f0f2c0");
    expect(costPipPlate("U")).toBe("#b5cde3");
    expect(costPipPlate("2")).toBe("#beb9b2");
    expect(costPipPlate("X")).toBe("#beb9b2");
  });
});
