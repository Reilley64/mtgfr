import { describe, expect, it } from "vitest";
import {
  ARROW_DRAW_MS,
  STACK_CARD_W,
  STACK_OVERLAY_RIGHT,
  STACK_PEEK,
  arrowDrawProgress,
  stackAimOrigin,
  stagingAimFrom,
  TARGET_COLOR,
} from "~/lib/boardDraw";

describe("stackAimOrigin", () => {
  it("centers a single staged card on the viewport mid-height", () => {
    const o = stackAimOrigin(1000, 800, 1);
    expect(o.x).toBe(1000 - (STACK_OVERLAY_RIGHT + STACK_CARD_W / 2));
    expect(o.y).toBe(400);
  });

  it("raises the top card as the pile grows", () => {
    const one = stackAimOrigin(1000, 800, 1);
    const three = stackAimOrigin(1000, 800, 3);
    expect(three.y).toBeLessThan(one.y);
    expect(one.y - three.y).toBeCloseTo(STACK_PEEK, 5);
  });

  it("honors a compressed peek for the top-card center", () => {
    const full = stackAimOrigin(1000, 400, 8, STACK_PEEK);
    const compressed = stackAimOrigin(1000, 400, 8, 10);
    expect(compressed.y).toBeGreaterThan(full.y);
    expect(full.y - compressed.y).toBeCloseTo(-((8 - 1) * (STACK_PEEK - 10)) / 2, 5);
  });
});

describe("stagingAimFrom", () => {
  it("returns null unless arrow-staging", () => {
    expect(stagingAimFrom(1000, 800, 0, false)).toBeNull();
  });

  it("aims from the staged top (stack length + 1)", () => {
    expect(stagingAimFrom(1000, 800, 2, true)).toEqual(stackAimOrigin(1000, 800, 3));
  });

  it("passes compressed peek through to the aim origin", () => {
    expect(stagingAimFrom(1000, 400, 5, true, 12)).toEqual(stackAimOrigin(1000, 400, 6, 12));
  });

  it("shares the canvas target accent", () => {
    expect(TARGET_COLOR).toBe("#77CCFF");
  });
});

describe("arrowDrawProgress", () => {
  it("starts at the source and finishes after the draw-on window", () => {
    expect(arrowDrawProgress(1000, 1000)).toBe(0);
    expect(arrowDrawProgress(1000, 1000 + ARROW_DRAW_MS / 2)).toBeCloseTo(0.5, 5);
    expect(arrowDrawProgress(1000, 1000 + ARROW_DRAW_MS)).toBe(1);
    expect(arrowDrawProgress(1000, 1000 + ARROW_DRAW_MS + 50)).toBe(1);
  });
});
