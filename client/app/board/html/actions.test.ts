import { describe, expect, it } from "vitest";
import type { ActionView } from "~/wire/types";
import { autoTapPreviewIds, barZoneAura } from "./actions";

describe("autoTapPreviewIds", () => {
  it("returns an empty set when no action is hovered", () => {
    expect(autoTapPreviewIds(null)).toEqual(new Set());
  });

  it("returns auto_tap object ids from the live action", () => {
    const action = {
      id: 3,
      kind: "cast",
      label: "Cast",
      needs_target: false,
      section: "hand",
      auto_tap: [10, 11],
    } as ActionView;
    expect(autoTapPreviewIds(action)).toEqual(new Set([10, 11]));
  });
});

describe("barZoneAura", () => {
  it("keeps commander gold alone when the command tile is not playable", () => {
    const aura = barZoneAura("command", false);
    expect(aura).toContain("ring-commander-gold");
    expect(aura).not.toContain("ring-playable-border");
    expect(aura).not.toContain("outline-");
  });

  it("layers mint ring with outer commander-gold outline when playable", () => {
    // Zone colour must use outline (not a same-radius box-shadow): ring and shadow share
    // box-shadow, so a 2px gold shadow is fully covered by ring-2 mint.
    const aura = barZoneAura("command", true);
    expect(aura).toContain("ring-playable-border");
    expect(aura).toContain("outline-commander-gold");
    expect(aura).toContain("outline-offset-2");
    expect(aura).not.toMatch(/shadow-\[0_0_0_2px/);
  });

  it("keeps graveyard purple alone when the GY tile is not playable", () => {
    const aura = barZoneAura("graveyard", false);
    expect(aura).toContain("ring-graveyard-outline");
    expect(aura).not.toContain("ring-playable-border");
  });

  it("layers mint ring with outer graveyard outline when playable", () => {
    const aura = barZoneAura("graveyard", true);
    expect(aura).toContain("ring-playable-border");
    expect(aura).toContain("outline-graveyard-outline");
    expect(aura).toContain("outline-offset-2");
    expect(aura).not.toMatch(/shadow-\[0_0_0_2px/);
  });

  it("keeps exile green alone when the exile tile is not playable", () => {
    const aura = barZoneAura("exile", false);
    expect(aura).toContain("ring-exile-outline");
    expect(aura).not.toContain("ring-playable-border");
  });

  it("layers mint ring with outer exile outline when playable", () => {
    const aura = barZoneAura("exile", true);
    expect(aura).toContain("ring-playable-border");
    expect(aura).toContain("outline-exile-outline");
    expect(aura).toContain("outline-offset-2");
    expect(aura).not.toMatch(/shadow-\[0_0_0_2px/);
  });
});
