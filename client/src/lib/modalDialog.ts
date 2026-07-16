/** Open a `<dialog>` as a modal after any sibling modal's `close()` in the same flush has settled.
 *
 * Cost-pick → target-pick chains (discard/exile then an off-board cast target) unmount the first
 * `showModal()` dialog and mount the next in one Solid update. A synchronous `showModal()` in
 * `onMount` then races the prior `close()` and the second prompt never appears.
 *
 * Returns a cancel function for the component's `onCleanup` (skip the deferred open if unmounted
 * first; still close if the dialog did open).
 */
export function openModalWhenReady(dialog: HTMLDialogElement): () => void {
  let cancelled = false;
  queueMicrotask(() => {
    if (cancelled || !dialog.isConnected || dialog.open) return;
    dialog.showModal();
  });
  return () => {
    cancelled = true;
    if (dialog.open) dialog.close();
  };
}
