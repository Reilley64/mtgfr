import { imageUrlByPrint } from "../../../lib/deck-builder/scryfall";
import type { ImageCache } from "../../../lib/image-cache";
import { CARD_H, CARD_W } from "../geometry/layout";
import type { CardFlight } from "../motion/flights";
import { type BitmapImageCache, CARD_OUTLINE, roundRect } from "./paint-cards";

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
  ctx.shadowColor = "rgba(0,0,0,0.45)";
  ctx.shadowBlur = 16;
  roundRect(ctx, x, y, w, h, r);
  ctx.fillStyle = "#e8e4d8";
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, 2 * zoom);
  ctx.stroke();

  const img = flight.print ? cache.get(imageUrlByPrint(flight.print)) : undefined;
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
