// Confirm-prompt dialog. Behaviour (open/close, focus trap, Escape, scroll lock, backdrop click)
// comes from @foldkit/ui's Dialog; markup and classes come from here.
//
// Dialog is a submodel, not a pure view: the owner holds a `Dialog.Model`, delegates to
// `Dialog.update`, and maps its `Closed` OutMessage to its own cancel message. Escape, a backdrop
// click, and the Cancel button all take that one path, so there is no `onCancel` prop — only
// Confirm carries a parent message.
//
// This stays a wrapper function rather than a `Submodel.defineView` because `onConfirm` is the
// *parent's* message: a message dispatched inside the child's boundary would be wrapped by
// `toParentMessage`. `h.submodel` auto-wraps top-level `viewInputs` functions to the parent's
// boundary, so building the panel inside `toView` — which is what this does — is the path that
// lets a parent message dispatch unwrapped.

import * as Dialog from "@foldkit/ui/dialog";
import type { html as createHtml, Html } from "foldkit/html";
import { button } from "./button";
import { modalClass } from "./surfaces";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

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
 * `initialFocus` marker, so a destructive confirm is never one Enter away. `render.isVisible`
 * gates the backdrop and panel: a closed dialog renders an empty `<dialog>`, which has to stay
 * in the DOM for Dialog to open and close it.
 */
export function confirmDialog<Msg>(h: HtmlFactory<Msg>, props: ConfirmDialogProps<Msg>): Html {
  const { model, toDialogMessage, title, body, confirmLabel, danger = false, onConfirm } = props;
  const testId = props.testId ?? "confirm-dialog";

  return h.submodel({
    slotId: testId,
    model,
    view: Dialog.view,
    viewInputs: {
      toView: (render) =>
        h.dialog(
          // Dialog styles the element itself as a full-viewport transparent layer, so this only
          // adds centring and the test hook.
          [...render.dialog, h.DataAttribute("testid", testId), h.Class("flex items-center justify-center")],
          render.isVisible
            ? [
                h.div(
                  [
                    ...render.backdrop,
                    h.DataAttribute("testid", "confirm-backdrop"),
                    h.Class("fixed inset-0 bg-black/60"),
                  ],
                  [],
                ),
                h.div(
                  // `relative` puts the panel above the fixed backdrop without a z-index race.
                  [...render.panel, h.Class(modalClass("relative flex max-w-[380px] flex-col gap-md"))],
                  [
                    h.div(
                      [...render.title, h.Class("font-semibold text-body"), h.DataAttribute("testid", "confirm-title")],
                      [title],
                    ),
                    body != null ? h.div([...render.description, h.Class("text-label text-lichen")], [body]) : null,
                    h.div(
                      [h.Class("flex justify-end gap-sm")],
                      [
                        button(
                          h,
                          {
                            testId: "confirm-cancel",
                            variant: "ghost",
                            attrs: [...render.closeButton, ...render.initialFocus],
                          },
                          ["Cancel"],
                        ),
                        button(
                          h,
                          { testId: "confirm-ok", onClick: onConfirm, variant: danger ? "danger" : "primary" },
                          [confirmLabel],
                        ),
                      ],
                    ),
                  ],
                ),
              ]
            : [],
        ),
    },
    toParentMessage: toDialogMessage,
  }) as Html;
}
