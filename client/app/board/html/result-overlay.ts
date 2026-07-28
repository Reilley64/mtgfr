// Game result overlay: shown when a player wins, loses, or the game ends.
// Mirrors Solid board-overlays.tsx ResultOverlay.

import type * as Dialog from "@foldkit/ui/dialog";
import { type Html, html } from "foldkit/html";
import { outcome } from "~/outcome";
import { playerLabel } from "~/players";
import { button } from "~/ui/button";
import { modalDialog } from "~/ui/dialog";
import type { VisibleState } from "~/wire/types";
import { GotResultDialogMessage, LeaveGame, type Message } from "../messages";
import { RESULT_DIALOG_ID } from "../submodel";

const h = html<Message>();

function headline(state: VisibleState): string {
  const o = outcome(state.players, state.viewer);
  switch (o.kind) {
    case "won":
      return "You win";
    case "lost":
      return o.winner === null ? "You're eliminated" : `${playerLabel([...state.players], o.winner)} wins`;
    case "over":
      return o.winner === null ? "Nobody wins" : `${playerLabel([...state.players], o.winner)} wins`;
    case "playing":
      return "";
  }
}

function detail(state: VisibleState): string {
  const o = outcome(state.players, state.viewer);
  switch (o.kind) {
    case "won":
      return "Last player standing.";
    case "lost":
      return o.winner === null ? "The game continues without you." : "You were eliminated.";
    case "over":
      return "The game is over.";
    case "playing":
      return "";
  }
}

function watchLabel(state: VisibleState): string {
  const o = outcome(state.players, state.viewer);
  return o.kind === "lost" && o.winner === null ? "Keep watching" : "Stay on the board";
}

/**
 * Result overlay — raised once the game has concluded or the viewer was eliminated
 * (`raiseResultDialog` on the fold that ends it). Always rendered: a closed `<dialog>` is what
 * Dialog opens. Staying on the board is the dismiss, so it takes Dialog's close path.
 */
export function resultOverlayView(state: VisibleState, model: Dialog.Model): Html {
  return modalDialog(
    h,
    {
      model,
      toDialogMessage: (message) => GotResultDialogMessage({ message }),
      panel: "max-w-[420px] items-center text-center",
      testId: RESULT_DIALOG_ID,
    },
    (render) => [
      h.div([...render.title, h.Class("font-bold text-title text-snow")], [headline(state)]),
      h.div([...render.description, h.Class("text-label text-lichen")], [detail(state)]),
      h.div(
        [h.Class("flex gap-md")],
        [
          button(
            h,
            {
              testId: "result-watch",
              variant: "ghost",
              attrs: [...render.closeButton, ...render.initialFocus],
            },
            [watchLabel(state)],
          ),
          button(h, { testId: "result-leave", onClick: LeaveGame(), variant: "primary" }, ["Back to your decks"]),
        ],
      ),
    ],
  );
}
