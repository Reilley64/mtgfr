// Pure planners for pre-submit action pipeline. Board update.ts folds these into BoardModel and
// emits SubmitIntent commands via the parent app update. No Solid signals here; state lives in
// BoardModel.

import type {
  ActionView,
  ModeView,
  ObjectView,
  VisibleState,
  WireCost,
  WireIntent,
  WireModeChoice,
  WireTarget,
} from "~/wire/types";
import { type TargetMode, targetMode } from "./targeting";

export type Vec = { x: number; y: number };

/** Pre-submit cost picks that ride `take_action` (discard / delve-escape / sacrifice). */
export type CostPicks = {
  discard_cost: number[];
  graveyard_exile: number[];
  sacrifice: number | null;
  /** True once the discard prompt (if any) has been answered — empty pick is still a settlement. */
  discard_settled: boolean;
  /** True once the GY-exile prompt (if any) has been answered — empty delve is a settlement. */
  gy_exile_settled: boolean;
};

export const emptyCostPicks = (): CostPicks => ({
  discard_cost: [],
  graveyard_exile: [],
  sacrifice: null,
  discard_settled: false,
  gy_exile_settled: false,
});

/** True when a full-screen cost dialog should force the *target* picker next (not the stack arrow).
 *
 * Escape/delve/discard casts often need that — the arrow is easy to miss right after a modal.
 * Sacrifice alone must not: Dina-style activates stage onto the stack and aim at a creature like
 * any other targeted ability (`preferPick` would hide the stack ghost and open a second modal).
 */
export function usedCostPick(picks: CostPicks): boolean {
  if (picks.discard_settled) return true;
  // Auto-settled delve (min/max 0) sets gy_exile_settled without a dialog — ignore that.
  return picks.gy_exile_settled && picks.graveyard_exile.length > 0;
}

/** Snapshot a sacrifice dialog's payload before clearing the Show signal. */
export function settleSacrificePick(
  pick: {
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  },
  sacrificed: number,
): {
  action: ActionView;
  card: ObjectView | null;
  dropSeed: Vec;
  screenOrigin: Vec;
  picks: CostPicks;
} {
  return {
    action: pick.action,
    card: pick.card,
    dropSeed: pick.dropSeed,
    screenOrigin: pick.screenOrigin,
    picks: { ...pick.picks, sacrifice: sacrificed },
  };
}

export type StagedAction = {
  card: ObjectView;
  action: ActionView;
  picks: CostPicks;
  /** After escape/delve/discard, prefer the target picker over the arrow (easy to miss). */
  preferPick: boolean;
  /** World origin for the play-in leg (drop or hand slot). */
  playOrigin: Vec;
  /** Screen origin for the play-in leg. */
  playOriginScreen: Vec;
};

export function buildTakeActionIntent(
  player: number,
  id: number,
  target: WireTarget | null = null,
  x = 0,
  modes: WireModeChoice[] = [],
  picks: CostPicks = emptyCostPicks(),
): WireIntent {
  return {
    kind: "take_action",
    player,
    id,
    target,
    x,
    modes,
    sacrifice: picks.sacrifice === null ? [] : [picks.sacrifice],
    discard_cost: picks.discard_cost,
    graveyard_exile: picks.graveyard_exile,
  };
}

/** Finish aiming a staged action: target click must forward cost picks (escape/delve/discard). */
export function stagedCastSubmission(
  staged: { action: ActionView; picks: CostPicks },
  target: WireTarget,
): { action: ActionView; target: WireTarget; picks: CostPicks } {
  return { action: staged.action, target, picks: staged.picks };
}

export type CastClickResolution =
  | { kind: "complete-staged-target"; target: WireTarget }
  | { kind: "cast-commander"; card: ObjectView; target: WireTarget | null }
  | { kind: "ignore" };

/** Board click on a legal target. */
export function planCastClickResolution(
  hasStaged: boolean,
  click: { card: ObjectView; target: WireTarget | null },
): CastClickResolution {
  if (hasStaged && click.target) return { kind: "complete-staged-target", target: click.target };
  if (!hasStaged) return { kind: "cast-commander", card: click.card, target: click.target };
  return { kind: "ignore" };
}

/** The cast action for a commander (or any object) in the viewer's action list. */
export function findCastActionForObject(actions: ActionView[] | undefined, objectId: number): ActionView | undefined {
  return actions?.find((a) => a.kind === "cast" && a.object === objectId);
}

