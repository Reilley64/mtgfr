import { describe, expect, it } from "vitest";
import {
  buildTakeActionIntent,
  emptyCostPicks,
  findCastActionForObject,
  planCastClickResolution,
  planCostPipeline,
  planHandDrop,
  planRunAction,
  settleSacrificePick,
  stagedCastSubmission,
  usedCostPick,
} from "~/controllers/actionExecution";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";

const mkAction = (over: Partial<ActionView> = {}): ActionView => ({
  id: 1,
  kind: "cast",
  label: "Bolt",
  needs_target: false,
  section: "hand",
  ...over,
});

const card = (id: number): ObjectView =>
  ({
    id,
    name: `Card ${id}`,
    mana_cost: { has_x: false },
  }) as ObjectView;

describe("buildTakeActionIntent", () => {
  it("builds a take_action wire intent with cost picks", () => {
    expect(
      buildTakeActionIntent(0, 7, { kind: "object", id: 3 }, 2, [], {
        ...emptyCostPicks(),
        sacrifice: 9,
        discard_cost: [4],
        graveyard_exile: [5, 6],
      }),
    ).toEqual({
      kind: "take_action",
      player: 0,
      id: 7,
      target: { kind: "object", id: 3 },
      x: 2,
      modes: [],
      sacrifice: [9],
      discard_cost: [4],
      graveyard_exile: [5, 6],
    });
  });
});

describe("usedCostPick", () => {
  it("is true after escape exile is settled", () => {
    expect(usedCostPick({ ...emptyCostPicks(), gy_exile_settled: true, graveyard_exile: [1, 2] })).toBe(true);
  });

  it("is false for a fresh cast with no cost prompts", () => {
    expect(usedCostPick(emptyCostPicks())).toBe(false);
  });

  it("is false when free delve auto-settles with no cards exiled", () => {
    expect(usedCostPick({ ...emptyCostPicks(), gy_exile_settled: true, graveyard_exile: [] })).toBe(false);
  });

  it("is true after discard is settled", () => {
    expect(usedCostPick({ ...emptyCostPicks(), discard_settled: true, discard_cost: [3] })).toBe(true);
  });

  it("is false after sacrifice alone so the activate can stage onto the stack and aim", () => {
    // Dina: sacrifice Zulaport → ability ghost on stack → arrow-target a creature → priority.
    expect(usedCostPick({ ...emptyCostPicks(), sacrifice: 3 })).toBe(false);
  });
});

describe("settleSacrificePick", () => {
  it("captures action/card/picks before the Show signal is cleared", () => {
    // Board must call this *before* setSacrificePick(null); otherwise Solid's sp() is falsy.
    const action = mkAction({
      kind: "activate",
      object: 1,
      sacrifice_choices: [3],
      needs_target: true,
      targets: [{ kind: "object", id: 1 }],
    });
    const settled = settleSacrificePick(
      { action, card: card(1), dropSeed: { x: 0, y: 0 }, picks: emptyCostPicks() },
      3,
    );
    expect(settled.action).toBe(action);
    expect(settled.card?.id).toBe(1);
    expect(settled.picks.sacrifice).toBe(3);
    expect(planCostPipeline(settled.action, settled.card, settled.picks).kind).toBe("run");
  });
});

describe("planCastClickResolution", () => {
  it("routes a staged target click through completeTarget (not bare takeCastAction)", () => {
    const target = { kind: "object" as const, id: 9 };
    expect(planCastClickResolution(true, { card: card(1), target })).toEqual({
      kind: "complete-staged-target",
      target,
    });
  });

  it("routes commander clicks when nothing is staged", () => {
    const c = card(50);
    expect(planCastClickResolution(false, { card: c, target: null })).toEqual({
      kind: "cast-commander",
      card: c,
      target: null,
    });
  });

  it("ignores staged clicks with no target", () => {
    expect(planCastClickResolution(true, { card: card(1), target: null })).toEqual({ kind: "ignore" });
  });
});

