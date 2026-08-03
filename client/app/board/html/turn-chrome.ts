// Turn banner: whose turn, current phase track, priority watch.

import type { Html, HtmlBuilder } from "foldkit/html";
import { cn } from "~/cn";
import { playerLabel } from "~/players";
import { isActivePlayer } from "~/spectator";
import { type Heat, heatOf } from "~/watch";
import type { VisibleState } from "~/wire/types";
import { PHASES, phaseOf, STEP_NAMES } from "../geometry/layout";
import type { Message } from "../messages";
import type { BoardModel } from "../submodel";
import { MountPriorityWatch } from "./audio-mount";

const HEAT_INK: Record<Heat, string> = {
  sage: "text-watch-sage",
  ember: "text-turn-ember",
  flare: "text-watch-flare",
};

function phaseSegment(
  state: "past" | "now" | "future",
  yourTurn: boolean,
  name: string,
  detail: string | null,
  h: HtmlBuilder<Message>,
): Html {
  // Interactive chrome is attribute-driven: JS sets data-phase-state / data-your-turn,
  // Tailwind variants own the look (mint = your turn now, ember = theirs, dim = past/future).
  return h.div(
    [
      h.DataAttribute("phase-state", state),
      h.DataAttribute("your-turn", String(yourTurn)),
      h.Class(
        cn(
          "w-[7.5rem] rounded-control border border-transparent bg-tapped-out/60 px-md py-xs text-center font-semibold text-caption text-phase-fern",
          "data-[phase-state=past]:bg-quiet-hover data-[phase-state=past]:text-snow-mint",
          "data-[phase-state=now]:text-snow-mint",
          "data-[phase-state=now]:data-[your-turn=true]:border-phase-mint data-[phase-state=now]:data-[your-turn=true]:bg-llanowar/90",
          "data-[phase-state=now]:data-[your-turn=false]:border-phase-ember data-[phase-state=now]:data-[your-turn=false]:bg-phase-ember/90",
        ),
      ),
    ],
    [name, detail == null ? null : h.div([h.Class("mt-px text-micro text-snow-mint/85")], [detail])].filter(
      (v): v is Html | string => v !== null,
    ),
  );
}

function priorityWatchView(board: BoardModel, state: VisibleState, h: HtmlBuilder<Message>): Html {
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

export function turnChromeView(board: BoardModel, state: VisibleState, h: HtmlBuilder<Message>): Html {
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
          h.DataAttribute("your-turn", String(yourTurn)),
          h.Class("font-bold text-label data-[your-turn=true]:text-turn-mint data-[your-turn=false]:text-turn-ember"),
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
            h,
          ),
        ),
      ),
      isActivePlayer(state.players, state.viewer) ? priorityWatchView(board, state, h) : null,
    ].filter((v): v is Html => v !== null),
  );
}
