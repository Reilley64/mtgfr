import { describe, expect, it } from "vitest";
import type { ActionView } from "~/wire/types";
import { playableBattlefieldObjectIds } from "./chrome";

function activate(object: number, over: Partial<ActionView> = {}): ActionView {
  return {
    id: object + 100,
    kind: "activate",
    label: "Activate",
    needs_target: false,
    object,
    section: "battlefield",
    ...over,
  };
}

describe("playableBattlefieldObjectIds", () => {
  it("includes battlefield activates and skips tap-only lands with no action", () => {
    expect(playableBattlefieldObjectIds([activate(7)])).toEqual(new Set([7]));
    expect(playableBattlefieldObjectIds([])).toEqual(new Set());
  });

  it("omits summoning-sick tap activates when the action is marked taps_self", () => {
    const ids = playableBattlefieldObjectIds(
      [activate(7, { taps_self: true }), activate(8, { taps_self: false })],
      [
        { id: 7, summoningSick: true, hasHaste: false },
        { id: 8, summoningSick: true, hasHaste: false },
      ],
    );
    expect(ids.has(7)).toBe(false);
    expect(ids.has(8)).toBe(true);
  });

  it("keeps playable chrome on sick creatures with haste", () => {
    const ids = playableBattlefieldObjectIds([activate(7, { taps_self: true })], [
      { id: 7, summoningSick: true, hasHaste: true },
    ]);
    expect(ids.has(7)).toBe(true);
  });
});
