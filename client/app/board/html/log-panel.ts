// Game log panel: last 30 fold lines in a Hud surface above the hand bar (left column).

import { type Html, html } from "foldkit/html";
import type { LogLine } from "../../game/fold";
import type { Message } from "../messages";
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

export function logPanelView(log: ReadonlyArray<LogLine>): Html | null {
  if (log.length === 0) return null;

  const lines = log.slice(-LOG_VISIBLE);

  return h.div(
    [
      h.Class("fixed bottom-(--b) left-md z-20 flex max-w-[min(420px,46vw)] flex-col items-start gap-sm"),
      h.Style({ "--b": `${HAND_BAR_H + 10}px` }),
    ],
    [
      h.div(
        [
          h.DataAttribute("testid", "board-log"),
          h.Role("log"),
          h.Attribute("aria-live", "polite"),
          h.Class(
            "max-h-[150px] w-[min(300px,46vw)] overflow-y-auto rounded-hud bg-forest-hud p-md text-label leading-normal shadow-hud",
          ),
        ],
        lines.map(lineView),
      ),
    ],
  );
}
