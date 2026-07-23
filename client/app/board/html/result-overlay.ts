// Game result overlay: shown when a player wins, loses, or the game ends.
// Mirrors Solid board-overlays.tsx ResultOverlay.

import { type Html, html } from "foldkit/html";
import { outcome } from "~/outcome";
import { playerLabel } from "~/players";
import { buttonClass } from "~/ui/buttonClass";
import type { VisibleState } from "~/wire/types";
import { LeaveGame, type Message, ResultSeen } from "../messages";

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
 * Result overlay — shown when the game has concluded or the viewer was eliminated.
 * Returns null when the game is still playing or the viewer has already seen the result.
 */
export function resultOverlayView(state: VisibleState, resultSeen: boolean): Html | null {
  const o = outcome(state.players, state.viewer);
  if (o.kind === "playing" || resultSeen) return null;

  return h.div(
    [
      h.DataAttribute("testid", "result-overlay"),
      h.Class("fixed inset-0 z-55 flex items-center justify-center bg-black/70"),
    ],
    [
      h.div(
        [
          h.Class(
            "rounded-panel border border-vine bg-forest-surface p-xl shadow-hud flex max-w-[420px] flex-col items-center gap-lg text-center",
          ),
        ],
        [
          h.div([h.Class("font-bold text-title text-snow")], [headline(state)]),
          h.div([h.Class("text-label text-lichen")], [detail(state)]),
          h.div(
            [h.Class("flex gap-md")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "result-watch"),
                  h.OnClick(ResultSeen()),
                  h.Class(buttonClass("ghost")),
                ],
                [watchLabel(state)],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "result-leave"),
                  h.OnClick(LeaveGame()),
                  h.Class(buttonClass("primary")),
                ],
                ["Back to your decks"],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}
