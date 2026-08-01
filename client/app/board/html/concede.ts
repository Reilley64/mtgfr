// Concede: top-right ghost button + confirm dialog.
// Conceding is a real game action (CR 104.3a), not navigation.

import type * as Dialog from "@foldkit/ui/dialog";
import type { Html, HtmlBuilder } from "foldkit/html";
import { button } from "~/ui/button";
import { confirmDialog } from "~/ui/confirmDialog";
import { ConcedeClicked, ConcedeConfirmed, GotConcedeDialogMessage, type Message } from "../messages";
import { CONCEDE_DIALOG_ID } from "../submodel";

/** Concede button — fixed top-right, shown while the viewer is still in the game. */
export function concedeButtonView(h: HtmlBuilder<Message>): Html {
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

/** Confirmation dialog for conceding. Always rendered — a closed `<dialog>` is what Dialog opens. */
export function concedeDialogView(model: Dialog.Model, h: HtmlBuilder<Message>): Html {
  return confirmDialog(h, {
    model,
    toDialogMessage: (message) => GotConcedeDialogMessage({ message }),
    title: "Concede the game?",
    body: "You're out for good, and the other players carry on without you.",
    confirmLabel: "Concede",
    danger: true,
    onConfirm: ConcedeConfirmed(),
    testId: CONCEDE_DIALOG_ID,
  });
}
