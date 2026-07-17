import { describe, expect, it } from "vitest";
import { CARD_H } from "~/layout";
import { activationRadialRadius, radialOptions } from "~/lib/radial";
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