describe("stagedCastSubmission", () => {
  it("keeps escape exile picks when the player names a target", () => {
    const action = mkAction({ id: 42, needs_target: true, label: "Sentinel's Eyes" });
    const picks = { ...emptyCostPicks(), graveyard_exile: [8, 9], gy_exile_settled: true };
    const sub = stagedCastSubmission({ action, picks }, { kind: "object", id: 3 });
    const intent = buildTakeActionIntent(0, sub.action.id, sub.target, 0, [], sub.picks);
    expect(intent.kind).toBe("take_action");
    if (intent.kind !== "take_action") return;
    expect(intent.graveyard_exile).toEqual([8, 9]);
  });

  it("is what Board must use after planCastClickResolution returns complete-staged-target", () => {
    const action = mkAction({ id: 42, needs_target: true });
    const picks = { ...emptyCostPicks(), graveyard_exile: [8, 9], gy_exile_settled: true };
    const click = planCastClickResolution(true, {
      card: card(77),
      target: { kind: "object", id: 3 },
    });
    expect(click.kind).toBe("complete-staged-target");
    if (click.kind !== "complete-staged-target") return;
    const intent = buildTakeActionIntent(
      0,
      action.id,
      stagedCastSubmission({ action, picks }, click.target).target,
      0,
      [],
      picks,
    );
    if (intent.kind !== "take_action") return;
    expect(intent.graveyard_exile).toEqual([8, 9]);
    expect(intent.target).toEqual({ kind: "object", id: 3 });
  });
});

describe("findCastActionForObject", () => {
  it("finds the cast action for a commander object", () => {
    const actions = [mkAction({ id: 9, object: 50, section: "command" }), mkAction({ id: 2, object: 3 })];
    expect(findCastActionForObject(actions, 50)?.id).toBe(9);
  });

  it("returns undefined when no cast action exists", () => {
    expect(findCastActionForObject([mkAction({ kind: "play_land", object: 1 })], 1)).toBeUndefined();
  });
});

describe("planHandDrop", () => {
  it("ignores a release back in the hand bar", () => {
    expect(planHandDrop(mkAction(), card(1), 900, 800)).toEqual({ kind: "ignore" });
  });

  it("opens a modal picker for modal spells", () => {
    const action = mkAction({ modal: { choose: 1, choose_max: 1, modes: [] } });
    expect(planHandDrop(action, card(1), 100, 800)).toEqual({
      kind: "modal",
      action,
      modes: [],
      picks: emptyCostPicks(),
    });
  });

  it("rejects sacrifice costs with no legal payers", () => {
    const action = mkAction({ sacrifice_choices: [] });
    expect(planHandDrop(action, card(1), 100, 800)).toEqual({ kind: "reject", reason: "CannotActivate" });
  });

  it("parks on sacrifice pick when choices exist", () => {
    const action = mkAction({ sacrifice_choices: [3, 4] });
    const c = card(1);
    expect(planHandDrop(action, c, 100, 800)).toEqual({
      kind: "sacrifice-pick",
      action,
      card: c,
      picks: emptyCostPicks(),
    });
  });

  it("after naming the sacrifice, a targeted activate stages for stack+arrow aiming", () => {
    // Dina: sacrifice prompt → stage onto stack → aim creature (not a second full-screen picker).
    const action = mkAction({
      kind: "activate",
      object: 1,
      needs_target: true,
      targets: [
        { kind: "object", id: 1 },
        { kind: "object", id: 3 },
      ],
      sacrifice_choices: [3, 4],
      label: "Gain life and put counters",
    });
    const picks = { ...emptyCostPicks(), sacrifice: 3 };
    expect(planCostPipeline(action, card(1), picks)).toEqual({
      kind: "run",
      action,
      card: card(1),
      picks,
    });
    expect(planRunAction(action, card(1), picks, null)).toEqual({
      kind: "stage",
      card: card(1),
      action,
      picks,
    });
    expect(usedCostPick(picks)).toBe(false);
  });

  it("parks on discard pick for additional discard costs", () => {
    const action = mkAction({ discard_choices: [2, 3], discard_count: 1 });
    const c = card(1);
    expect(planHandDrop(action, c, 100, 800)).toEqual({
      kind: "discard-pick",
      action,
      card: c,
      picks: emptyCostPicks(),
    });
  });

  it("rejects discard costs with no legal payers", () => {
    const action = mkAction({ discard_choices: [], discard_count: 1 });
    expect(planHandDrop(action, card(1), 100, 800)).toEqual({
      kind: "reject",
      reason: "CannotDiscardCost",
    });
  });
});

