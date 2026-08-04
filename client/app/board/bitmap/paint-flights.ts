import { colors } from "~/design-tokens.generated";
import type { FaceData } from "../../domain/card-render/frame";
import { imageUrlByPrint } from "../../domain/deck-builder/scryfall";
import type { ImageCache } from "../../domain/image-cache";
import { FLIGHT_CARD_H, FLIGHT_CARD_W } from "../geometry/layout";
import { LIFT_SHADOW_BLUR, LIFT_SHADOW_COLOR, LIFT_SHADOW_OFFSET_Y } from "../lift-shadow";
import type { CardFlight } from "../motion/flights";
import { type BitmapImageCache, CARD_OUTLINE, type FaceSource, roundRect } from "./paint-cards";

export const FLIGHT_SHADOW_BLUR = LIFT_SHADOW_BLUR;
export const FLIGHT_SHADOW_OFFSET_Y = LIFT_SHADOW_OFFSET_Y;
export const FLIGHT_SHADOW_COLOR = LIFT_SHADOW_COLOR;

export function paintFlightCard(
  ctx: CanvasRenderingContext2D,
  flight: CardFlight,
  zoom: number,
  cache: BitmapImageCache | Pick<ImageCache, "get">,
  faces?: FaceSource,
): void {
  const w = FLIGHT_CARD_W * zoom * flight.scale;
  const h = FLIGHT_CARD_H * zoom * flight.scale;
  const x = flight.x - w / 2;
  const y = flight.y - h / 2;
  const r = 6 * zoom * Math.max(flight.scale, 0.5);

  ctx.save();
  ctx.shadowColor = FLIGHT_SHADOW_COLOR;
  ctx.shadowBlur = FLIGHT_SHADOW_BLUR;
  ctx.shadowOffsetY = FLIGHT_SHADOW_OFFSET_Y;
  roundRect(ctx, x, y, w, h, r);
  ctx.fillStyle = colors.oracleIvory;
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.shadowOffsetY = 0;
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, 2 * zoom);
  ctx.stroke();

  // The card the player picked up wears its rendered face in the hand bar, so the flight wears the
  // same one — the printed image is the fallback until the face is drawn.
  const rendered = flight.face != null ? renderedFace(flight.face, faces) : undefined;
  const img = rendered ?? (flight.print ? cache.get(imageUrlByPrint(flight.print)) : undefined);
  if (img) {
    ctx.save();
    roundRect(ctx, x, y, w, h, r);
    ctx.clip();
    ctx.drawImage(img, x, y, w, h);
    ctx.restore();
  } else {
    ctx.fillStyle = "#1a1a1a";
    ctx.font = `bold ${Math.max(10, 12 * zoom * flight.scale)}px system-ui,sans-serif`;
    ctx.textAlign = "center";
    ctx.fillText(flight.name, flight.x, flight.y, w - 8);
  }

  ctx.restore();
}

/** The card-shaped rendered face, asking the cache to draw it the first time it is wanted. */
function renderedFace(face: FaceData, faces: FaceSource | undefined): CanvasImageSource | undefined {
  if (faces == null) return undefined;
  const drawn = faces.get(face, "full");
  if (drawn == null) faces.request(face, "full");
  return drawn;
}
