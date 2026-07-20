// CardArt always mounts an <img>, including when the art URL is empty (`src=""`).
// A <div> placeholder dropped `{...rest}` — hand/drag put pointer handlers and `style` on CardArt.

import type { ImageFace } from "~/lib/scryfall";

/** DOM tag for CardArt's face. Locked to `img` for empty and non-empty URLs alike. */
export function cardArtFaceTag(_artUrl: string): "img" {
  return "img";
}

/**
 * When a face image 404s, which face to try next.
 * Prepare/flip DFCs have no Scryfall `/back/` art — fall back to front. Front failures stay front
 * (no retry loop). Transform backs that exist load on the first try and never hit this.
 */
export function imageFaceAfterLoadError(requested: ImageFace): ImageFace {
  if (requested === "back") return "front";
  return "front";
}
