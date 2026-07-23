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
  });

  it("layers playable border with commander gold when the command tile is playable", () => {
    const aura = barZoneAura("command", true);
    expect(aura).toContain("ring-playable-border");
    expect(aura).toContain("--color-commander-gold");
  });
});
