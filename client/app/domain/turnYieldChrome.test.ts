import { describe, expect, it } from "vitest";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/turnYieldChrome";

describe("turnYieldChrome (Gold Means Act)", () => {
  it("arms with yielded amber, never priority gold", () => {
    expect(turnYieldRockerClass()).toContain("aria-checked:border-yielded/60");
    expect(turnYieldRockerClass()).not.toContain("priority-gold");
    expect(turnYieldTrackClass()).toContain("group-aria-checked:bg-yielded");
    expect(turnYieldTrackClass()).not.toContain("priority-gold");
    expect(turnYieldThumbClass()).toContain("group-aria-checked:text-yielded-ink");
    expect(turnYieldThumbClass()).not.toContain("priority-gold");
  });

  it("rests muted without gold", () => {
    expect(turnYieldRockerClass()).not.toContain("priority-gold");
    expect(turnYieldTrackClass()).toContain("bg-tapped-out");
    expect(turnYieldThumbClass()).not.toContain("priority-gold");
  });

  it("keys visual state off aria-checked so ARIA and chrome cannot drift", () => {
    expect(turnYieldRockerClass()).toContain("group/yield");
    expect(turnYieldThumbClass()).toContain("group-aria-checked:translate-x-[16px]");
    expect(turnYieldThumbClass()).toContain("group-aria-checked:bg-forest-floor");
  });
});
