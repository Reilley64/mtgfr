import { describe, expect, it, vi } from "vitest";
import { openModalWhenReady } from "~/lib/modalDialog";

function fakeDialog(): HTMLDialogElement {
  let open = false;
  return {
    get open() {
      return open;
    },
    get isConnected() {
      return true;
    },
    showModal() {
      if (open) throw new Error("showModal on an already-open dialog");
      open = true;
    },
    close() {
      open = false;
    },
  } as HTMLDialogElement;
}

describe("openModalWhenReady", () => {
  it("defers showModal to a microtask so a prior dialog close can settle", async () => {
    const dialog = fakeDialog();
    const show = vi.spyOn(dialog, "showModal");
    openModalWhenReady(dialog);
    expect(show).not.toHaveBeenCalled();
    await Promise.resolve();
    expect(show).toHaveBeenCalledOnce();
    expect(dialog.open).toBe(true);
  });

  it("skips showModal when cancelled before the microtask (chained unmount)", async () => {
    const dialog = fakeDialog();
    const show = vi.spyOn(dialog, "showModal");
    const cancel = openModalWhenReady(dialog);
    cancel();
    await Promise.resolve();
    expect(show).not.toHaveBeenCalled();
    expect(dialog.open).toBe(false);
  });

  it("closes an already-open dialog on cancel", async () => {
    const dialog = fakeDialog();
    const cancel = openModalWhenReady(dialog);
    await Promise.resolve();
    expect(dialog.open).toBe(true);
    cancel();
    expect(dialog.open).toBe(false);
  });
});
