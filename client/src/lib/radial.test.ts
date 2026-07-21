import { describe, expect, it } from "vitest";
import { CARD_H, CARD_W } from "~/layout";
import {
  activationRadialInnerRadius,
  activationRadialOuterRadius,
  activationRadialRadius,
  radialOptionKey,
  radialOptions,
  radialPressDown,
  radialPressUp,
  type RadialPress,
  wedgeIndex,
  wedgeLabelPoint,
  wedgePath,
} from "~/lib/radial";
import type { ActionView } from "~/wire/types";

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

describe("radialOptions", () => {
  it("includes tap-for-mana only when canAct and the permanent can and is untapped", () => {
    expect(radialOptions(7, [], true, false, true)).toEqual([{ kind: "tap_for_mana", label: "Tap for mana" }]);
    expect(radialOptions(7, [], true, false, false)).toEqual([]);
    expect(radialOptions(7, [], true, true, true)).toEqual([]);
  });

  it("lists each battlefield action for that object", () => {
    const actions = [
      activate({ id: 1, label: "Pump" }),
      activate({ id: 2, object: 8, label: "Other" }),
      activate({ id: 3, section: "hand", label: "Cast" }),
    ];
    expect(radialOptions(7, actions, false, false, true)).toEqual([
      { kind: "action", action: actions[0], label: "Pump" },
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
      { kind: "action", action: prepared, label: "Pack a Punch" },
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
      { kind: "action", action: filter, label: "Add {U}{R}" },
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

  it("places the single-wedge label at the top", () => {
    const p = wedgeLabelPoint(0, 1, 50, 90);
    expect(p.x).toBeCloseTo(0, 5);
    expect(p.y).toBeLessThan(0);
  });
});

describe("radialOptionKey", () => {
  it("keys tap-for-mana and actions stably", () => {
    expect(radialOptionKey({ kind: "tap_for_mana", label: "Tap for mana" })).toBe("tap_for_mana");
    expect(
      radialOptionKey({
        kind: "action",
        label: "Pump",
        action: activate({ id: 42 }),
      }),
    ).toBe("action:42");
  });
});

const idle: RadialPress = { armed: null };

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
