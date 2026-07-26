import { colors } from "~/design-tokens.generated";
import { imageUrlByPrint } from "../../../lib/deck-builder/scryfall";
import type { ImageCache } from "../../../lib/image-cache";
import { CARD_H, CARD_W } from "../geometry/layout";
import type { ExitFx, ExitParticle } from "../motion/exit-fx";
import { type BitmapImageCache, CARD_OUTLINE, roundRect } from "./paint-cards";
import { FLIGHT_SHADOW_BLUR, FLIGHT_SHADOW_COLOR, FLIGHT_SHADOW_OFFSET_Y } from "./paint-flights";

type ExitFxCardRect = {
  x: number;
  y: number;
  w: number;
  h: number;
  r: number;
};

export function paintExitFx(
  ctx: CanvasRenderingContext2D,
  fx: ExitFx,
  zoom: number,
  cache: BitmapImageCache | Pick<ImageCache, "get">,
  particles: readonly ExitParticle[],
): void {
  const rect = exitFxCardRect(fx, zoom);
  const baseAlpha = Math.max(0, 1 - fx.progress * 0.85);

  ctx.save();
  ctx.globalAlpha = baseAlpha;
  if (fx.kind === "exile") applyExileSquash(ctx, fx.progress, fx.x, fx.y);
  paintExitFxCard(ctx, fx, rect, zoom, cache);
  if (fx.kind === "destroy") {
    paintDestroyVeil(ctx, rect, fx.progress);
  } else {
    paintExileVoid(ctx, rect, fx);
  }
  ctx.restore();

  if (fx.kind === "destroy") {
    paintDestroyParticles(ctx, particles, baseAlpha);
    return;
  }
  paintExileParticles(ctx, particles, baseAlpha);
}

function exitFxCardRect(fx: ExitFx, zoom: number): ExitFxCardRect {
  const w = CARD_W * zoom * fx.scale;
  const h = CARD_H * zoom * fx.scale;
  return {
    x: fx.x - w / 2,
    y: fx.y - h / 2,
    w,
    h,
    r: 6 * zoom * Math.max(fx.scale, 0.5),
  };
}

function paintExitFxCard(
  ctx: CanvasRenderingContext2D,
  fx: ExitFx,
  rect: ExitFxCardRect,
  zoom: number,
  cache: BitmapImageCache | Pick<ImageCache, "get">,
): void {
  ctx.save();
  ctx.shadowColor = FLIGHT_SHADOW_COLOR;
  ctx.shadowBlur = FLIGHT_SHADOW_BLUR;
  ctx.shadowOffsetY = FLIGHT_SHADOW_OFFSET_Y;
  roundRect(ctx, rect.x, rect.y, rect.w, rect.h, rect.r);
  ctx.fillStyle = colors.oracleIvory;
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.shadowOffsetY = 0;
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, 2 * zoom);
  ctx.stroke();

  const img = fx.print ? cache.get(imageUrlByPrint(fx.print)) : undefined;
  if (img) {
    ctx.save();
    roundRect(ctx, rect.x, rect.y, rect.w, rect.h, rect.r);
    ctx.clip();
    ctx.drawImage(img, rect.x, rect.y, rect.w, rect.h);
    ctx.restore();
  } else {
    ctx.fillStyle = "#1a1a1a";
    ctx.font = `bold ${Math.max(10, 12 * zoom * fx.scale)}px system-ui,sans-serif`;
    ctx.textAlign = "center";
    ctx.fillText(fx.name, fx.x, fx.y, rect.w - 8);
  }

  ctx.restore();
}

function paintDestroyVeil(ctx: CanvasRenderingContext2D, rect: ExitFxCardRect, progress: number): void {
  const edge = Math.max(6, rect.w * (0.08 + progress * 0.16));
  const ash = Math.max(8, rect.h * (0.08 + progress * 0.18));

  ctx.save();
  roundRect(ctx, rect.x, rect.y, rect.w, rect.h, rect.r);
  ctx.clip();
  ctx.fillStyle = `rgba(24, 12, 8, ${0.14 + progress * 0.28})`;
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.fillStyle = `rgba(46, 28, 18, ${0.22 + progress * 0.26})`;
  ctx.fillRect(rect.x, rect.y, rect.w, ash);
  ctx.fillRect(rect.x, rect.y + rect.h - ash, rect.w, ash);
  ctx.fillRect(rect.x, rect.y, edge, rect.h);
  ctx.fillRect(rect.x + rect.w - edge, rect.y, edge, rect.h);
  ctx.fillStyle = `rgba(255, 96, 32, ${0.12 + progress * 0.18})`;
  ctx.fillRect(rect.x, rect.y + rect.h - ash * 0.8, rect.w, ash * 0.55);
  ctx.restore();
}

function applyExileSquash(ctx: CanvasRenderingContext2D, progress: number, centerX: number, centerY: number): void {
  const scaleX = Math.max(0.18, 1 - progress * 0.32);
  const scaleY = Math.max(0.08, 1 - progress * 0.62);
  ctx.translate(centerX, centerY);
  ctx.scale(scaleX, scaleY);
  ctx.translate(-centerX, -centerY);
}

function paintExileVoid(ctx: CanvasRenderingContext2D, rect: ExitFxCardRect, fx: ExitFx): void {
  ctx.save();
  roundRect(ctx, rect.x, rect.y, rect.w, rect.h, rect.r);
  ctx.clip();
  ctx.fillStyle = `rgba(7, 18, 26, ${0.1 + fx.progress * 0.22})`;
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
  ctx.fillStyle = `rgba(61, 220, 151, ${0.12 + fx.progress * 0.16})`;
  ctx.fillRect(rect.x, rect.y + rect.h * 0.1, rect.w, rect.h * 0.8);
  ctx.restore();

  ctx.save();
  ctx.translate(fx.x, fx.y);
  ctx.scale(1, Math.max(0.12, 0.7 - fx.progress * 0.3));
  ctx.beginPath();
  ctx.arc(0, 0, Math.max(rect.w, rect.h) * (0.08 + fx.progress * 0.1), 0, Math.PI * 2);
  ctx.fillStyle = `rgba(126, 232, 208, ${0.24 + fx.progress * 0.24})`;
  ctx.fill();
  ctx.restore();
}

function paintDestroyParticles(
  ctx: CanvasRenderingContext2D,
  particles: readonly ExitParticle[],
  baseAlpha: number,
): void {
  for (const particle of particles) {
    ctx.save();
    ctx.globalAlpha = particle.alpha * baseAlpha;
    ctx.beginPath();
    ctx.arc(particle.x, particle.y, particle.r, 0, Math.PI * 2);
    ctx.fillStyle = particle.color;
    ctx.fill();
    ctx.restore();
  }
}

function paintExileParticles(
  ctx: CanvasRenderingContext2D,
  particles: readonly ExitParticle[],
  baseAlpha: number,
): void {
  for (const particle of particles) {
    ctx.save();
    ctx.globalAlpha = particle.alpha * baseAlpha;
    ctx.translate(particle.x, particle.y);
    ctx.scale(0.35, 1.15);
    ctx.beginPath();
    ctx.arc(0, 0, particle.r, 0, Math.PI * 2);
    ctx.fillStyle = particle.color;
    ctx.fill();
    ctx.restore();
  }
}
