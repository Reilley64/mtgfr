// Passive banner while another seat answers engine `pending_choice`.

import { type Html, html } from "foldkit/html";
import { pendingChoiceWaitingText } from "~/choiceWaiting";
import type { VisibleState } from "~/wire/types";
import type { Message } from "../messages";

const h = html<Message>();

/** Non-interactive status for non-deciders / spectators. Null when local seat is the decider. */
export function pendingChoiceWaitingView(state: VisibleState): Html | null {
  const text = pendingChoiceWaitingText({
    pendingPlayer: state.pending_choice?.player ?? null,
    viewer: state.viewer,
    mulliganing: state.mulliganing,
    players: state.players,
  });
  if (text == null) return null;
  return h.div(
    [
      h.DataAttribute("testid", "pending-choice-waiting"),
      h.Class(
        // top-12 clears the spectating badge when both are visible
        "pointer-events-none fixed top-12 left-1/2 z-30 -translate-x-1/2 rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-chip text-seafoam shadow-hud",
      ),
    ],
    [text],
  );
}
