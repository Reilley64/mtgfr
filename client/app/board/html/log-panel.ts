// Game log panel: recent or expanded fold lines in a Hud surface above the hand bar (left column).

import { type Html, html } from "foldkit/html";
import { button } from "~/ui/button";
import type { LogLine } from "../../game/fold";
import { LogCopyRequested, LogExpandToggled, type Message } from "../messages";
import type { BoardModel } from "../submodel";
import { HAND_BAR_H } from "./hand";

const h = html<Message>();

const LOG_VISIBLE = 30;

function lineView(line: LogLine): Html {
  if (line.auto) {
    return h.div(
      [h.Class("flex items-start gap-xs text-caption text-snow-mint")],
      [
        h.span(
          [
            h.Class(
              "mt-px shrink-0 rounded-full bg-auto-moss px-xs py-px font-bold text-micro text-snow-mint tracking-chip",
            ),
          ],
          ["AUTO"],
        ),
        h.span([], [line.text]),
      ],
    );
  }

  return h.div([h.Class("text-caption text-mist")], [line.text]);
}

export function logPanelView(board: BoardModel, log: ReadonlyArray<LogLine>): Html | null {
  if (log.length === 0) return null;

  const lines = board.logExpanded ? log : log.slice(-LOG_VISIBLE);
  // Expand state is attribute-driven: data-expanded flips the height cap, JS sets no class ternary.
  const logClass =
    "pointer-events-auto max-h-[150px] w-[min(300px,46vw)] overflow-y-auto rounded-hud bg-forest-hud p-md text-label leading-normal shadow-hud data-[expanded=true]:max-h-[min(40vh,420px)]";
  const copyLabel = board.logCopied ? "Copied" : board.logCopyFailed ? "Copy failed" : "Copy";

  return h.div(
    [
      h.Class("fixed bottom-(--b) left-md z-20 flex max-w-[min(420px,46vw)] flex-col items-start gap-sm"),
      h.Style({ "--b": `${HAND_BAR_H + 10}px` }),
    ],
    [
      h.div(
        [
          h.DataAttribute("testid", "board-log-toolbar"),
          h.Class("pointer-events-auto flex w-[min(300px,46vw)] items-center justify-between gap-xs"),
        ],
        [
          button(
            h,
            {
              testId: "board-log-expand",
              onClick: LogExpandToggled(),
              variant: "ghost",
              class: "px-2 py-1 text-chip",
              attrs: [h.Attribute("aria-expanded", board.logExpanded ? "true" : "false")],
            },
            [board.logExpanded ? "Collapse" : "Expand"],
          ),
          button(
            h,
            {
              testId: "board-log-copy",
              onClick: LogCopyRequested(),
              variant: "ghost",
              class: "px-2 py-1 text-chip",
              ariaLabel: "Copy game log",
            },
            [copyLabel],
          ),
        ],
      ),
      h.div(
        [
          h.DataAttribute("testid", "board-log"),
          h.DataAttribute("expanded", String(board.logExpanded)),
          h.Role("log"),
          h.Attribute("aria-live", "polite"),
          h.Class(logClass),
        ],
        lines.map(lineView),
      ),
    ],
  );
}
