import { describe, expect, it } from "vitest";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/turnYieldChrome";

describe("turnYieldChrome (Gold Means Act)", () => {
  it("arms with yielded amber, never priority gold", () => {
    expect(turnYieldRockerClass()).toContain("aria-checked:border-yielded/60");
    expect(turnYieldRockerClass()).not.toContain("priority-gold");
    expect(turnYieldTrackClass()).toContain("group-aria-checked/yield:bg-yielded");
    expect(turnYieldTrackClass()).not.toContain("priority-gold");
    expect(turnYieldThumbClass()).toContain("group-aria-checked/yield:text-yielded-ink");
    expect(turnYieldThumbClass()).not.toContain("priority-gold");
  });

  it("rests muted without gold", () => {
    expect(turnYieldRockerClass()).not.toContain("priority-gold");
    expect(turnYieldTrackClass()).toContain("bg-tapped-out");
    expect(turnYieldThumbClass()).not.toContain("priority-gold");
  });

  it("keys visual state off aria-checked so ARIA and chrome cannot drift", () => {
    expect(turnYieldRockerClass()).toContain("group/yield");
    expect(turnYieldThumbClass()).toContain("group-aria-checked/yield:translate-x-[16px]");
    expect(turnYieldThumbClass()).toContain("group-aria-checked/yield:bg-forest-floor");
  });

  it("ties every group-aria-checked variant to the rocker's named group", () => {
    // Regression: unnamed group-aria-checked: compiles to `:where(.group)[aria-checked] *`,
    // which never matches a parent carrying the named `group/yield` — the thumb never slid.
    expect(turnYieldRockerClass()).toContain("group/yield");
    for (const cls of [turnYieldRockerClass(), turnYieldTrackClass(), turnYieldThumbClass()]) {
      for (const token of cls.split(" ").filter((t) => t.startsWith("group-aria-checked"))) {
        expect(token).toContain("/yield:");
      }
    }
  });
});
