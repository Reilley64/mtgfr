// CardArt always mounts an <img>, including when the art URL is empty (`src=""`).
// A <div> placeholder dropped `{...rest}` — hand/drag put pointer handlers and `style` on CardArt.

/** DOM tag for CardArt's face. Locked to `img` for empty and non-empty URLs alike. */
export function cardArtFaceTag(_artUrl: string): "img" {
  return "img";
}
