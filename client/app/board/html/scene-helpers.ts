import * as Combobox from "@foldkit/ui/combobox";
import { Scene } from "foldkit/test";
import { BindCardArt, CardArtTick } from "~/ui/card-art";
import { MountBitmapLayer, MountFlightLayer } from "../bitmap/mount";
import { AltDown, ArtLoaded, BoardCameraZoomed, HandActionHovered, HintAutoHidden, PriorityElapsed } from "../messages";
import { MountBoardAudio, MountHintAutoHide, MountPriorityWatch } from "./audio-mount";
import { MountBoardCameraGesture } from "./camera-gesture-mount";
import { MountHandBarDrag } from "./hand-drag-mount";
import { MountBoardKeyboard } from "./keyboard-mount";

/** Resolve stream mounts emitted by `boardOverlays` / `turnChromeView` in Foldkit scene tests. */
export function resolveBoardOverlayMounts() {
  return Scene.Mount.resolveAll(
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}

/** Resolve the anchoring and backdrop-portal hosts Combobox renders on its open suggestion
 * panel. */
export function resolveCardNameComboboxMounts() {
  return Scene.Mount.resolveAll(
    [Combobox.AnchorCombobox, Combobox.CompletedAnchorCombobox()],
    [Combobox.PortalComboboxBackdrop, Combobox.CompletedPortalComboboxBackdrop()],
  );
}

/** Resolve `cardArt` hosts when the rendered overlay includes card faces. */
export function resolveBoardCardArtMounts(count = 1) {
  const resolvers = Array.from({ length: count }, () => [BindCardArt, CardArtTick()] as const);
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
    [MountBoardCameraGesture(), BoardCameraZoomed({ x: 0, y: 0, factor: 1 })],
    [MountPriorityWatch(), PriorityElapsed({ seconds: 0 })],
    [MountHandBarDrag(), HandActionHovered({ actionId: null })],
  );
}
