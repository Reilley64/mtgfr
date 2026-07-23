import { describe, expect, it } from "vitest";
import {
  combatStagingClearsOnStepChange,
  handleCombatDrop,
  mergeRequiredAttacks,
  primaryActionIntent,
  stagedAttackersForDisplay,
} from "./combat-staging";
import type { RenderCard } from "./layout";
import { ZONE } from "./layout";

const creature = (id: number, over: Partial<RenderCard> = {}): RenderCard =>
  ({
    id,
    tapped: false,
    summoningSick: false,
    hasHaste: false,
    kind: "creature",
    ...over,
  }) as RenderCard;

describe("handleCombatDrop", () => {
  it("stages an attacker onto a defender seat", () => {
    const result = handleCombatDrop("attackers", [], [], creature(3), 1, null, [], 0);
    expect(result).toEqual({ kind: "attackers", value: [{ attacker: 3, defender: 1 }] });
  });

  it("retargets an already-staged attacker", () => {
    const result = handleCombatDrop("attackers", [{ attacker: 3, defender: 1 }], [], creature(3), 2, null, [], 0);
    expect(result).toEqual({ kind: "attackers", value: [{ attacker: 3, defender: 2 }] });
  });

  it("stages a blocker onto an attacker aimed at me", () => {
    const declared = [{ attacker: 9, defender: 0 }];
    const target = creature(9, { zone: ZONE.Battlefield, controller: 1 });
    const result = handleCombatDrop("blockers", [], [], creature(4), null, target, declared, 0);
    expect(result).toEqual({ kind: "blockers", value: [{ blocker: 4, attacker: 9 }] });
  });

  it("returns none outside a combat mode", () => {
    expect(handleCombatDrop(null, [], [], creature(3), 1, null, [], 0)).toEqual({ kind: "none" });
  });
});

describe("primaryActionIntent", () => {
  it("confirms staged attackers", () => {
    const attackers = [{ attacker: 3, defender: 1 }];
    expect(primaryActionIntent({ kind: "confirm-attackers", label: "Attack (1)" }, 0, attackers, [])).toEqual({
      kind: "declare_attackers",
      player: 0,
      attackers,
    });
  });

  it("confirms an empty attack declaration", () => {
    expect(primaryActionIntent({ kind: "confirm-attackers", label: "No attackers" }, 0, [], [])).toEqual({
      kind: "declare_attackers",
      player: 0,
      attackers: [],
    });
  });

  it("confirms an empty block declaration", () => {
    expect(primaryActionIntent({ kind: "confirm-blockers", label: "No blockers" }, 1, [], [])).toEqual({
      kind: "declare_blockers",
      player: 1,
      blocks: [],
    });
  });

  it("passes priority on Next", () => {
    expect(primaryActionIntent({ kind: "pass", label: "Next" }, 2, [], [])).toEqual({
      kind: "pass_priority",
      player: 2,
    });
  });
});

describe("mergeRequiredAttacks", () => {
  it("appends missing required attackers without replacing staged ones", () => {
    expect(
      mergeRequiredAttacks(
        [{ attacker: 1, defender: 2 }],
        [
          { attacker: 1, defender: 3 },
          { attacker: 4, defender: 2 },
        ],
      ),
    ).toEqual([
      { attacker: 1, defender: 2 },
      { attacker: 4, defender: 2 },
    ]);
  });

  it("fills an empty stage from required goad attacks", () => {
    const required = [{ attacker: 7, defender: 1 }];
    expect(mergeRequiredAttacks([], required)).toEqual(required);
  });

  it("keeps the player's defender choice when a required attacker is already staged", () => {
    const staged = [{ attacker: 7, defender: 2 }];
    const required = [{ attacker: 7, defender: 1 }];
    expect(mergeRequiredAttacks(staged, required)).toEqual(staged);
  });
});

describe("stagedAttackersForDisplay", () => {
  const required = [{ attacker: 7, defender: 1 }];

  it("merges required attacks while declaration is still open", () => {
    expect(stagedAttackersForDisplay([], required, false)).toEqual(required);
  });

  it("does not re-merge required attacks after declaration is done (SSE lag)", () => {
    // Confirm clears local staging; required_attacks can linger on the old action until SSE.
    expect(stagedAttackersForDisplay([], required, true)).toEqual([]);
  });
});

describe("combatStagingClearsOnStepChange", () => {
  it("does not clear on the first observation or same-step SSE churn", () => {
    expect(combatStagingClearsOnStepChange(undefined, 5)).toBe(false);
    expect(combatStagingClearsOnStepChange(5, 5)).toBe(false);
  });

  it("clears only when the step value actually changes", () => {
    expect(combatStagingClearsOnStepChange(5, 6)).toBe(true);
    expect(combatStagingClearsOnStepChange(6, 5)).toBe(true);
  });
});
