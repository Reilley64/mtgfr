// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { markRevealSeen, revealSeen, revealSlot, spotlightSteps } from "./first-player-reveal";

describe("spotlightSteps", () => {
  it("ends on the winner", () => {
    const steps = spotlightSteps(2, 4, false);
    expect(steps.at(-1)?.slot).toBe(2);
  });

  it("hops every seat in screen order and decelerates", () => {
    const steps = spotlightSteps(1, 4, false);
    expect(steps.map((s) => s.slot).slice(0, 5)).toEqual([0, 1, 2, 3, 0]);
    const gaps = steps.slice(1).map((s) => s.delayMs);
    expect(gaps.at(-1)).toBeGreaterThan(gaps[0] ?? 0);
    expect(steps[0]?.delayMs).toBe(0);
  });

  it("skips the hop under reduced motion", () => {
    expect(spotlightSteps(3, 4, true)).toEqual([{ slot: 3, delayMs: 0 }]);
  });

  it("survives a one-seat table", () => {
    expect(spotlightSteps(0, 1, false).at(-1)?.slot).toBe(0);
  });
});

describe("revealSlot", () => {
  it("is viewer-relative", () => {
    expect(revealSlot(2, 2, 4)).toBe(0);
    expect(revealSlot(3, 2, 4)).toBe(1);
  });

  it("falls back to seat order for a spectator", () => {
    expect(revealSlot(2, 255, 4)).toBe(2);
  });

  it("clamps count to avoid NaN", () => {
    expect(revealSlot(0, 0, 0)).toBe(0);
    expect(revealSlot(1, 0, 0)).toBe(0);
  });
});

describe("one-shot storage", () => {
  it("remembers a table it has shown", () => {
    expect(revealSeen("t-1")).toBe(false);
    markRevealSeen("t-1");
    expect(revealSeen("t-1")).toBe(true);
    expect(revealSeen("t-2")).toBe(false);
  });

  it("treats a throwing sessionStorage as not-yet-seen", () => {
    const setItem = sessionStorage.setItem;
    sessionStorage.setItem = () => {
      throw new Error("denied");
    };
    expect(() => markRevealSeen("t-3")).not.toThrow();
    sessionStorage.setItem = setItem;
  });
});
