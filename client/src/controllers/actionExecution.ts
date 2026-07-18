import { type Accessor, createMemo, createSignal } from "solid-js";
import { humanReason } from "~/controllers/reject";
import { CARD_H, CARD_W } from "~/layout";
import { type Camera, screenToWorld } from "~/lib/camera";
import { advance } from "~/lib/modal";
import { targetMode } from "~/lib/targeting";
import type {
  ActionView,
  ModeView,
  ObjectView,
  VisibleState,
  WireIntent,
  WireModeChoice,
  WireTarget,
} from "~/wire/types";

type Vec = { x: number; y: number };

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

/** Snapshot a sacrifice dialog's payload before clearing the Show signal.
 *
 * Solid's `<Show when={sacrificePick()}>{(sp) => …}` accessor reads the live signal — after
 * `setSacrificePick(null)`, `sp().action` throws and the activate never stages onto the stack
 * (prompt vanishes, nothing happens). Capture first, clear second — same order as discard/exile.
 */
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
  /** After escape/delve/discard, prefer the target picker over the arrow (easy to miss). Not set
   * for sacrifice alone — those activates stage onto the stack and aim normally. */
  preferPick: boolean;
  /** World origin for the play-in leg (drop or hand slot) — canvas land path. */
  playOrigin: Vec;
  /** Screen origin for the play-in leg — stack DOM path. */
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

/** Board click on a legal target: staged casts must use completeTarget (cost picks ride the intent). */
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

export interface ModalCast {
  action: ActionView;
  modes: ModeView[];
  picks: CostPicks;
  /** The chosen mode indices, or null while the mode picker is still up. */
  chosen: number[] | null;
  /** Answers for `chosen`, filled left to right; complete when the two lengths match. */
  answers: WireModeChoice[];
}

export interface ActionExecutionDeps {
  me: Accessor<number>;
  act: (intent: WireIntent) => Promise<boolean>;
  getState: () => VisibleState | null;
  camera: Accessor<Camera>;
  size: Accessor<Vec>;
  handBarH: number;
  setReject: (msg: string | null) => void;
  /** Record play-in origin and spawn canvas flight (ADR 0035). */
  seedDrop: (cardId: number, world: Vec, screen: Vec, flight: "battlefield" | "stack") => void;
  clearPlayOrigin: (cardId: number) => void;
  onHintUsed: () => void;
}

/** Return shape of [`useActionExecution`] — the internal chrome/session port (not Board). */
export type ActionExecution = ReturnType<typeof useActionExecution>;

