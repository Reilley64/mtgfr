// The shared "are you sure?" prompt, on the native `<dialog>` element: `showModal()` already gives
// us the focus trap, Escape-to-dismiss, an inert background and top-layer stacking, so none of that
// is reimplemented here. Cancel takes the initial focus — a destructive action should never be one
// stray Enter away.

import { createEffect, onCleanup, Show } from "solid-js";
import { Button } from "~/components/atoms";
import { cn } from "~/lib/cn";
import { openModalWhenReady } from "~/lib/modalDialog";

export default function ConfirmDialog(props: {
  open: boolean;
  title: string;
  body?: string;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  let dialog!: HTMLDialogElement;

  // `open` is the source of truth; the element follows it. Opening is deferred so a prior modal's
  // `close()` in the same Solid flush can settle (same race PickDialog guards against).
  createEffect(() => {
    if (!props.open) {
      if (dialog.open) dialog.close();
      return;
    }
    if (dialog.open) return;
    onCleanup(openModalWhenReady(dialog));
  });

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: this backdrop click's keyboard equivalent is Escape, which showModal() wires up natively and delivers to onClose below.
    <dialog
      ref={dialog}
      data-testid="confirm-dialog"
      // Escape and backdrop dismissal both land here. The effect's own `close()` does too, but by
      // then `props.open` is already false, so the guard keeps that from re-firing onCancel.
      onClose={() => props.open && props.onCancel()}
      // A native dialog's backdrop is painted by the element itself, so a click landing on the
      // <dialog> rather than the panel inside it is a backdrop click.
      onClick={(e) => e.target === dialog && props.onCancel()}
      class={cn(
        "rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table",
        "m-auto backdrop:bg-black/60",
      )}
    >
      <div class="flex max-w-[380px] flex-col gap-md">
        <div class="font-semibold text-body" data-testid="confirm-title">
          {props.title}
        </div>
        <Show when={props.body}>
          <div class="text-label text-lichen">{props.body}</div>
        </Show>
        <div class="flex justify-end gap-sm">
          <Button type="button" data-testid="confirm-cancel" autofocus onClick={props.onCancel} variant="ghost">
            Cancel
          </Button>
          <Button
            type="button"
            data-testid="confirm-ok"
            onClick={props.onConfirm}
            variant={props.danger ? "danger" : "primary"}
          >
            {props.confirmLabel}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
