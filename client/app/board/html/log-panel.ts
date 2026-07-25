// Game log panel: recent or expanded fold lines in a Hud surface above the hand bar (left column).

import { type Html, html } from "foldkit/html";
import { buttonClass } from "~/ui/buttonClass";
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
              "mt-px shrink-0 rounded-full bg-auto-moss px-xs py-px font-bold text-micro text-snow-mint tracking-[0.06em]",
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
  const logClass = board.logExpanded
    ? "pointer-events-auto max-h-[min(40vh,420px)] w-[min(300px,46vw)] overflow-y-auto rounded-hud bg-forest-hud p-md text-label leading-normal shadow-hud"
    : "pointer-events-auto max-h-[150px] w-[min(300px,46vw)] overflow-y-auto rounded-hud bg-forest-hud p-md text-label leading-normal shadow-hud";
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
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "board-log-expand"),
              h.OnClick(LogExpandToggled()),
              h.Class(buttonClass("ghost", "px-2 py-1 text-chip")),
              h.Attribute("aria-expanded", board.logExpanded ? "true" : "false"),
            ],
            [board.logExpanded ? "Collapse" : "Expand"],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "board-log-copy"),
              h.OnClick(LogCopyRequested()),
              h.Class(buttonClass("ghost", "px-2 py-1 text-chip")),
              h.Attribute("aria-label", "Copy game log"),
            ],
            [copyLabel],
          ),
        ],
      ),
      h.div(
        [h.DataAttribute("testid", "board-log"), h.Role("log"), h.Attribute("aria-live", "polite"), h.Class(logClass)],
        lines.map(lineView),
      ),
    ],
  );
}
