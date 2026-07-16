/** Portrait phones: native modal (DESIGN.md Landscape Rule). Escape is swallowed — rotate to dismiss. */

import { onCleanup, onMount } from "solid-js";
import { openModalWhenReady } from "~/lib/modalDialog";

export function PortraitGate() {
  let dialog!: HTMLDialogElement;
  let cancelOpen: (() => void) | undefined;

  onMount(() => {
    const mq = window.matchMedia("(orientation: portrait) and (max-width: 900px)");
    const sync = () => {
      cancelOpen?.();
      cancelOpen = undefined;
      if (mq.matches) {
        if (dialog.open) return;
        cancelOpen = openModalWhenReady(dialog);
        return;
      }
      if (dialog.open) dialog.close();
    };
    sync();
    mq.addEventListener("change", sync);
    onCleanup(() => {
      cancelOpen?.();
      mq.removeEventListener("change", sync);
    });
  });

  return (
    <dialog
      ref={dialog}
      class="portrait-gate bg-forest-floor font-sans text-body text-snow"
      aria-labelledby="portrait-gate-title"
      onCancel={(e) => e.preventDefault()}
    >
      <div id="portrait-gate-title" class="text-title">
        Rotate to landscape
      </div>
      <div class="max-w-[28ch] text-label text-lichen">
        The table and deck builder are built for horizontal screens. Turn your device sideways to continue.
      </div>
    </dialog>
  );
}
