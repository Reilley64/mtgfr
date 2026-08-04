// Confirm prompt — a question and two choices in `modalDialog`'s frame. The modal behaviour
// (open/close, focus trap, Escape, scroll lock, backdrop click) is Dialog's; the frame is
// `dialog.ts`'s; the question, the two buttons, and which one takes focus are here.
//
// Dialog is a submodel, not a pure view: the owner holds a `Dialog.Model`, delegates to
// `Dialog.update`, and maps its `Closed` OutMessage to its own cancel message. Escape, a backdrop
// click, and the Cancel button all take that one path, so there is no `onCancel` prop — only
// Confirm carries a parent message.

import type * as Dialog from "@foldkit/ui/dialog";
import type { Html, HtmlBuilder } from "foldkit/html";
import { button } from "./button";
import { modalDialog } from "./dialog";

export type ConfirmDialogProps<Msg> = {
  /** The owner's dialog state. Create with `Dialog.init({ id })`; drive with `Dialog.update`. */
  model: Dialog.Model;
  /** Lifts a `Dialog.Message` into the owner's message union. */
  toDialogMessage: (message: Dialog.Message) => Msg;
  title: string;
  body?: string;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: Msg;
  testId?: string;
};

/** Renders a confirm prompt over a `Dialog` submodel.
 *
 * Cancel spreads Dialog's `closeButton` (so a plain dismiss needs no parent message) and its
 * `initialFocus` marker, so a destructive confirm is never one Enter away.
 */
export function confirmDialog<Msg>(h: HtmlBuilder<Msg>, props: ConfirmDialogProps<Msg>): Html {
  const { model, toDialogMessage, title, body, confirmLabel, danger = false, onConfirm } = props;

  return modalDialog(
    h,
    {
      model,
      toDialogMessage,
      panel: "max-w-[380px]",
      testId: props.testId ?? "confirm-dialog",
      backdropTestId: "confirm-backdrop",
    },
    (render) => [
      h.div([...render.title, h.Class("font-semibold text-body"), h.DataAttribute("testid", "confirm-title")], [title]),
      body != null ? h.div([...render.description, h.Class("text-label text-lichen")], [body]) : null,
      h.div(
        [h.Class("flex justify-end gap-sm")],
        [
          button(
            h,
            { testId: "confirm-cancel", variant: "ghost", attrs: [...render.closeButton, ...render.initialFocus] },
            ["Cancel"],
          ),
          button(h, { testId: "confirm-ok", onClick: onConfirm, variant: danger ? "danger" : "primary" }, [
            confirmLabel,
          ]),
        ],
      ),
    ],
  );
}
