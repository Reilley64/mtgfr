import { type Html, html } from "foldkit/html";
import type { Message } from "../messages";

const h = html<Message>();

export function promptModalFrame(options: { testId: string; title: string; body: Html[]; actions: Html[] }): Html {
  return h.div(
    [
      h.DataAttribute("testid", options.testId),
      h.Class("fixed inset-0 z-40 flex items-center justify-center bg-black/45 px-md py-lg"),
    ],
    [
      h.div(
        [
          h.Class(
            "pointer-events-auto flex max-h-[min(90vh,720px)] max-w-[min(92vw,720px)] flex-col gap-3 overflow-hidden rounded-hud border border-vine/50 bg-forest-hud px-md py-sm text-snow shadow-hud",
          ),
        ],
        [
          h.div(
            [h.DataAttribute("testid", "prompt-modal-title"), h.Class("shrink-0 text-center font-semibold text-body")],
            [options.title],
          ),
          h.div([h.Class("flex min-h-0 flex-1 flex-col gap-2")], options.body),
          options.actions.length === 0
            ? null
            : h.div([h.Class("flex shrink-0 flex-wrap items-center justify-center gap-2")], options.actions),
        ].filter((v): v is Html => v !== null),
      ),
    ],
  );
}
