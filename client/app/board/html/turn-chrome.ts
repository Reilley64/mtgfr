// Turn banner: whose turn, current phase track, priority watch.

import { type Html, html } from "foldkit/html";
import { cn } from "~/cn";
import { playerLabel } from "~/players";
import { isActivePlayer } from "~/spectator";
import { type Heat, heatOf } from "~/watch";
import type { VisibleState } from "~/wire/types";
import { PHASES, phaseOf, STEP_NAMES } from "../geometry/layout";
import type { Message } from "../messages";
import type { BoardModel } from "../submodel";
import { MountPriorityWatch } from "./audio-mount";

const h = html<Message>();

const HEAT_INK: Record<Heat, string> = {
  sage: "text-watch-sage",
  ember: "text-turn-ember",
  flare: "text-watch-flare",
};

function phaseSegment(state: "past" | "now" | "future", yourTurn: boolean, name: string, detail: string | null): Html {
  const bg =
    state === "now"
      ? yourTurn
        ? "border-phase-mint bg-llanowar/90 text-snow-mint"
        : "border-phase-ember bg-phase-ember/90 text-snow-mint"
      : state === "past"
        ? "bg-quiet-hover text-snow-mint"
        : "bg-tapped-out/60 text-phase-fern";
  return h.div(
    [
      h.Class(
        `w-[7.5rem] rounded-control border border-transparent px-md py-xs text-center font-semibold text-caption ${bg}`,
      ),
    ],
    [name, detail == null ? null : h.div([h.Class("mt-px text-micro text-snow-mint/85")], [detail])].filter(
      (v): v is Html | string => v !== null,
    ),
  );
}

function priorityWatchView(board: BoardModel, state: VisibleState): Html {
  const holder = state.priority;
  const yours = holder === state.viewer;
  const elapsed = board.priorityElapsed;
  const heat = heatOf(elapsed);

  return h.div(
    [
      h.DataAttribute("priority", String(holder)),
      h.OnMount(MountPriorityWatch()),
      h.Class(cn("font-semibold text-caption tracking-[0.01em]", HEAT_INK[heat], yours && "text-turn-mint")),
    ],
    [
      yours ? "You have priority" : `Waiting on ${playerLabel(state.players, holder)}`,
      elapsed >= 10 ? h.span([h.Class("text-fog")], [` · ${elapsed}s`]) : null,
    ].filter((v): v is Html | string => v !== null),
  );
}

export function turnChromeView(board: BoardModel, state: VisibleState): Html {
  const yourTurn = state.active_player === state.viewer;
  const current = phaseOf(state.step);
  const currentBand = PHASES[current];
  const stepName = STEP_NAMES[state.step] ?? String(state.step);
  const detail = currentBand && currentBand.steps.length > 1 && currentBand.name !== stepName ? stepName : null;

  return h.div(
    [
      h.DataAttribute("testid", "board-turn-banner"),
      h.DataAttribute("step", String(state.step)),
      h.DataAttribute("active-player", String(state.active_player)),
      h.DataAttribute("priority", String(state.priority)),
      h.DataAttribute("stack-len", String(state.stack.length)),
      h.Class(
        "pointer-events-none fixed top-md left-1/2 z-20 flex -translate-x-1/2 flex-col items-center gap-xs rounded-hud bg-forest-hud p-md text-label text-seafoam leading-normal shadow-hud",
      ),
    ],
    [
      h.div(
        [
          h.DataAttribute("testid", "board-turn-label"),
          h.Class(`font-bold text-label ${yourTurn ? "text-turn-mint" : "text-turn-ember"}`),
        ],
        [yourTurn ? "Your turn" : `${playerLabel(state.players, state.active_player)}'s turn`],
      ),
      h.div(
        [h.Class("flex gap-xs")],
        PHASES.map((band, i) =>
          phaseSegment(
            i < current ? "past" : i === current ? "now" : "future",
            yourTurn,
            band.name,
            i === current ? detail : null,
          ),
        ),
      ),
      isActivePlayer(state.players, state.viewer) ? priorityWatchView(board, state) : null,
    ].filter((v): v is Html => v !== null),
  );
}
