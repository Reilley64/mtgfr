/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from "vitest";
import type { ActionView } from "~/wire/types";
import {
  ACTIVATION_MENU_GAP_PX,
  ACTIVATION_MENU_MAX_HEIGHT_PX,
  ACTIVATION_MENU_WIDTH_PX,
  activationCostChip,
  activationMenuEstimatedHeight,
  activationMenuPlacement,
  type RadialPress,
  radialOptionKey,
  radialOptions,
  radialPressDown,
  radialPressUp,
  radialScreenCenter,
  radialWedgeAtPoint,
  radialWedgeFromElement,
} from "./radial";

const activate = (over: Partial<ActionView> = {}): ActionView =>
  ({
    ability_index: 0,
    id: 1,
    kind: "activate",
    label: "Draw a card",
    needs_target: false,
    object: 7,
    section: "battlefield",
    targets: [],
    ...over,
  }) as unknown as ActionView;

describe("radialScreenCenter", () => {
  it("maps the selected card center from world to screen coordinates", () => {
    const camera = { panX: 5, panY: -13, zoom: 2 };
    const card = { x: 10, y: 20, w: 120, h: 80 };

    expect(radialScreenCenter(camera, card)).toEqual({ x: 145, y: 107 });
  });
});

describe("radialOptions", () => {
  it("always includes tap-for-mana for mana sources and disables it when unusable", () => {
    expect(radialOptions(7, [], true, false, true)).toEqual([
      { kind: "tap_for_mana", label: "Tap for mana", disabled: false },
    ]);
    expect(radialOptions(7, [], true, false, false)).toEqual([
      { kind: "tap_for_mana", label: "Tap for mana", disabled: true },
    ]);
    expect(radialOptions(7, [], true, true, true)).toEqual([
      { kind: "tap_for_mana", label: "Tap for mana", disabled: true },
    ]);
    expect(radialOptions(7, [], true, false, true, true, false)).toEqual([
      { kind: "tap_for_mana", label: "Tap for mana", disabled: true },
    ]);
  });

  it("lists each battlefield action for that object", () => {
    const actions = [
      activate({ id: 1, label: "Pump" }),
      activate({ id: 2, object: 8, label: "Other" }),
      activate({ id: 3, section: "hand", label: "Cast" }),
    ];
    expect(radialOptions(7, actions, false, false, true)).toEqual([
      { kind: "action", action: actions[0], label: "Pump", disabled: false },
    ]);
  });

  it("lists cast_prepared battlefield actions", () => {
    const prepared = activate({
      id: 9,
      kind: "cast_prepared",
      label: "Pack a Punch",
      needs_target: true,
      targets: [{ kind: "object", id: 3 }],
    });
    expect(radialOptions(7, [prepared], false, false, true)).toEqual([
      { kind: "action", action: prepared, label: "Pack a Punch", disabled: false },
    ]);
  });

  it("combines tap-for-mana with activates", () => {
    const a = activate();
    expect(radialOptions(7, [a], true, false, true)).toHaveLength(2);
  });

  it("shows a paid mana activate when the permanent does not tapsForMana", () => {
    // Filter lands like Ferrous Lake: no free tap, but a {{1}},{{T}} activate on the wire.
    const filter = activate({
      id: 4,
      label: "Add {U}{R}",
    });
    expect(radialOptions(7, [filter], false, false, true)).toEqual([
      { kind: "action", action: filter, label: "Add {U}{R}", disabled: false },
    ]);
  });
});

describe("activationCostChip", () => {
  it("shows tap for tap_for_mana", () => {
    expect(activationCostChip({ kind: "tap_for_mana", label: "Tap for mana", disabled: false })).toEqual({
      kind: "tap",
    });
  });

  it("shows tap when action.taps_self is true", () => {
    expect(
      activationCostChip({
        kind: "action",
        label: "Scry 1",
        disabled: false,
        action: activate({ taps_self: true }),
      }),
    ).toEqual({ kind: "tap" });
  });

  it("shows mana from x_cost when present", () => {
    const cost = { generic: 1, colored: [0, 0, 0, 0, 0], has_x: true, x_symbols: 1 };
    expect(
      activationCostChip({
        kind: "action",
        label: "X pump",
        disabled: false,
        action: activate({ has_x: true, x_cost: cost }),
      }),
    ).toEqual({ kind: "mana", cost });
  });

  it("combines tap and mana when both apply", () => {
    const cost = { generic: 0, colored: [0, 1, 0, 0, 0], has_x: false };
    expect(
      activationCostChip({
        kind: "action",
        label: "Pay U, tap",
        disabled: false,
        action: activate({ taps_self: true, x_cost: cost }),
      }),
    ).toEqual({ kind: "tap_and_mana", cost });
  });

  it("returns null when no structured cost exists", () => {
    expect(
      activationCostChip({
        kind: "action",
        label: "Add {U}{R}",
        disabled: false,
        action: activate({ label: "Add {U}{R}" }),
      }),
    ).toBeNull();
  });
});

