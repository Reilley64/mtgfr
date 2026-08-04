import type { ImageCache } from "../../domain/image-cache";
import { COMMANDER_GOLD, EXILE_OUTLINE, GRAVEYARD_OUTLINE, PLAYABLE_BORDER } from "../chrome";
import { FLIGHT_CARD_H, FLIGHT_CARD_W } from "../geometry/layout";
import { type ExitFx, exitFxParticles, particleAllowancePerFx } from "../motion/exit-fx";
import type { CardFlight } from "../motion/flights";
import type { DragGhost, DragGhostZone } from "../motion/screen-motion";
import type { BitmapImageCache, FaceSource } from "./paint-cards";
import { roundRect } from "./paint-cards";
import { paintExitFx } from "./paint-exit-fx";
import { paintFlightCard } from "./paint-flights";

export type ScreenMotionPaintInput = {
  dragGhost: DragGhost | null | undefined;
  flights: readonly CardFlight[];
  exitFx: readonly ExitFx[];
  zoom: number;
  cache: BitmapImageCache | Pick<ImageCache, "get">;
  faces?: FaceSource;
};

function zoneOutline(zone: DragGhostZone): string | null {
  if (zone === "command") return COMMANDER_GOLD;
  if (zone === "graveyard") return GRAVEYARD_OUTLINE;
  if (zone === "exile") return EXILE_OUTLINE;
  return null;
}

/** Paint a hand-drag ghost as a flight-sized face plus zone playable strokes. */
export function paintDragGhost(
  ctx: CanvasRenderingContext2D,
  ghost: DragGhost,
  zoom: number,
  cache: BitmapImageCache | Pick<ImageCache, "get">,
  faces?: FaceSource,
): void {
  paintFlightCard(
    ctx,
    {
      id: -1,
      print: ghost.print,
      name: ghost.name,
      face: ghost.face,
      x: ghost.x,
      y: ghost.y,
      scale: ghost.scale,
      targetX: ghost.x,
      targetY: ghost.y,
      targetScale: ghost.scale,
      phase: "flying",
      kind: "battlefield",
    },
    zoom,
    cache,
    faces,
  );

  const w = FLIGHT_CARD_W * zoom * ghost.scale;
  const h = FLIGHT_CARD_H * zoom * ghost.scale;
  const x = ghost.x - w / 2;
  const y = ghost.y - h / 2;
  const r = 6 * zoom * Math.max(ghost.scale, 0.5);
  const ring = Math.max(2, 2 * zoom);

  ctx.save();
  roundRect(ctx, x, y, w, h, r);
  ctx.strokeStyle = PLAYABLE_BORDER;
  ctx.lineWidth = ring;
  ctx.stroke();

  const outline = zoneOutline(ghost.zone);
  if (outline != null) {
    const pad = ring + Math.max(2, 2 * zoom);
    roundRect(ctx, x - pad / 2, y - pad / 2, w + pad, h + pad, r + pad / 2);
    ctx.strokeStyle = outline;
    ctx.lineWidth = ring;
    ctx.stroke();
  }
  ctx.restore();
}

/** Single flight-layer paint pass: drag ghost, then flights, then ExitFx. */
export function paintScreenMotion(ctx: CanvasRenderingContext2D, input: ScreenMotionPaintInput): void {
  if (input.dragGhost != null) {
    paintDragGhost(ctx, input.dragGhost, input.zoom, input.cache, input.faces);
  }
  for (const flight of input.flights) {
    paintFlightCard(ctx, flight, input.zoom, input.cache, input.faces);
  }
  const exitFx = input.exitFx;
  const particleAllowance = particleAllowancePerFx(exitFx.length);
  for (const fx of exitFx) {
    paintExitFx(ctx, fx, input.zoom, input.cache, exitFxParticles(fx, particleAllowance));
  }
}
