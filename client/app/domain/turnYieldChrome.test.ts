import { describe, expect, it } from "vitest";
import { turnYieldLabelClass, turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/turnYieldChrome";

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
    for (const tone of ["yield", "end-turn"] as const) {
      for (const cls of [turnYieldRockerClass(tone), turnYieldTrackClass(tone), turnYieldThumbClass(tone)]) {
        for (const token of cls.split(" ").filter((t) => t.startsWith("group-aria-checked"))) {
          expect(token).toContain("/yield:");
        }
      }
    }
  });

  it("arms end turn in island blue, not amber and not priority gold", () => {
    expect(turnYieldRockerClass("end-turn")).toContain("aria-checked:border-island-blue/60");
    expect(turnYieldRockerClass("end-turn")).toContain("aria-checked:shadow-[0_0_12px_rgba(74,158,255,0.45)]");
    expect(turnYieldTrackClass("end-turn")).toContain("group-aria-checked/yield:bg-island-blue");
    for (const cls of [
      turnYieldRockerClass("end-turn"),
      turnYieldTrackClass("end-turn"),
      turnYieldThumbClass("end-turn"),
    ]) {
      expect(cls).not.toContain("priority-gold");
      expect(cls).not.toContain("yielded");
    }
  });

  it("keeps both tones on one silhouette so only the hue differs", () => {
    const strip = (cls: string) => cls.split(" ").filter((t) => !t.includes("island-blue") && !t.includes("yielded"));
    expect(strip(turnYieldTrackClass("end-turn"))).toEqual(strip(turnYieldTrackClass("yield")));
    expect(strip(turnYieldThumbClass("end-turn"))).toEqual(strip(turnYieldThumbClass("yield")));
  });

  it("hides the hover label until hover or keyboard focus opens it", () => {
    expect(turnYieldLabelClass()).toContain("max-w-0");
    expect(turnYieldLabelClass()).toContain("opacity-0");
    expect(turnYieldLabelClass()).toContain("group-hover/yield:max-w-[160px]");
    // Pointer-only reveal would leave the name unreachable by keyboard.
    expect(turnYieldLabelClass()).toContain("group-focus-visible/yield:opacity-100");
  });
});
