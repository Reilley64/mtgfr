import { describe, expect, it } from "vitest";
import { choiceShowKey, chooseTargetIsCardPick, myChoice } from "~/lib/promptChoice";
import type { PendingChoiceView, VisibleState } from "~/wire/types";

const baseState = {
  active_player: 0,
  can_act: true,
  combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
  objects: [],
  players: [],
  priority: 0,
  stack: [],
  step: 0,
  viewer: 0,
} as VisibleState;

const scry = (player: number): PendingChoiceView =>
  ({ kind: "scry", player, items: [{ id: 1, label: "Forest" }] }) as PendingChoiceView;

const may = (player: number): PendingChoiceView =>
  ({ kind: "may_yes_no", player, source: 1, label: "Draw a card" }) as PendingChoiceView;

describe("myChoice", () => {
  it("returns the pending choice only for the answering seat", () => {
    const state = { ...baseState, pending_choice: scry(0) };
    expect(myChoice(state, 0)?.kind).toBe("scry");
    expect(myChoice(state, 1)).toBeNull();
  });

  it("is null when nothing is pending", () => {
    expect(myChoice({ ...baseState, pending_choice: null }, 0)).toBeNull();
  });
});

describe("choiceShowKey", () => {
  it("is false with no choice for this seat", () => {
    expect(choiceShowKey({ ...baseState, pending_choice: scry(1) }, 0)).toBe(false);
    expect(choiceShowKey({ ...baseState, pending_choice: null }, 0)).toBe(false);
  });

  it("changes when the choice kind changes so a prior form is not reused", () => {
    expect(choiceShowKey({ ...baseState, pending_choice: scry(0) }, 0)).toBe("scry:0");
    expect(choiceShowKey({ ...baseState, pending_choice: may(0) }, 0)).toBe("may_yes_no:0");
  });
});

describe("chooseTargetIsCardPick", () => {
  it("is true for object-only targets (card image picker)", () => {
    expect(chooseTargetIsCardPick([{ id: 4, label: "Bear" }])).toBe(true);
  });

  it("is false for Bojuka Bog-style player seats (need life-orb PickDialog)", () => {
    expect(chooseTargetIsCardPick([{ id: 0, label: "Player 2", player: 1 }])).toBe(false);
  });

  it("is false when any item is a player (mixed any-target lists)", () => {
    expect(
      chooseTargetIsCardPick([
        { id: 4, label: "Bear" },
        { id: 0, label: "Player 1", player: 0 },
      ]),
    ).toBe(false);
  });

  it("is false for an empty list", () => {
    expect(chooseTargetIsCardPick([])).toBe(false);
  });
});
