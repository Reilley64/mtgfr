// Modal chrome — the `<dialog>` element, the dimmed backdrop, and the panel. Behaviour (open/close,
// focus trap, focus restore, Escape, page scroll lock, backdrop click) comes from @foldkit/ui's
// Dialog; markup and classes come from here.
//
// Dialog is a submodel, not a pure view: the owner holds a `Dialog.Model`, delegates to
// `Dialog.update`, and acts on its `Closed` OutMessage. Escape, a backdrop click, and a spread of
// `render.closeButton` all take that one path, so there is no `onDismiss` prop.
//
// The panel's contents are entirely the caller's — headings, buttons, and grids differ per modal,
// and only the frame is shared. `children` is a function of Dialog's `RenderInfo` so a caller can
// spread `title`, `description`, `closeButton`, and `initialFocus` onto its own elements. Building
// the body inside `toView` is also what lets a caller's parent message dispatch unwrapped:
// `h.submodel` wraps top-level `viewInputs` functions to the parent's boundary, but not what
// `toView` returns.

import * as Dialog from "@foldkit/ui/dialog";
import type { html as createHtml, Html } from "foldkit/html";
import { modalClass } from "./surfaces";

type HtmlFactory<Msg> = ReturnType<typeof createHtml<Msg>>;

export type ModalDialogProps<Msg> = {
  /** The owner's dialog state. Create with `Dialog.init({ id })`; drive with `Dialog.update`. */
  model: Dialog.Model;
  /** Lifts a `Dialog.Message` into the owner's message union. */
  toDialogMessage: (message: Dialog.Message) => Msg;
  /** Extra classes on the panel — sizing and inner layout. */
  panel?: string;
  testId: string;
  /** Defaults to `<testId>-backdrop`. */
  backdropTestId?: string;
};

/** Renders a modal over a `Dialog` submodel, with `children` as its panel contents.
 *
 * `render.isVisible` gates the backdrop and panel: a closed dialog renders an empty `<dialog>`,
 * which has to stay in the DOM for Dialog to open and close it.
 */
export function modalDialog<Msg>(
  h: HtmlFactory<Msg>,
  props: ModalDialogProps<Msg>,
  children: (render: Dialog.RenderInfo) => ReadonlyArray<Html>,
): Html {
  const { model, toDialogMessage, panel, testId } = props;
  const backdropTestId = props.backdropTestId ?? `${testId}-backdrop`;

  return h.submodel({
    slotId: testId,
    model,
    view: Dialog.view,
    viewInputs: {
      toView: (render) =>
        h.dialog(
          // Dialog styles the element itself as a full-viewport transparent layer, so this only
          // adds centring and the test hook. `pointer-events-auto` is for owners whose root is
          // `pointer-events-none` (the board overlays); a modal is always meant to take clicks.
          //
          // Centring is gated on `isVisible`: a closed `<dialog>` is hidden only by the UA rule
          // `dialog:not([open]) { display: none }`, and `flex` overrides it. Left on, every closed
          // modal stays a full-viewport `pointer-events-auto` layer that swallows clicks on the
          // page behind it.
          [
            ...render.dialog,
            h.DataAttribute("testid", testId),
            h.Class(render.isVisible ? "pointer-events-auto flex items-center justify-center" : "pointer-events-auto"),
          ],
          render.isVisible
            ? [
                h.div(
                  [...render.backdrop, h.DataAttribute("testid", backdropTestId), h.Class("fixed inset-0 bg-black/60")],
                  [],
                ),
                h.div(
                  // `relative` puts the panel above the fixed backdrop without a z-index race.
                  [...render.panel, h.Class(modalClass("relative flex flex-col gap-md", panel))],
                  children(render),
                ),
              ]
            : [],
        ),
    },
    toParentMessage: toDialogMessage,
  }) as Html;
}
