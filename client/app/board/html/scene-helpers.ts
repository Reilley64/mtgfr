import { Scene } from "foldkit/test";
import { MountBitmapLayer } from "../bitmap/mount";
import { BindCardArt } from "~/ui/card-art";
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

/** Resolve `cardArt` hosts when the rendered overlay includes card faces. */
export function resolveBoardCardArtMounts(count = 1) {
  const resolvers = Array.from({ length: count }, () => [BindCardArt, ArtLoaded()] as const);
  return Scene.Mount.resolveAll(...resolvers);
}

export function resolveLiveBoardMounts() {
  return Scene.Mount.resolveAll(
    [MountHintAutoHide(), HintAutoHidden()],
    [MountBitmapLayer(), ArtLoaded()],
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}
