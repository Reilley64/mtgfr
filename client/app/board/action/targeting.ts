// How the board asks for a staged action's target.
//
// The engine already enumerates what's legal (`Game::legal_targets`, on the wire as
// `ActionView.targets`), so nothing here re-derives `TargetSpec` — this module only decides *how
// to ask*, which depends on where the legal targets live:
//
//   - battlefield permanents and players are on the canvas → point at them (arrow)
//   - graveyard/exile cards are collapsed into a single pile card, and stack objects live in the
//     DOM overlay, so neither can be clicked → offer them as a picker instead

import { colors } from "~/design-tokens.generated";
import type { ActionView, VisibleState, WireTarget } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import type { StagedAction } from "./execution";

/** Shared target-arrow / staged-preview accent (canvas stroke + DOM ring). */
export const TARGET_COLOR = colors.islandBlue;

export type Vec = { x: number; y: number };

export type StagingOverlay = {
  aiming: boolean;
  targetObjects: ReadonlySet<number>;
  targetPlayers: ReadonlySet<number>;
  aimFrom: Vec | null;
};

// Stack overlay geometry — one source for the DOM overlay and the canvas aim origin.
const STACK_CARD_W = 180;
const STACK_OVERLAY_RIGHT = 16;
const STACK_PEEK = 34;
const STACK_ANCHOR_FROM_RIGHT = STACK_OVERLAY_RIGHT + STACK_CARD_W / 2;

function stackCardH(cardW = STACK_CARD_W): number {
  return cardW / 0.716;
}

/** Screen-space center of the top card in a right-edge pile of `count` cards. */
export function stackAimOrigin(viewportW: number, viewportH: number, count: number, peek = STACK_PEEK): Vec {
  const n = Math.max(1, count);
  const cardH = stackCardH();
  const pileH = cardH + (n - 1) * peek;
  return {
    x: viewportW - STACK_ANCHOR_FROM_RIGHT,
    y: viewportH / 2 + pileH / 2 - (n - 1) * peek - cardH / 2,
  };
}

export function stagingOverlay(
  staged: StagedAction | null,
  state: VisibleState,
  viewport: { width: number; height: number },
  stackLen: number,
): StagingOverlay {
  const idle: StagingOverlay = {
    aiming: false,
    targetObjects: new Set(),
    targetPlayers: new Set(),
    aimFrom: null,
  };
  if (staged == null) return idle;

  const mode = targetMode(staged.action, state);
  if (mode.kind !== "arrow" || staged.preferPick) return idle;

  return {
    aiming: true,
    targetObjects: mode.objects,
    targetPlayers: mode.players,
    aimFrom: stackAimOrigin(viewport.width, viewport.height, stackLen + 1),
  };
}

export type TargetMode =
  | { kind: "none" }
  | { kind: "impossible" }
  | { kind: "arrow"; objects: ReadonlySet<number>; players: ReadonlySet<number> }
  | { kind: "pick"; targets: WireTarget[] };

export function onBoard(target: WireTarget, state: VisibleState): boolean {
  if (target.kind === "player") return true;
  return state.objects.find((o) => o.id === target.id)?.zone === ZONE.Battlefield;
}

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

/** Legal targets for the staged-action picker, or null when the arrow should ask instead. */
export function stagedPickTargets(staged: StagedAction, state: VisibleState): WireTarget[] | null {
  const mode = targetMode(staged.action, state);
  if (mode.kind === "none" || mode.kind === "impossible") return null;
  if (mode.kind === "pick") return mode.targets;
  if (staged.preferPick && mode.kind === "arrow") {
    return [
      ...[...mode.objects].map((id) => ({ kind: "object" as const, id })),
      ...[...mode.players].map((player) => ({ kind: "player" as const, player })),
    ];
  }
  return null;
}

export function objectName(state: VisibleState, id: number): string {
  return state.objects.find((o) => o.id === id)?.name ?? `#${id}`;
}

export function playerSeatLabel(state: VisibleState, seat: number): string {
  const name = state.players.find((p) => p.player === seat)?.username?.trim();
  return name || `P${seat}`;
}

/** Title while the player is aiming a staged cast or activation before submitting. */
export function stagedTargetTitle(staged: StagedAction): string {
  const { card, action } = staged;
  if (action.kind === "activate" && action.label !== card.name) {
    return `${action.label} — ${card.name}`;
  }
  return action.label;
}
