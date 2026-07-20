// Instant-speed priority focus: which permanents stay bright while the rest dim.

import { STEP } from "~/layout";
import { SPECTATOR_VIEWER } from "~/store";
import type { ActionView, VisibleState } from "~/wire/types";

/** Untapped mana-source facts the radial synthesizes (not present in wire `actions`). */
export type ManaSourceCard = {
  id: number;
  controller: number;
  tapsForMana: boolean;
  tapped: boolean;
};

/**
 * Empty-stack windows where the battlefield stays fully bright (no activate-focus dimming).
 * Main phases for the active player, plus declare-attackers / declare-blockers for everyone —
 * combat declaration is its own job; hand cards still dim via missing cast actions.
 */
export function fullBattlefieldWindow(stackLen: number, active: number, viewer: number, step: number): boolean {
  if (stackLen > 0) return false;
  if (step === STEP.DeclareAttackers || step === STEP.DeclareBlockers) return true;
  if (active !== viewer) return false;
  return step === STEP.Main1 || step === STEP.Main2;
}

/**
 * Viewer holds priority in an instant-speed window — dim non-usable permanents.
 * Off for spectators, and during main / combat-declaration windows.
 */
export function instantPriorityFocus(opts: {
  canAct: boolean;
  stackLen: number;
  viewer: number;
  priority: number;
  active: number;
  step: number;
  spectating?: boolean;
}): boolean {
  if (opts.spectating) return false;
  if (!opts.canAct || opts.viewer !== opts.priority) return false;
  return !fullBattlefieldWindow(opts.stackLen, opts.active, opts.viewer, opts.step);
}

/**
 * This seat currently holds priority with a meaningful action while the stack is up —
 * one-shot Pass / stack-yield arm are offered.
 */
export function canActOnStack(opts: {
  spectating: boolean;
  stackLen: number;
  canAct: boolean;
  viewer: number;
  priority: number;
  actions: readonly { id: number }[] | undefined;
}): boolean {
  if (opts.spectating || opts.stackLen <= 0) return false;
  if (!opts.canAct || opts.viewer !== opts.priority) return false;
  return !viewerIsHelpless(opts.actions);
}

/** Stack-yield arm button: can act now and not yet armed (ADR 0027 one-shot). */
export function showStackYieldArm(opts: {
  spectating: boolean;
  staged: boolean;
  yielded: boolean;
  stackLen: number;
  canAct: boolean;
  viewer: number;
  priority: number;
  actions: readonly { id: number }[] | undefined;
}): boolean {
  if (opts.spectating || opts.staged || opts.stackLen <= 0 || opts.yielded) return false;
  return canActOnStack(opts);
}

/** Disabled stack-yield control while armed for this stack (no chrome cancel). */
export function showStackYieldArmed(opts: {
  spectating: boolean;
  staged: boolean;
  yielded: boolean;
  stackLen: number;
}): boolean {
  if (opts.spectating || opts.staged || opts.stackLen <= 0) return false;
  return opts.yielded;
}

/**
 * Permanents to keep bright during instant priority: legal non-mana activates, plus the viewer's
 * untapped mana sources (mana abilities are omitted from wire `actions` but still usable).
 */
export function activatableBattlefieldIds(
  actions: readonly ActionView[] | undefined,
  focus: boolean,
  manaSources: readonly ManaSourceCard[],
  viewer: number,
): ReadonlySet<number> {
  if (!focus) return new Set();
  const ids = new Set<number>();
  for (const a of actions ?? []) {
    if (a.section !== "battlefield" || a.object == null) continue;
    ids.add(a.object);
  }
  for (const c of manaSources) {
    if (c.controller !== viewer || !c.tapsForMana || c.tapped) continue;
    ids.add(c.id);
  }
  return ids;
}

/**
 * Viewer has no projected legal actions — same seat-level idea as the server's
 * `has_meaningful_action` (mana abilities are omitted from both).
 */
export function viewerIsHelpless(actions: readonly { id: number }[] | undefined): boolean {
  return (actions?.length ?? 0) === 0;
}

