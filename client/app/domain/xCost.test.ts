import { describe, expect, it } from "vitest";
import { clampX, costText, costWithChosenX } from "./xCost";

describe("clampX", () => {
  it("clamps to max", () => {
    expect(clampX(7, 0, 3)).toBe(3);
  });
  it("clamps to min", () => {
    expect(clampX(-1, 0, 3)).toBe(0);
  });
  it("returns min when max < min", () => {
    expect(clampX(2, 5, 3)).toBe(5);
  });
});

describe("costWithChosenX", () => {
  it("doubles X for Hangarback {X}{X}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 2 };
    expect(costWithChosenX(base, 3)).toEqual({
      generic: 6,
      colored: [0, 0, 0, 0, 0],
      has_x: false,
      x_symbols: 0,
    });
  });
  it("keeps colored pips for {X}{R}", () => {
    const base = { generic: 0, colored: [0, 0, 0, 1, 0], has_x: true, x_symbols: 1 };
    expect(costWithChosenX(base, 4).generic).toBe(4);
    expect(costWithChosenX(base, 4).colored[3]).toBe(1);
  });
  it("defaults x_symbols to 1 when has_x and x_symbols omitted", () => {
    const base = { generic: 2, colored: [0, 0, 0, 0, 0], has_x: true };
    expect(costWithChosenX(base, 3).generic).toBe(5);
  });
});

describe("costText", () => {
  it("formats resolved Hangarback X=11 as {22} without collapsing to {0}", () => {
    const resolved = costWithChosenX({ generic: 0, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 2 }, 11);
    expect(costText(resolved)).toBe("{22}");
  });

  it("keeps colored pips after a large generic", () => {
    const resolved = costWithChosenX({ generic: 1, colored: [0, 0, 0, 1, 0], has_x: true, x_symbols: 1 }, 25);
    expect(costText(resolved)).toBe("{26}{R}");
  });

  it("shows {0} for an empty cost", () => {
    expect(costText({ generic: 0, colored: [0, 0, 0, 0, 0] })).toBe("{0}");
  });

  it("prints hybrid and Phyrexian symbols instead of reading as {0}", () => {
    // Boros Guildmage {R/W}{R/W} — the pay prompt showed "{0}" while the hybrids were dropped.
    const guildmage = { generic: 0, colored: [0, 0, 0, 0, 0], hybrid: [0, 0, 2, 0, 0, 0, 0, 0, 0, 0] };
    expect(costText(guildmage)).toBe("{W/R}{W/R}");

    const vraska = { generic: 4, colored: [0, 0, 2, 0, 0], phyrexian: [0, 0, 1, 0, 0] };
    expect(costText(vraska)).toBe("{4}{B}{B}{B/P}");
  });

  it("carries hybrid pips through an X choice", () => {
    const cost = {
      generic: 0,
      colored: [0, 0, 0, 0, 0],
      has_x: true,
      x_symbols: 1,
      hybrid: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    expect(costText(costWithChosenX(cost, 2))).toBe("{2}{W/U}");
  });
});
