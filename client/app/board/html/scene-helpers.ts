import { Scene } from "foldkit/test";
import { MountBitmapLayer } from "../bitmap/mount";
import { ArtLoaded, HandActionHovered, HintAutoHidden, PriorityElapsed } from "../messages";
import { MountHintAutoHide, MountPriorityWatch } from "./audio-mount";
import { MountHandBarDrag } from "./hand-drag-mount";

/** Resolve stream mounts emitted by `boardOverlays` / `turnChromeView` in Foldkit scene tests. */
export function resolveBoardOverlayMounts() {
  return Scene.Mount.resolveAll(
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}

export function resolveLiveBoardMounts() {
  return Scene.Mount.resolveAll(
    [MountHintAutoHide(), HintAutoHidden()],
    [MountBitmapLayer(), ArtLoaded()],
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}