/** Seat facts for stack / instant-window / turn-yield chrome (ADR 0026 / 0027 / 0029). */
export type StackChromeInput = {
  spectating: boolean;
  staged: boolean;
  yielded: boolean;
  /** Seat's turn-yield flag from the wire (`turn_yielded`). */
  turnYielded: boolean;
  stackLen: number;
  holdRemainingMs: number;
  canAct: boolean;
  viewer: number;
  priority: number;
  active: number;
  step: number;
  actions: readonly ActionView[] | undefined;
  manaSources: readonly ManaSourceCard[];
  /**
   * Local declare-attackers staging has at least one attacker and Attack has not been confirmed.
   * End Turn must not compete with `Attack (N)` (clicking it would auto-pass and seal an empty declare).
   */
  pendingAttackers: boolean;
};

/**
 * One policy snapshot for Pass / stack yield / turn yield / Space / dwell / Next / focus.
 * Priority context bar, keyboard, and canvas all read this — they do not reassemble predicates.
 */
export type StackChrome = {
  /** One-shot resolve-card control — bar button and Space/Enter while the stack is up (`pass_priority`). */
  pass: boolean;
  /** Clickable resolve-stack arm (one-shot stack yield). */
  stackYieldArm: boolean;
  /** Armed resolve-stack — show disabled control, no cancel. */
  stackYieldArmed: boolean;
  /** Helpless during an uncontested hold (dwell eligibility base). */
  helpless: boolean;
  /** Hover may POST dwell while hold is live. */
  allowDwell: boolean;
  /** Instant-priority battlefield dimming. */
  focus: boolean;
  /** IDs that stay bright under `focus`. */
  brightIds: ReadonlySet<number>;
  /** Hide priority-bar Next while the stack owns Resolve card. */
  hideControlsPass: boolean;
  /**
   * Space/Enter binding: resolve-card pass on stack, fire primary Next on empty stack, or ignore.
   * Board still gates prompt-open / inspect — not chrome policy.
   */
  space: "pass_priority" | "primary" | "ignore";
  /**
   * Show the turn-yield rocker (ADR 0029). Hidden for spectators and on your own turn —
   * on your turn the same flag is **End Turn** (`showEndTurn`).
   */
  showTurnYield: boolean;
  /** Current turn-yield / end-turn armed state (wire mirror). */
  turnYielded: boolean;
  /**
   * Show Arena End Turn (ADR 0037) — arms the same `turn_yielded` flag while you are active.
   * Hidden for spectators, when it is not your turn (`showTurnYield` covers until-my-turn),
   * while Resolve card / Resolve stack own the pass job, and while Attack (N) is pending.
   */
  showEndTurn: boolean;
  /**
   * Armed End Turn must disarm when chrome would hide the arm control (stack resolve / pending
   * Attack). Board POSTs SetTurnYield(false). Never for until-my-turn (not active).
   */
  clearEndTurn: boolean;
  /**
   * Show the priority-bar primary (Next / Declare / …). Hidden when the stack owns Resolve card and
   * the primary action is itself a bare pass (duplicate affordance).
   */
  showPrimary: (primaryKind: string) => boolean;
};

/** Whether the turn-yield rocker is offered (ADR 0029 — until my turn, not your turn). */
export function showTurnYieldControl(opts: { spectating: boolean; viewer: number; active: number }): boolean {
  if (opts.spectating) return false;
  return opts.active !== opts.viewer;
}

/**
 * Whether End Turn is offered (ADR 0037 — same flag, only while you are active).
 * Hidden while stack resolve chrome is the priority job, or while Attack (N) is pending confirm.
 */
export function showEndTurnControl(opts: {
  spectating: boolean;
  viewer: number;
  active: number;
  /** Resolve card / Resolve stack (armed or arm) — End Turn must not compete. */
  stackResolveChrome: boolean;
  /** Staged attackers awaiting Attack confirm. */
  pendingAttackers: boolean;
}): boolean {
  if (opts.spectating) return false;
  if (opts.active !== opts.viewer) return false;
  if (opts.stackResolveChrome) return false;
  if (opts.pendingAttackers) return false;
  return true;
}

/**
 * Armed End Turn should disarm when the seat enters a window where End Turn cannot be armed.
 * Does not clear until-my-turn (active !== viewer).
 */
export function clearEndTurnControl(opts: {
  spectating: boolean;
  viewer: number;
  active: number;
  turnYielded: boolean;
  stackResolveChrome: boolean;
  pendingAttackers: boolean;
}): boolean {
  if (!opts.turnYielded || opts.spectating) return false;
  if (opts.active !== opts.viewer) return false;
  return opts.stackResolveChrome || opts.pendingAttackers;
}

