// Concede: top-right ghost button + confirm dialog.
// Conceding is a real game action (CR 104.3a), not navigation.

import { type Html, html } from "foldkit/html";
import { cn } from "~/cn";
import { buttonClass } from "~/ui/buttonClass";
import { ConcedeCancelled, ConcedeClicked, ConcedeConfirmed, type Message } from "../messages";

const h = html<Message>();

/** Concede button — fixed top-right, shown while the viewer is still in the game. */
export function concedeButtonView(): Html {
  return h.button(
    [
      h.Type("button"),
      h.DataAttribute("testid", "board-concede"),
      h.OnClick(ConcedeClicked()),
      h.Class(cn("pointer-events-auto fixed top-md right-md z-45", buttonClass("ghost"))),
    ],
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
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "concede-cancel"),
                  h.OnClick(ConcedeCancelled()),
                  h.Class(buttonClass("ghost")),
                ],
                ["Cancel"],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "concede-confirm"),
                  h.OnClick(ConcedeConfirmed()),
                  h.Class(buttonClass("danger")),
                ],
                ["Concede"],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}
