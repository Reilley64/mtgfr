/**
 * @vitest-environment happy-dom
 */
import { describe, expect, it } from "vitest";
import type { ActionView } from "~/wire/types";
import { CARD_H, CARD_W } from "./layout";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  activationRadialRadius,
  type RadialPress,
  radialOptionKey,
  radialOptions,
  radialPressDown,
  radialPressUp,
  radialScreenCenter,
  radialWedgeAtPoint,
  radialWedgeFromElement,
  wedgeIndex,
  wedgeLabelPoint,
  wedgePath,
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

describe("activationRadialRadius", () => {
  it("scales with zoom so the ring tracks the on-screen card", () => {
    expect(activationRadialRadius(1)).toBe(CARD_H / 2 + 12);
    expect(activationRadialRadius(2)).toBe(CARD_H + 12);
    expect(activationRadialRadius(0.1)).toBe(40);
  });
});

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

describe("activationRadialInnerRadius / outer", () => {
  it("clears the upright card corners and keeps a usable ring thickness", () => {
    const zoom = 1;
    const inner = activationRadialInnerRadius(zoom);
    const outer = activationRadialOuterRadius(zoom);
    const corner = Math.hypot(CARD_W / 2, CARD_H / 2) * zoom;
    expect(inner).toBeGreaterThan(corner);
    expect(outer - inner).toBeGreaterThanOrEqual(36);
    expect(outer).toBeGreaterThanOrEqual(activationRadialRadius(zoom));
  });

  it("scales with zoom", () => {
    expect(activationRadialInnerRadius(2)).toBeGreaterThan(activationRadialInnerRadius(1));
    expect(activationRadialOuterRadius(2)).toBeGreaterThan(activationRadialOuterRadius(1));
  });
});

describe("wedgeIndex", () => {
  it("puts the top of the ring in wedge 0 when count is 4", () => {
    // atan2(-1, 0) === -π/2 — straight up from center
    expect(wedgeIndex(-Math.PI / 2, 4)).toBe(0);
  });

  it("wraps angles into [0, count)", () => {
    expect(wedgeIndex(Math.PI, 4)).toBeGreaterThanOrEqual(0);
    expect(wedgeIndex(Math.PI, 4)).toBeLessThan(4);
  });

  it("returns 0 for a single wedge at any angle", () => {
    expect(wedgeIndex(0, 1)).toBe(0);
    expect(wedgeIndex(2, 1)).toBe(0);
  });
});

describe("wedgePath / wedgeLabelPoint", () => {
  it("returns a non-empty path for each of 6 wedges", () => {
    for (let i = 0; i < 6; i++) {
      expect(wedgePath(i, 6, 50, 90).length).toBeGreaterThan(10);
    }
  });

  it("draws a full donut with two outer semicircles when count is 1", () => {
    // SVG cannot express a 360° arc in one A command (start==end collapses).
    const d = wedgePath(0, 1, 50, 90);
    const outerArcs = d.match(/A 90 90/g) ?? [];
    expect(outerArcs.length).toBe(2);
    expect(d).toContain("A 50 50");
    // evenodd hole: outer closed, then inner closed (no radial L seam through the label)
    expect(d.indexOf("Z")).toBeLessThan(d.lastIndexOf("Z"));
    expect(d).not.toMatch(/L /);
  });

  it("places the single-wedge label at the top", () => {
    const p = wedgeLabelPoint(0, 1, 50, 90);
    expect(p.x).toBeCloseTo(0, 5);
    expect(p.y).toBeLessThan(0);
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
