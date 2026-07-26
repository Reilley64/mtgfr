import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView } from "~/wire/types";
import { autoTapPreviewIds, barZoneAura, handTileCaption, modesForObject, orderPlayModes, paymentPreviewAction } from "./actions";

const castAction = {
  id: 3,
  kind: "cast",
  label: testMessageRef("Cast"),
  needs_target: true,
  section: "hand",
  auto_tap: [10, 11],
} as ActionView;

const hoverAction = {
  id: 9,
  kind: "cast",
  label: testMessageRef("Other"),
  needs_target: false,
  section: "hand",
  auto_tap: [20],
} as ActionView;

function board(overrides: Partial<Parameters<typeof paymentPreviewAction>[0]> = {}) {
  return {
    hoverActionId: null as number | null,
    staged: null,
    xPrompt: null,
    modalCast: null,
    sacrificePick: null,
    discardPick: null,
    gyExilePick: null,
    ...overrides,
  };
}

describe("autoTapPreviewIds", () => {
  it("returns an empty set when no action is hovered", () => {
    expect(autoTapPreviewIds(null)).toEqual(new Set());
  });

  it("returns auto_tap object ids from the live action", () => {
    expect(autoTapPreviewIds(castAction)).toEqual(new Set([10, 11]));
  });
});

describe("paymentPreviewAction", () => {
  it("falls back to the hovered action when no session is open", () => {
    expect(paymentPreviewAction(board({ hoverActionId: 9 }), [hoverAction])).toBe(hoverAction);
  });

  it("prefers the staged action over hover so auto_tap survives activate", () => {
    expect(paymentPreviewAction(board({ hoverActionId: 9, staged: { action: castAction } }), [hoverAction])).toBe(
      castAction,
    );
  });

  it("prefers the choose-X prompt action while the stepper is open", () => {
    expect(paymentPreviewAction(board({ xPrompt: { action: castAction } }), [hoverAction])).toBe(castAction);
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

function act(id: number, kind: string, object: number, section = "hand"): ActionView {
  return {
    id,
    kind,
    label: testMessageRef(`${kind}-${id}`),
    needs_target: false,
    object,
    section,
  } as ActionView;
}

describe("modesForObject", () => {
  it("collects only hand-section actions for that object", () => {
    const actions = [
      act(1, "cast", 10),
      act(2, "activate_hand_ability", 10),
      act(3, "cycle", 11),
      act(4, "activate", 10, "battlefield"),
    ];
    expect(modesForObject(actions, 10).map((a) => a.id)).toEqual([1, 2]);
  });
});

describe("orderPlayModes", () => {
  it("orders cast/play_land, then cycle, then hand abilities by action id", () => {
    const modes = [act(30, "activate_hand_ability", 1), act(10, "cycle", 1), act(5, "cast", 1), act(31, "activate_hand_ability", 1)];
    expect(orderPlayModes(modes).map((a) => a.id)).toEqual([5, 10, 30, 31]);
  });
});

describe("handTileCaption", () => {
  it("omits caption when multiple modes are legal", () => {
    expect(handTileCaption([act(1, "cast", 1), act(2, "cycle", 1)])).toBeUndefined();
  });
  it("keeps Cycle/Discard for a sole ability mode", () => {
    expect(handTileCaption([act(1, "cycle", 1)])).toBe("Cycle");
    expect(handTileCaption([act(1, "activate_hand_ability", 1)])).toBe("Discard");
  });
  it("omits caption for sole cast", () => {
    expect(handTileCaption([act(1, "cast", 1)])).toBeUndefined();
  });
});
