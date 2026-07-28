// Native <dialog> opened with showModal(), so the browser supplies the focus trap, top-layer
// stacking, and Escape-to-cancel.
//
// ponytail: the builder's print picker is the last surface on this; confirm prompts moved to
// @foldkit/ui's Dialog submodel (see confirmDialog.ts). Move the picker over too when it needs
// anything this doesn't give — animation, scroll lock, or a managed close — and delete this file.

import { Effect } from "effect";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";

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