/** Staged targeting, modal casts, cost picks, and take_action submission. */
export function useActionExecution(deps: ActionExecutionDeps) {
  const [staged, setStaged] = createSignal<StagedAction | null>(null);
  /** Staged card flying back to hand after cancel (ADR 0033). */
  const [returningStaged, setReturningStaged] = createSignal<StagedAction | null>(null);
  let returnTimer: ReturnType<typeof setTimeout> | null = null;
  const [xPrompt, setXPrompt] = createSignal<{ name: string; submit: (x: number) => void } | null>(null);
  const [modalCast, setModalCast] = createSignal<ModalCast | null>(null);
  const [sacrificePick, setSacrificePick] = createSignal<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>(null);
  const [discardPick, setDiscardPick] = createSignal<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>(null);
  const [gyExilePick, setGyExilePick] = createSignal<{
    action: ActionView;
    card: ObjectView | null;
    dropSeed: Vec;
    screenOrigin: Vec;
    picks: CostPicks;
  } | null>(null);

  const takeCastAction = async (
    action: ActionView,
    target: WireTarget | null,
    x?: number,
    modes: WireModeChoice[] = [],
    picks: CostPicks = emptyCostPicks(),
  ) => {
    const state = deps.getState();
    if (!state?.actions?.some((a) => a.id === action.id)) {
      deps.setReject(humanReason("UnknownAction"));
      return;
    }
    const card = state.objects.find((o) => o.id === action.object);
    // Prefer ActionView.has_x (back face for cast_prepared; paid cost for cast) over the
    // permanent/card object's front-face mana_cost.
    if (action.has_x && x === undefined) {
      setXPrompt({
        name: action.kind === "cast_prepared" ? action.label : (card?.name ?? action.label),
        submit: (chosen) => takeCastAction(action, target, chosen, modes, picks),
      });
      return;
    }
    setXPrompt(null);
    await deps.act(buildTakeActionIntent(deps.me(), action.id, target, x ?? 0, modes, picks));
  };

  /** Commander recast from the command-zone card on the canvas — routed through take_action (ADR 0020). */
  const castFromCommandZone = async (card: ObjectView, target: WireTarget | null, x?: number) => {
    const action = findCastActionForObject(deps.getState()?.actions, card.id);
    if (!action) {
      deps.setReject(humanReason("NotCastable"));
      return;
    }
    await takeCastAction(action, target, x);
  };

  const stagedMode = createMemo(() => {
    const s = staged();
    const state = deps.getState();
    if (!s || !state) return { kind: "none" as const };
    return targetMode(s.action, state);
  });

  const stagedObjects = (): ReadonlySet<number> => {
    const m = stagedMode();
    return m.kind === "arrow" ? m.objects : new Set();
  };
  const stagedPlayers = (): ReadonlySet<number> => {
    const m = stagedMode();
    return m.kind === "arrow" ? m.players : new Set();
  };

  const completeTarget = (target: WireTarget) => {
    const s = staged();
    if (!s) return;
    setStaged(null);
    const sub = stagedCastSubmission(s, target);
    void takeCastAction(sub.action, sub.target, undefined, [], sub.picks);
  };

  const advanceModal = (mc: ModalCast & { chosen: number[] }) => {
    const step = advance(mc.modes, mc.chosen, mc.answers);
    if (step.kind === "submit") {
      setModalCast(null);
      void takeCastAction(mc.action, null, undefined, step.modes, mc.picks);
      return;
    }
    setModalCast(mc);
  };

  const pendingMode = () => {
    const mc = modalCast();
    if (!mc?.chosen) return null;
    const step = advance(mc.modes, mc.chosen, mc.answers);
    return step.kind === "ask" ? step.mode : null;
  };

  const answerMode = (target: WireTarget) => {
    const mc = modalCast();
    const chosen = mc?.chosen;
    if (!mc || !chosen) return;
    const step = advance(mc.modes, chosen, mc.answers);
    if (step.kind !== "ask") return;
    advanceModal({ ...mc, chosen, answers: [...mc.answers, { index: step.index, target }] });
  };

  const objectName = (id: number): string => deps.getState()?.objects.find((o) => o.id === id)?.name ?? `#${id}`;
  // Printing UUID for a target's art (ADR 0031); empty renders a broken image.
  const objectPrint = (id: number): string => deps.getState()?.objects.find((o) => o.id === id)?.print ?? "";

  const runAction = (
    action: ActionView,
    card: ObjectView | null,
    picks: CostPicks,
    dropSeed: Vec,
    screenOrigin: Vec,
  ) => {
    const plan = planRunAction(action, card, picks, deps.getState());
    if (plan.kind === "noop") return;
    if (plan.kind === "reject") {
      deps.setReject(humanReason(plan.reason));
      return;
    }
    if (plan.kind === "stage") {
      if (plan.card.id != null) deps.seedDrop(plan.card.id, dropSeed, screenOrigin, "stack");
      setStaged({
        card: plan.card,
        action: plan.action,
        picks: plan.picks,
        preferPick: usedCostPick(plan.picks),
        playOrigin: dropSeed,
        playOriginScreen: screenOrigin,
      });
      return;
    }
    if (plan.kind === "play-land") {
      if (card) deps.seedDrop(card.id, dropSeed, screenOrigin, "battlefield");
      void deps.act(buildTakeActionIntent(deps.me(), plan.actionId, null, 0, [], plan.picks));
      return;
    }
    if (plan.kind === "cast") {
      if (card) deps.seedDrop(card.id, dropSeed, screenOrigin, "stack");
      void takeCastAction(plan.action, null, undefined, [], plan.picks);
      return;
    }
    void deps.act(buildTakeActionIntent(deps.me(), plan.actionId, null, 0, [], plan.picks));
  };

  /** Continue after a cost-pick prompt resolves. */
  const continueAfterCostPick = (
    action: ActionView,
    card: ObjectView | null,
    picks: CostPicks,
    dropSeed: Vec,
    screenOrigin: Vec,
  ) => {
    const plan = planCostPipeline(action, card, picks);
    if (plan.kind === "reject") {
      deps.setReject(humanReason(plan.reason));
      return;
    }
    if (plan.kind === "sacrifice-pick") {
      setSacrificePick({ action, card, dropSeed, screenOrigin, picks });
      return;
    }
    if (plan.kind === "discard-pick") {
      setDiscardPick({ action, card, dropSeed, screenOrigin, picks });
      return;
    }
    if (plan.kind === "gy-exile-pick") {
      setGyExilePick({ action, card, dropSeed, screenOrigin, picks });
      return;
    }
    if (plan.kind === "modal") {
      setModalCast({ action: plan.action, modes: plan.modes, chosen: null, answers: [], picks: plan.picks });
      return;
    }
    if (plan.kind === "run") {
      runAction(plan.action, plan.card, plan.picks, dropSeed, screenOrigin);
    }
  };

  const onHandDrop = (action: ActionView, x: number, y: number) => {
    const threshold = deps.size().y - deps.handBarH;
    const card = action.object != null ? (deps.getState()?.objects.find((o) => o.id === action.object) ?? null) : null;
    const plan = planHandDrop(action, card, y, threshold);
    if (plan.kind === "ignore") return;
    deps.onHintUsed();
    const w = screenToWorld(deps.camera(), x, y);
    const dropSeed = { x: w.x - CARD_W / 2, y: w.y - CARD_H / 2 };
    const screenOrigin = { x, y };
    if (plan.kind === "reject") {
      deps.setReject(humanReason(plan.reason));
      return;
    }
    if (plan.kind === "sacrifice-pick") {
      setSacrificePick({ action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks });
      return;
    }
    if (plan.kind === "discard-pick") {
      setDiscardPick({ action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks });
      return;
    }
    if (plan.kind === "gy-exile-pick") {
      setGyExilePick({ action: plan.action, card: plan.card, dropSeed, screenOrigin, picks: plan.picks });
      return;
    }
    if (plan.kind === "modal") {
      setModalCast({ action: plan.action, modes: plan.modes, chosen: null, answers: [], picks: plan.picks });
      return;
    }
    if (plan.kind === "run") {
      runAction(plan.action, plan.card, plan.picks, dropSeed, screenOrigin);
    }
  };

  const cancelStagedOnly = () => {
    const s = staged();
    if (!s) return;
    deps.clearPlayOrigin(s.card.id);
    setReturningStaged(s);
    if (returnTimer) clearTimeout(returnTimer);
    returnTimer = setTimeout(() => {
      setReturningStaged(null);
      returnTimer = null;
    }, 220);
    setStaged(null);
  };

  const cancelActionState = () => {
    cancelStagedOnly();
    setXPrompt(null);
    setModalCast(null);
    setSacrificePick(null);
    setDiscardPick(null);
    setGyExilePick(null);
  };

  return {
    staged,
    setStaged,
    returningStaged,
    xPrompt,
    setXPrompt,
    modalCast,
    setModalCast,
    sacrificePick,
    setSacrificePick,
    discardPick,
    setDiscardPick,
    gyExilePick,
    setGyExilePick,
    stagedMode,
    stagedObjects,
    stagedPlayers,
    takeCastAction,
    castFromCommandZone,
    completeTarget,
    advanceModal,
    pendingMode,
    answerMode,
    objectName,
    objectPrint,
    runAction,
    continueAfterCostPick,
    onHandDrop,
    cancelStagedOnly,
    cancelActionState,
    getState: deps.getState,
  };
}
