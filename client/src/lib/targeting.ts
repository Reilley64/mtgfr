// How the board asks for a staged action's target.
//
// The engine already enumerates what's legal (`Game::legal_targets`, on the wire as
// `ActionView.targets`), so nothing here re-derives `TargetSpec` — this module only decides *how to
// ask*, which depends on where the legal targets live:
//
//   - battlefield permanents and players are on the canvas → point at them (the targeting arrow)
//   - graveyard/exile cards are collapsed into a single pile card, and stack objects live in the
//     DOM overlay, so neither can be clicked → offer them as a picker instead
//
// A spell whose legal targets are all on the board keeps the arrow; anything else falls back to the
// picker, which can show a target from any zone — and a player.

import type { ActionView, VisibleState, WireTarget } from "~/api/generated";
import { ZONE } from "~/layout";

export type TargetMode =
  /** Takes no target — submit the action as-is. */
  | { kind: "none" }
  /** Wants a target but none is legal right now. Only an activated ability reaches this: the cast
   * gate won't offer a spell with no legal target, but `meaningful_actions` offers abilities
   * without checking (see `ActionView.targets`). */
  | { kind: "impossible" }
  /** Point at the board: `objects` are the legal battlefield permanents, `players` the legal seats. */
  | { kind: "arrow"; objects: ReadonlySet<number>; players: ReadonlySet<number> }
  /** Pick from a list — at least one legal target isn't reachable on the canvas. */
  | { kind: "pick"; targets: WireTarget[] };

/** Whether a target is something the player can physically click on the canvas: a player's life
 * orb, or a permanent on the battlefield. */
export function onBoard(target: WireTarget, state: VisibleState): boolean {
  if (target.kind === "player") return true;
  return state.objects.find((o) => o.id === target.id)?.zone === ZONE.Battlefield;
}

/** How to ask for one target out of `targets`: point at the board, or pick from a list. */
export function askFor(targets: WireTarget[], state: VisibleState): TargetMode {
  if (targets.length === 0) return { kind: "impossible" };
  if (targets.every((t) => onBoard(t, state))) {
    return {
      kind: "arrow",
      objects: new Set(targets.filter((t) => t.kind === "object").map((t) => t.id)),
      players: new Set(targets.filter((t) => t.kind === "player").map((t) => t.player)),
    };
  }
  return { kind: "pick", targets };
}

export function targetMode(action: ActionView, state: VisibleState): TargetMode {
  if (!action.needs_target) return { kind: "none" };
  return askFor(action.targets ?? [], state);
}
