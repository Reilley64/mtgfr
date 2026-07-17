import { describe, expect, it } from "vitest";
import { advance, modeAvailable } from "~/lib/modal";
import type { ModeView } from "~/wire/types";

const damage: ModeView = {
  label: "Deal 2 damage to any target",
  needs_target: true,
  targets: [
    { kind: "object", id: 1 },
    { kind: "player", player: 2 },
  ],
};
const treasure: ModeView = { label: "Create a Treasure", needs_target: false, targets: [] };
const destroy: ModeView = {
  label: "Destroy target artifact",
  needs_target: true,
  targets: [{ kind: "object", id: 5 }],
};
const stranded: ModeView = { label: "Destroy target artifact", needs_target: true, targets: [] };

// Prismari Command's four printed modes, in order.
const MODES = [damage, { label: "Target player draws two", needs_target: true, targets: [] }, treasure, destroy];

describe("advance", () => {
  it("asks for the first chosen mode that wants a target", () => {
    expect(advance(MODES, [0, 2], [])).toEqual({ kind: "ask", index: 0, mode: damage });
  });

  it("auto-answers an untargeted mode instead of asking about it", () => {
    // Choosing Treasure + Destroy: Treasure needs nothing, so the picker jumps straight to Destroy.
    expect(advance(MODES, [2, 3], [])).toEqual({ kind: "ask", index: 3, mode: destroy });
  });

  it("submits once every chosen mode is answered, in the chosen order", () => {
    const answers = [{ index: 0, target: { kind: "player", player: 2 } as const }];
    expect(advance(MODES, [0, 2], answers)).toEqual({
      kind: "submit",
      modes: [
        { index: 0, target: { kind: "player", player: 2 } },
        { index: 2, target: null },
      ],
    });
  });

  it("submits immediately when no chosen mode takes a target", () => {
    expect(advance(MODES, [2], [])).toEqual({ kind: "submit", modes: [{ index: 2, target: null }] });
  });

  it("never submits an empty mode list while modes are still unchosen", () => {
    // The regression that bit: opening the picker must not look like "all zero modes answered".
    // `chosen: []` only ever reaches here as a real (illegal) choice, and the caller gates on count.
    expect(advance(MODES, [], [])).toEqual({ kind: "submit", modes: [] });
  });
});

describe("modeAvailable", () => {
  it("greys out a mode that wants a target and has none", () => {
    expect(modeAvailable(stranded)).toBe(false);
    expect(modeAvailable(damage)).toBe(true);
    expect(modeAvailable(treasure)).toBe(true);
  });
});
