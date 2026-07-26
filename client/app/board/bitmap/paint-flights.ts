import { colors } from "~/design-tokens.generated";
import type { ImageCache } from "../../domain/image-cache";
import { CARD_H, CARD_W } from "../geometry/layout";
import { LIFT_SHADOW_BLUR, LIFT_SHADOW_COLOR, LIFT_SHADOW_OFFSET_Y } from "../lift-shadow";
import type { CardFlight } from "../motion/flights";
import { type BitmapImageCache, CARD_OUTLINE, resolvedBitmapFaceImage, roundRect } from "./paint-cards";

export const FLIGHT_SHADOW_BLUR = LIFT_SHADOW_BLUR;
export const FLIGHT_SHADOW_OFFSET_Y = LIFT_SHADOW_OFFSET_Y;
export const FLIGHT_SHADOW_COLOR = LIFT_SHADOW_COLOR;

export function paintFlightCard(
  ctx: CanvasRenderingContext2D,
  flight: CardFlight,
  zoom: number,
  cache: BitmapImageCache | Pick<ImageCache, "get">,
): void {
  const w = CARD_W * zoom * flight.scale;
  const h = CARD_H * zoom * flight.scale;
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

  const img = flight.print ? resolvedBitmapFaceImage(cache, flight.print, flight.proxyArtUrl) : undefined;
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
