// Combat / target arrow strokes and draw-on animation state.

import type { Stroke, Vec } from "~/lib/boardPaintPrims";

const ARROW_DRAW_MS = 180;

export { ARROW_DRAW_MS };

/** Birth times for in-flight arrow draw-on animations (owned by the paint loop). */
export type ArrowAnimState = {
  born: Map<string, number>;
};

export function emptyArrowAnimState(): ArrowAnimState {
  return { born: new Map() };
}

/** Fraction of an arrow's draw-on (0 → 1) given birth time and now. */
export function arrowDrawProgress(bornAtMs: number, nowMs: number): number {
  return Math.min(1, Math.max(0, (nowMs - bornAtMs) / ARROW_DRAW_MS));
}

/**
 * Progress for one arrow key; may insert a birth timestamp into `born`.
 * Mutates `born` in place (same map reference the caller owns).
 */
export function arrowProgressFor(
  born: Map<string, number>,
  key: string,
  nowMs: number,
  reducedMotion: boolean,
): { progress: number; animating: boolean } {
  if (reducedMotion) return { progress: 1, animating: false };
  if (!born.has(key)) born.set(key, nowMs);
  const progress = arrowDrawProgress(born.get(key) ?? 0, nowMs);
  return { progress, animating: progress < 1 };
}

/** Drop birth entries for keys not seen this frame. */
export function pruneArrowBorn(born: Map<string, number>, seen: ReadonlySet<string>): void {
  for (const k of [...born.keys()]) {
    if (!seen.has(k)) born.delete(k);
  }
}

export function arrowBetweenWithProgress(
  ctx: CanvasRenderingContext2D,
  a: Vec,
  b: Vec,
  stroke: Stroke,
  t: number,
) {
  const mx = (a.x + b.x) / 2;
  const my = (a.y + b.y) / 2;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  const bulge = Math.min(48, len * 0.22);
  const cx = mx - (dy / len) * bulge;
  const cy = my + (dx / len) * bulge;

  ctx.save();
  ctx.strokeStyle = stroke.color;
  ctx.fillStyle = stroke.color;
  ctx.lineWidth = 3;
  ctx.setLineDash(stroke.dash);
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  const steps = Math.max(2, Math.ceil(24 * Math.max(t, 0.05)));
  let endX = a.x;
  let endY = a.y;
  for (let i = 1; i <= steps; i++) {
    const u = (i / steps) * t;
    endX = (1 - u) * (1 - u) * a.x + 2 * (1 - u) * u * cx + u * u * b.x;
    endY = (1 - u) * (1 - u) * a.y + 2 * (1 - u) * u * cy + u * u * b.y;
    ctx.lineTo(endX, endY);
  }
  const uTip = Math.max(0, t - 0.02);
  const tx0 = (1 - uTip) * (1 - uTip) * a.x + 2 * (1 - uTip) * uTip * cx + uTip * uTip * b.x;
  const ty0 = (1 - uTip) * (1 - uTip) * a.y + 2 * (1 - uTip) * uTip * cy + uTip * uTip * b.y;
  ctx.stroke();
  ctx.setLineDash([]);
  const ang = Math.atan2(endY - ty0, endX - tx0);
  ctx.beginPath();
  ctx.moveTo(endX, endY);
  ctx.lineTo(endX - 13 * Math.cos(ang - 0.4), endY - 13 * Math.sin(ang - 0.4));
  ctx.lineTo(endX - 13 * Math.cos(ang + 0.4), endY - 13 * Math.sin(ang + 0.4));
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}
