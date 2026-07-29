// CR 103.1 reveal: a one-shot spotlight hopping the seat quadrants onto the rolled starter.
import { type Html, html } from "foldkit/html";
import type { VisibleState } from "~/wire/types";
import { seatCell, seatColor, seatSlot } from "../geometry/layout";
import type { Message } from "../messages";
import type { FirstPlayerReveal } from "../submodel";

const h = html<Message>();

function seatLabel(state: VisibleState, seat: number): string {
  return state.players.find((p) => p.player === seat)?.username ?? `Seat ${seat + 1}`;
}

export function firstPlayerRevealView(reveal: FirstPlayerReveal | null, state: VisibleState): Html | null {
  if (reveal == null) return null;

  const count = Math.max(1, state.players.length);
  const litSlot = reveal.steps[reveal.index]?.slot ?? 0;
  const winnerSlot = seatSlot(reveal.winner, state.viewer, count);
  const settled = reveal.index === reveal.steps.length - 1;

  return h.div(
    [
      h.DataAttribute("testid", "first-player-reveal"),
      h.Class(
        "pointer-events-auto fixed inset-0 z-50 flex flex-col items-center justify-center gap-md bg-black/80 px-md py-lg text-snow",
      ),
    ],
    [
      h.div([h.Class("text-label uppercase tracking-[0.08em] text-mist")], ["Rolling for the first turn"]),
      h.div(
        [h.Class("grid w-[min(70vw,420px)] grid-cols-2 grid-rows-2 gap-sm")],
        state.players.map((player) => {
          const slot = seatSlot(player.player, state.viewer, count);
          const cell = seatCell(player.player, state.viewer, count);
          const lit = slot === litSlot;
          return h.div(
            [
              h.DataAttribute("testid", `reveal-seat-${player.player}`),
              h.DataAttribute("lit", lit ? "true" : "false"),
              h.DataAttribute("winner", slot === winnerSlot ? "true" : "false"),
              h.Style({
                "--col": String(cell.col + 1),
                "--row": String(cell.row + 1),
                "--seat": seatColor(player.player, 1),
              }),
              h.Class(
                "col-start-(--col) row-start-(--row) flex items-center justify-center rounded-hud border-2 px-md py-md text-chip transition-all duration-100 " +
                  "border-[color:var(--seat)] data-[lit=false]:opacity-35 data-[lit=true]:bg-[color:var(--seat)] " +
                  "data-[lit=true]:scale-105 data-[lit=true]:text-forest-hud data-[winner=true]:data-[lit=true]:scale-110",
              ),
            ],
            [seatLabel(state, player.player)],
          );
        }),
      ),
      h.div(
        [
          h.DataAttribute("testid", "reveal-winner"),
          h.Class("h-6 text-caption text-seafoam transition-opacity duration-200"),
        ],
        [settled ? `${seatLabel(state, reveal.winner)} goes first` : ""],
      ),
    ],
  );
}
