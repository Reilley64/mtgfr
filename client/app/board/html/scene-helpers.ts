import { Scene } from "foldkit/test";
import { BindCardArt } from "~/ui/card-art";
import { MountBitmapLayer, MountFlightLayer } from "../bitmap/mount";
import {
  AltDown,
  ArtLoaded,
  HandActionHovered,
  HintAutoHidden,
  PriorityElapsed,
} from "../messages";
import { MountBoardAudio, MountHintAutoHide, MountPriorityWatch } from "./audio-mount";
import { MountHandBarDrag } from "./hand-drag-mount";
import { MountBoardKeyboard } from "./keyboard-mount";

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

export function resolveLiveBoardMounts(options: { hint?: boolean } = {}) {
  const withHint = options.hint !== false;
  return Scene.Mount.resolveAll(
    [MountBoardKeyboard(), AltDown()],
    [MountBoardAudio(), ArtLoaded()],
    ...(withHint ? ([[MountHintAutoHide(), HintAutoHidden()]] as const) : []),
    [MountBitmapLayer(), ArtLoaded()],
    [MountFlightLayer(), ArtLoaded()],
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}
