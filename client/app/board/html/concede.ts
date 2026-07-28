// Concede: top-right ghost button + confirm dialog.
// Conceding is a real game action (CR 104.3a), not navigation.

import { type Html, html } from "foldkit/html";
import { button } from "~/ui/button";
import { ConcedeCancelled, ConcedeClicked, ConcedeConfirmed, type Message } from "../messages";

const h = html<Message>();

/** Concede button — fixed top-right, shown while the viewer is still in the game. */
export function concedeButtonView(): Html {
  return button(
    h,
    {
      testId: "board-concede",
      onClick: ConcedeClicked(),
      variant: "ghost",
      class: "pointer-events-auto fixed top-md right-md z-45",
    },
    ["Concede"],
  );
}

/** Confirmation dialog shown when confirmConcede is true. */
export function concedeDialogView(open: boolean): Html | null {
  if (!open) return null;

  return h.div(
    [
      h.DataAttribute("testid", "concede-dialog"),
      h.Class("fixed inset-0 z-50 flex items-center justify-center bg-black/60"),
      h.OnClick(ConcedeCancelled()),
    ],
    [
      h.div(
        [
          h.Class(
            "pointer-events-auto rounded-panel border border-vine bg-forest-surface p-xl shadow-hud flex max-w-[380px] flex-col gap-lg",
          ),
          // Prevent clicks inside the dialog from closing it via the backdrop handler.
          h.Attribute("data-concede-modal", "true"),
        ],
        [
          h.div([h.Class("font-bold text-body text-snow")], ["Concede the game?"]),
          h.div(
            [h.Class("text-label text-lichen")],
            ["You're out for good, and the other players carry on without you."],
          ),
          h.div(
            [h.Class("flex justify-end gap-md")],
            [
              button(h, { testId: "concede-cancel", onClick: ConcedeCancelled(), variant: "ghost" }, ["Cancel"]),
              button(h, { testId: "concede-confirm", onClick: ConcedeConfirmed(), variant: "danger" }, ["Concede"]),
            ],
          ),
        ],
      ),
    ],
  );
}
