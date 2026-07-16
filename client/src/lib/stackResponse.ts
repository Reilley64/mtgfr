// Instant-speed priority focus: which permanents stay bright while the rest dim.

import type { ActionView } from "~/api/generated";
import { STEP } from "~/layout";

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

/** Seat facts for stack / instant-window chrome (ADR 0026 / 0027). */
export type StackChromeInput = {
  spectating: boolean;
  staged: boolean;
  yielded: boolean;
  stackLen: number;
  holdRemainingMs: number;
  canAct: boolean;
  viewer: number;
  priority: number;
  active: number;
  step: number;
  actions: readonly ActionView[] | undefined;
  manaSources: readonly ManaSourceCard[];
};

/**
 * One policy snapshot for Pass / stack yield / Space / dwell / Next / focus.
 * Priority context bar, keyboard, and canvas all read this — they do not reassemble predicates.
 */
export type StackChrome = {
  /** One-shot Stack Pass — bar button and Space/Enter while the stack is up. */
  pass: boolean;
  /** Clickable stack-yield arm (one-shot). */
  stackYieldArm: boolean;
  /** Armed stack yield — show disabled control, no cancel. */
  stackYieldArmed: boolean;
  /** Helpless during an uncontested hold (dwell eligibility base). */
  helpless: boolean;
  /** Hover may POST dwell while hold is live. */
  allowDwell: boolean;
  /** Instant-priority battlefield dimming. */
  focus: boolean;
  /** IDs that stay bright under `focus`. */
  brightIds: ReadonlySet<number>;
  /** Hide priority-bar Next while the stack owns Pass. */
  hideControlsPass: boolean;
  /** Space/Enter while stackLen > 0: pass_priority vs ignore. */
  spaceOnStack: "pass_priority" | "ignore";
};

/** Deep StackChrome module — pure policy; Board only binds UI / intents. */
export function stackChrome(input: StackChromeInput): StackChrome {
  // Local arrow staging owns the seat — Pass / Space would orphan the staged cast.
  const pass = !input.staged && canActOnStack(input);
  const stackYieldArm = showStackYieldArm(input);
  const stackYieldArmed = showStackYieldArmed(input);
  const helpless = viewerIsHelpless(input.actions);
  const focus = instantPriorityFocus(input);
  return {
    pass,
    stackYieldArm,
    stackYieldArmed,
    helpless,
    allowDwell: !input.spectating && helpless && input.holdRemainingMs > 0,
    focus,
    brightIds: activatableBattlefieldIds(input.actions, focus, input.manaSources, input.viewer),
    hideControlsPass: input.stackLen > 0,
    spaceOnStack: pass ? "pass_priority" : "ignore",
  };
}