/**
 * Space/Enter binding for priority chrome (ADR 0026 / 0027). Spectators never bind;
 * empty-stack fires primary; stack uses one-shot pass when the seat can act.
 */
export function spaceBinding(opts: {
  spectating: boolean;
  staged: boolean;
  stackLen: number;
  canAct: boolean;
  viewer: number;
  priority: number;
  actions: readonly { id: number }[] | undefined;
}): "pass_priority" | "primary" | "ignore" {
  if (opts.spectating) return "ignore";
  if (opts.stackLen > 0) {
    if (opts.staged) return "ignore";
    return canActOnStack(opts) ? "pass_priority" : "ignore";
  }
  // Empty stack: Board still requires yours()/!promptOpen — chrome only says "primary is live".
  return "primary";
}

/** Deep StackChrome module — pure policy; Board only binds UI / intents. */
export function stackChrome(input: StackChromeInput): StackChrome {
  // Local arrow staging owns the seat — Pass / Space would orphan the staged cast.
  const pass = !input.staged && canActOnStack(input);
  const stackYieldArm = showStackYieldArm(input);
  const stackYieldArmed = showStackYieldArmed(input);
  const helpless = viewerIsHelpless(input.actions);
  const focus = instantPriorityFocus(input);
  const hideControlsPass = input.stackLen > 0;
  const space = spaceBinding(input);
  const stackResolveChrome = pass || stackYieldArm || stackYieldArmed;
  const endTurnOpts = {
    spectating: input.spectating,
    viewer: input.viewer,
    active: input.active,
    stackResolveChrome,
    pendingAttackers: input.pendingAttackers,
  };
  return {
    pass,
    stackYieldArm,
    stackYieldArmed,
    helpless,
    allowDwell: !input.spectating && helpless && input.holdRemainingMs > 0,
    focus,
    brightIds: activatableBattlefieldIds(input.actions, focus, input.manaSources, input.viewer),
    hideControlsPass,
    space,
    showTurnYield: showTurnYieldControl(input),
    turnYielded: input.turnYielded,
    showEndTurn: showEndTurnControl(endTurnOpts),
    clearEndTurn: clearEndTurnControl({ ...endTurnOpts, turnYielded: input.turnYielded }),
    showPrimary: (primaryKind) => !(hideControlsPass && primaryKind === "pass"),
  };
}

/**
 * Local UI facts not on the wire. `staged` blocks Pass / Space / yield; `manaSources` come from
 * layout cards (mana abilities are omitted from wire `actions`).
 * promptOpen / yours() stay in the view — keyboard still gates those.
 */
export type BoardChromeLocal = {
  staged: boolean;
  manaSources: readonly ManaSourceCard[];
  /** True when Attack (N) is pending — local staged attackers, not yet confirmed. */
  pendingAttackers: boolean;
};

/** Map VisibleState + local staging into StackChromeInput (no Board field scattering). */
export function stackChromeInputFromState(state: VisibleState | null, local: BoardChromeLocal): StackChromeInput {
  if (!state) {
    return {
      spectating: false,
      staged: local.staged,
      yielded: false,
      turnYielded: false,
      stackLen: 0,
      holdRemainingMs: 0,
      canAct: false,
      viewer: 0,
      priority: -1,
      active: -1,
      step: -1,
      actions: undefined,
      manaSources: local.manaSources,
      pendingAttackers: local.pendingAttackers,
    };
  }
  const spectating = state.viewer === SPECTATOR_VIEWER;
  return {
    spectating,
    staged: local.staged,
    yielded: state.yielded ?? false,
    turnYielded: state.turn_yielded ?? false,
    stackLen: state.stack.length,
    holdRemainingMs: state.stack_hold_remaining_ms ?? 0,
    canAct: state.can_act,
    // Layout seat: spectators fall back to 0 (same as Board's me()).
    viewer: spectating ? 0 : state.viewer,
    priority: state.priority,
    active: state.active_player,
    step: state.step,
    actions: state.actions,
    manaSources: local.manaSources,
    pendingAttackers: local.pendingAttackers,
  };
}

/** Board chrome binder: VisibleState + locals → StackChrome. Pure; Effect wire stays upstream. */
export function boardChromeFromState(state: VisibleState | null, local: BoardChromeLocal): StackChrome {
  return stackChrome(stackChromeInputFromState(state, local));
}
