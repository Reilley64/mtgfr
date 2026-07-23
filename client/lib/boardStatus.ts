// Spoken board summary for AT / live regions. Uses wire `viewer`, not layout seat fallbacks.

import { STEP_NAMES } from "../app/board/geometry/layout";
import { playerLabel } from "./players";
import { SPECTATOR_VIEWER } from "./spectator";
import type { VisibleState } from "./wire/types";

/** One-line status of whose turn, step, priority, and stack depth. */
export function boardStatusSummary(state: VisibleState | null, viewer: number): string {
  if (state == null) return "Commander board. Connecting to the table.";

  const spectating = viewer === SPECTATOR_VIEWER;
  const turn =
    !spectating && state.active_player === viewer
      ? "Your turn"
      : `${playerLabel(state.players, state.active_player)}'s turn`;
  const step = STEP_NAMES[state.step] ?? "unknown step";
  const prio =
    !spectating && state.priority === viewer
      ? "You have priority"
      : `Priority: ${playerLabel(state.players, state.priority)}`;
  const stack = state.stack.length === 0 ? "Stack empty" : `Stack: ${state.stack.length} objects`;
  const lead = spectating ? "Spectating. " : "";
  return `Commander board. ${lead}${turn}, ${step}. ${prio}. ${stack}.`;
}
