import { describe, expect, it } from "vitest";
import { turnYieldRockerClass, turnYieldThumbClass, turnYieldTrackClass } from "~/lib/turnYieldChrome";

describe("turnYieldChrome (Gold Means Act)", () => {
  it("arms with yielded amber, never priority gold", () => {
    expect(turnYieldRockerClass(true)).toContain("border-yielded/60");
    expect(turnYieldRockerClass(true)).not.toContain("priority-gold");
    expect(turnYieldTrackClass(true)).toContain("bg-yielded");
    expect(turnYieldTrackClass(true)).not.toContain("priority-gold");
    expect(turnYieldThumbClass(true)).toContain("text-yielded-ink");
    expect(turnYieldThumbClass(true)).not.toContain("priority-gold");
  });

  it("rests muted without gold", () => {
    expect(turnYieldRockerClass(false)).not.toContain("priority-gold");
    expect(turnYieldTrackClass(false)).toContain("bg-tapped-out");
    expect(turnYieldThumbClass(false)).not.toContain("priority-gold");
  });
});