describe("planCostPipeline", () => {
  it("asks for graveyard exile after discard is settled", () => {
    const action = mkAction({
      discard_choices: [2],
      discard_count: 1,
      graveyard_exile_choices: [8, 9],
      graveyard_exile_min: 2,
      graveyard_exile_max: 2,
    });
    const c = card(1);
    const picks = { ...emptyCostPicks(), discard_cost: [2], discard_settled: true };
    expect(planCostPipeline(action, c, picks)).toEqual({
      kind: "gy-exile-pick",
      action,
      card: c,
      picks,
    });
  });

  it("rejects escape costs with an empty graveyard", () => {
    const action = mkAction({
      graveyard_exile_choices: [],
      graveyard_exile_min: 4,
      graveyard_exile_max: 4,
    });
    expect(planCostPipeline(action, card(1), emptyCostPicks())).toEqual({
      kind: "reject",
      reason: "CannotExileCost",
    });
  });

  it("skips gy exile when max is zero", () => {
    const action = mkAction({
      graveyard_exile_choices: [],
      graveyard_exile_min: 0,
      graveyard_exile_max: 0,
    });
    expect(planCostPipeline(action, card(1), emptyCostPicks()).kind).toBe("run");
  });
});

describe("planRunAction", () => {
  it("stages a targeted action", () => {
    const action = mkAction({ object: 1, needs_target: true, targets: [{ kind: "object", id: 2 }] });
    const c = card(1);
    const picks = emptyCostPicks();
    expect(planRunAction(action, c, picks, null)).toEqual({
      kind: "stage",
      card: c,
      action,
      picks,
    });
  });

  it("stages a targeted escape cast when the card is in state but was not passed in", () => {
    const action = mkAction({
      id: 42,
      object: 77,
      label: "Sentinel's Eyes",
      needs_target: true,
      targets: [{ kind: "object", id: 2 }],
      graveyard_exile_choices: [8, 9],
      graveyard_exile_min: 2,
      graveyard_exile_max: 2,
    });
    const picks = { ...emptyCostPicks(), graveyard_exile: [8, 9], gy_exile_settled: true };
    const state = { objects: [card(77)] } as VisibleState;
    const plan = planRunAction(action, null, picks, state);
    expect(plan.kind).toBe("stage");
    if (plan.kind !== "stage") return;
    expect(plan.card.id).toBe(77);
    expect(plan.picks.graveyard_exile).toEqual([8, 9]);
  });

  it("rejects staging when the cast object is not in state", () => {
    const action = mkAction({
      object: 77,
      needs_target: true,
      targets: [{ kind: "object", id: 2 }],
    });
    const emptyState = { objects: [] } as unknown as VisibleState;
    expect(planRunAction(action, null, emptyCostPicks(), emptyState)).toEqual({
      kind: "reject",
      reason: "UnknownObject",
    });
  });

  it("fires a cast with cost picks", () => {
    const action = mkAction();
    const picks = { ...emptyCostPicks(), discard_cost: [3], discard_settled: true };
    expect(planRunAction(action, card(1), picks, null)).toEqual({
      kind: "cast",
      action,
      picks,
    });
  });
});
