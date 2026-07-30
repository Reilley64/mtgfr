import { describe, expect, test } from "vitest";
import { combatCoachText } from "~/combatCoach";

describe("combatCoachText", () => {
  test("coaches declare attackers for the active viewer", () => {
    expect(
      combatCoachText({
        step: 5,
        activePlayer: 0,
        viewer: 0,
        attackersDeclared: false,
        blockersDeclaredForViewer: false,
        attackingViewer: false,
        attackersConfirmedLocally: false,
        blockersConfirmedLocally: false,
      }),
    ).toBe(
      "Drag onto an opponent to attack · Shift drag to send all copies · Click attacker to cancel · Confirm with Attack",
    );
  });

  test("hides attack coach after declaration or local confirm", () => {
    expect(
      combatCoachText({
        step: 5,
        activePlayer: 0,
        viewer: 0,
        attackersDeclared: true,
        blockersDeclaredForViewer: false,
        attackingViewer: false,
        attackersConfirmedLocally: false,
        blockersConfirmedLocally: false,
      }),
    ).toBeNull();
    expect(
      combatCoachText({
        step: 5,
        activePlayer: 0,
        viewer: 0,
        attackersDeclared: false,
        blockersDeclaredForViewer: false,
        attackingViewer: false,
        attackersConfirmedLocally: true,
        blockersConfirmedLocally: false,
      }),
    ).toBeNull();
  });

  test("coaches declare blockers when the viewer is being attacked", () => {
    expect(
      combatCoachText({
        step: 6,
        activePlayer: 1,
        viewer: 0,
        attackersDeclared: true,
        blockersDeclaredForViewer: false,
        attackingViewer: true,
        attackersConfirmedLocally: true,
        blockersConfirmedLocally: false,
      }),
    ).toBe(
      "Drag onto an attacker to block · Shift drag to block with all copies · Click blocker to cancel · Confirm with Block",
    );
  });

  test("hides block coach when not under attack or already declared", () => {
    expect(
      combatCoachText({
        step: 6,
        activePlayer: 1,
        viewer: 0,
        attackersDeclared: true,
        blockersDeclaredForViewer: false,
        attackingViewer: false,
        attackersConfirmedLocally: true,
        blockersConfirmedLocally: false,
      }),
    ).toBeNull();
  });
});