describe("radialOptionKey", () => {
  it("keys tap-for-mana and actions stably", () => {
    expect(radialOptionKey({ kind: "tap_for_mana", label: "Tap for mana", disabled: false })).toBe("tap_for_mana");
    expect(
      radialOptionKey({
        kind: "action",
        label: "Pump",
        disabled: false,
        action: activate({ id: 42 }),
      }),
    ).toBe("action:42");
  });
});

const idle: RadialPress = { armed: null };

describe("radialWedgeFromElement / radialWedgeAtPoint", () => {
  it("returns the wedge index from a data-wedge element", () => {
    const el = document.createElement("g");
    el.setAttribute("data-wedge", "2");
    expect(radialWedgeFromElement(el)).toBe(2);
  });

  it("returns null for null or non-wedge elements", () => {
    expect(radialWedgeFromElement(null)).toBeNull();
    expect(radialWedgeFromElement(document.createElement("div"))).toBeNull();
  });

  it("resolves wedge at point via elementFromPoint", () => {
    const wedge = document.createElement("g");
    wedge.setAttribute("data-wedge", "2");
    const fromPoint = (_x: number, _y: number) => wedge;
    expect(radialWedgeAtPoint(10, 20, fromPoint)).toBe(2);
    expect(radialWedgeAtPoint(10, 20, () => null)).toBeNull();
  });
});

describe("activationMenuEstimatedHeight", () => {
  it("grows with option count and caps at max height", () => {
    expect(activationMenuEstimatedHeight(1)).toBeLessThan(activationMenuEstimatedHeight(4));
    expect(activationMenuEstimatedHeight(50)).toBe(ACTIVATION_MENU_MAX_HEIGHT_PX);
  });
});

describe("activationMenuPlacement", () => {
  const menu = { width: ACTIVATION_MENU_WIDTH_PX, height: 120 };
  const card = { w: 96, h: 134 };
  const vp = { width: 1440, height: 900 };

  it("prefers the right of the card when there is room", () => {
    const center = { x: 400, y: 450 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    const expected = center.x + card.w / 2 + ACTIVATION_MENU_GAP_PX;
    expect(leftPx).toBeCloseTo(expected, 1);
    expect(place.width).toBe(`${(menu.width / vp.width) * 100}%`);
  });

  it("flips to the left when the right side overflows", () => {
    const center = { x: 1400, y: 450 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    expect(leftPx + menu.width).toBeLessThanOrEqual(vp.width + 0.5);
    expect(leftPx).toBeLessThan(center.x);
  });

  it("flips above when horizontal sides overflow", () => {
    // Narrow viewport: menu cannot fit left or right of a centered card.
    const narrow = { width: 300, height: 900 };
    const center = { x: 150, y: 450 };
    const wideMenu = { width: 240, height: 80 };
    const place = activationMenuPlacement(center, card, wideMenu, narrow);
    const topPx = (Number.parseFloat(place.top) / 100) * narrow.height;
    expect(topPx + wideMenu.height).toBeLessThanOrEqual(center.y - card.h / 2 + 0.5);
  });

  it("clamps so the panel stays fully on-screen", () => {
    const center = { x: 10, y: 10 };
    const place = activationMenuPlacement(center, card, menu, vp);
    const leftPx = (Number.parseFloat(place.left) / 100) * vp.width;
    const topPx = (Number.parseFloat(place.top) / 100) * vp.height;
    expect(leftPx).toBeGreaterThanOrEqual(0);
    expect(topPx).toBeGreaterThanOrEqual(0);
    expect(leftPx + menu.width).toBeLessThanOrEqual(vp.width + 0.5);
    expect(topPx + menu.height).toBeLessThanOrEqual(vp.height + 0.5);
  });

  it("returns zero box when viewport is invalid", () => {
    expect(activationMenuPlacement({ x: 1, y: 1 }, card, menu, { width: 0, height: 0 })).toEqual({
      left: "0%",
      top: "0%",
      width: "0%",
      maxHeight: "0%",
    });
  });
});

describe("radialPress", () => {
  it("commits when down and up on the same wedge", () => {
    const armed = radialPressDown(idle, 2);
    expect(armed).toEqual({ armed: 2 });
    const up = radialPressUp(armed, 2);
    expect(up.commit).toBe(2);
    expect(up.dismiss).toBe(false);
    expect(up.state.armed).toBeNull();
  });

  it("cancels when sliding off before release", () => {
    const armed = radialPressDown(idle, 1);
    const up = radialPressUp(armed, null);
    expect(up.commit).toBeNull();
    expect(up.dismiss).toBe(false);
    expect(up.state.armed).toBeNull();
  });

  it("dismisses on scrim up when nothing was armed", () => {
    const up = radialPressUp(idle, null);
    expect(up.commit).toBeNull();
    expect(up.dismiss).toBe(true);
  });

  it("commits an idle up on a wedge (no prior down)", () => {
    const up = radialPressUp(idle, 0);
    expect(up.commit).toBe(0);
    expect(up.dismiss).toBe(false);
  });
});
