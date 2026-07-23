// Shared ConfirmDialog view helper — renders a native <dialog> opened with showModal() so the
// browser provides the focus trap, top-layer stacking, and Escape-to-cancel behaviour for free.
// Mirrors the pattern used for the portrait gate (OpenPortraitGateModal in app/view.ts).
//
// Usage: call confirmDialog(h, { ... }) from within a view that has ModalOpened in its Message
// union. The OpenDialogAsModal Mount dispatches ModalOpened (a no-op) when the element mounts;
// h.OnCancel(onCancel) prevents the native Escape-close and instead dispatches the cancel message
// so the model drives visibility, not the browser.

import { Effect } from "effect";
import type { html as createHtml, Html } from "foldkit/html";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";
import { buttonClass } from "./buttonClass";
import { modalClass } from "./surfaces";

/** Dispatched when a modal dialog mounts — handled as a no-op by update. Declare it in every
 *  Message union that hosts a dialog opened with OpenDialogAsModal. */
export const ModalOpened = m("ModalOpened");

/** Opens an HTMLDialogElement as a modal via showModal() when mounted; closes it on unmount. */
export const OpenDialogAsModal = Mount.define(
  "OpenDialogAsModal",
  ModalOpened,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (typeof HTMLDialogElement === "undefined") return null;
        if (!(element instanceof HTMLDialogElement)) return null;
        const handle = { cancelled: false, dialog: element };
        queueMicrotask(() => {
          if (handle.cancelled || !element.isConnected || element.open) return;
          element.showModal();
        });
        return handle;
      }),
      (handle) =>
        Effect.sync(() => {
          if (handle == null) return;
          handle.cancelled = true;
          if (handle.dialog.open) handle.dialog.close();
        }),
    );
    return ModalOpened();
  }),
);

/** Renders a native <dialog> confirm prompt as a showModal modal.
 *
 * Escape: h.OnCancel calls event.preventDefault() (keeping the dialog open) then dispatches
 * onCancel, so the model drives closure — no race between browser-close and re-render.
 * Cancel autofocuses so a destructive confirm is never one Enter away.
 *
 * @param h  The html builder from the calling view (typed to include ModalOpened in Message).
 */
export function confirmDialog<M>(
  h: ReturnType<typeof createHtml<M>>,
  params: {
    title: string;
    body?: string;
    confirmLabel: string;
    danger?: boolean;
    onConfirm: M;
    onCancel: M;
    testId?: string;
  },
): Html {
  const { title, body, confirmLabel, danger = false, onConfirm, onCancel, testId = "confirm-dialog" } = params;

  return h.dialog(
    [
      h.DataAttribute("testid", testId),
      h.Class(`${modalClass()} m-auto backdrop:bg-black/60`),
      h.Attribute("aria-labelledby", `${testId}-title`),
      h.OnMount(OpenDialogAsModal() as never),
      h.OnCancel(onCancel),
    ],
    [
      h.div(
        [h.Class("flex max-w-[380px] flex-col gap-md")],
        [
          h.div(
            [h.Id(`${testId}-title`), h.Class("font-semibold text-body"), h.DataAttribute("testid", "confirm-title")],
            [title],
          ),
          body != null ? h.div([h.Class("text-label text-lichen")], [body]) : null,
          h.div(
            [h.Class("flex justify-end gap-sm")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "confirm-cancel"),
                  h.Autofocus(true),
                  h.OnClick(onCancel),
                  h.Class(buttonClass("ghost")),
                ],
                ["Cancel"],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "confirm-ok"),
                  h.OnClick(onConfirm),
                  h.Class(buttonClass(danger ? "danger" : "primary")),
                ],
                [confirmLabel],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}