export type HandDropPlan =
  | { kind: "ignore" }
  | { kind: "modal"; action: ActionView; modes: ModeView[]; picks: CostPicks }
  | { kind: "sacrifice-pick"; action: ActionView; card: ObjectView | null; picks: CostPicks }
  | { kind: "discard-pick"; action: ActionView; card: ObjectView | null; picks: CostPicks }
  | { kind: "gy-exile-pick"; action: ActionView; card: ObjectView | null; picks: CostPicks }
  | { kind: "run"; action: ActionView; card: ObjectView | null; picks: CostPicks }
  | { kind: "reject"; reason: string };

/** Next pre-submit step for an action once any earlier cost picks are settled. */
export function planCostPipeline(action: ActionView, card: ObjectView | null, picks: CostPicks): HandDropPlan {
  const sacrifice = action.sacrifice_choices;
  if (sacrifice && picks.sacrifice == null) {
    if (sacrifice.length === 0) return { kind: "reject", reason: "CannotActivate" };
    return { kind: "sacrifice-pick", action, card, picks };
  }
  if (action.discard_choices != null && !picks.discard_settled) {
    if (action.discard_choices.length === 0) return { kind: "reject", reason: "CannotDiscardCost" };
    return { kind: "discard-pick", action, card, picks };
  }
  if (action.graveyard_exile_choices != null && !picks.gy_exile_settled) {
    const min = action.graveyard_exile_min ?? 0;
    const max = action.graveyard_exile_max ?? 0;
    if (min === 0 && max === 0) {
      return planCostPipeline(action, card, { ...picks, gy_exile_settled: true });
    }
    if (action.graveyard_exile_choices.length === 0 && min > 0) {
      return { kind: "reject", reason: "CannotExileCost" };
    }
    return { kind: "gy-exile-pick", action, card, picks };
  }
  if (action.modal) return { kind: "modal", action, modes: action.modal.modes, picks };
  return { kind: "run", action, card, picks };
}

/** What a hand-bar drop should do once the release is above the play threshold. */
export function planHandDrop(action: ActionView, card: ObjectView | null, y: number, threshold: number): HandDropPlan {
  if (y > threshold) return { kind: "ignore" };
  return planCostPipeline(action, card, emptyCostPicks());
}

export type RunActionPlan =
  | { kind: "noop" }
  | { kind: "reject"; reason: string }
  | { kind: "stage"; card: ObjectView; action: ActionView; picks: CostPicks }
  | { kind: "play-land"; actionId: number; picks: CostPicks }
  | { kind: "cast"; action: ActionView; picks: CostPicks }
  | { kind: "take"; actionId: number; picks: CostPicks };

/** Everything an action does once its activation cost is settled: stage for a target, or fire. */
export function planRunAction(
  action: ActionView,
  card: ObjectView | null,
  picks: CostPicks,
  state: VisibleState | null,
): RunActionPlan {
  if (action.needs_target) {
    if (action.object == null) return { kind: "reject", reason: "UnknownObject" };
    const stageCard = card ?? state?.objects.find((o) => o.id === action.object) ?? null;
    if (!stageCard) return { kind: "reject", reason: "UnknownObject" };
    if (state && targetMode(action, state).kind === "impossible") {
      return { kind: "reject", reason: "IllegalTarget" };
    }
    return { kind: "stage", card: stageCard, action, picks };
  }
  if (action.kind === "play_land") return { kind: "play-land", actionId: action.id, picks };
  if (action.kind === "cast" || action.kind === "cast_prepared") {
    return { kind: "cast", action, picks };
  }
  // cycle / activate / combat declarations
  return { kind: "take", actionId: action.id, picks };
}

/** Modal cast state — chosen modes + pending answers. */
export interface ModalCast {
  action: ActionView;
  modes: ModeView[];
  picks: CostPicks;
  chosen: number[] | null;
  answers: WireModeChoice[];
  modeDraft: number[];
}

/** Pending X prompt — carries all context needed to re-submit with a chosen X. */
export type XPromptState = {
  action: ActionView;
  target: WireTarget | null;
  picks: CostPicks;
  modes: WireModeChoice[];
  name: string;
  minX: number;
  maxX: number;
  /** Clamped draft while the stepper is open; Confirm submits this value. */
  draftX: number;
  xCost: WireCost;
};

/** Pending sacrifice / discard / gy-exile pick — shared shape. */
export type CostPickState = {
  action: ActionView;
  card: ObjectView | null;
  dropSeed: Vec;
  screenOrigin: Vec;
  picks: CostPicks;
};

export type { TargetMode };
