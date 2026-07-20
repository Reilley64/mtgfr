// Combat / target arrow strokes and draw-on animation state.

import type { Stroke, Vec } from "~/lib/boardPaintPrims";

const ARROW_DRAW_MS = 180;

export { ARROW_DRAW_MS };

const arrowBorn = new Map<string, number>();
let arrowsSeenThisFrame = new Set<string>();
let arrowsAnimating = false;

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Fraction of an arrow's draw-on (0 → 1) given birth time and now. */
export function arrowDrawProgress(bornAtMs: number, nowMs: number): number {
  return Math.min(1, Math.max(0, (nowMs - bornAtMs) / ARROW_DRAW_MS));
}

function arrowProgress(key: string): number {
  arrowsSeenThisFrame.add(key);
  if (prefersReducedMotion()) return 1;
  if (!arrowBorn.has(key)) arrowBorn.set(key, performance.now());
  const t = arrowDrawProgress(arrowBorn.get(key) ?? 0, performance.now());
  if (t < 1) arrowsAnimating = true;
  return t;
}

export function pruneArrows() {
  for (const k of [...arrowBorn.keys()]) {
    if (!arrowsSeenThisFrame.has(k)) arrowBorn.delete(k);
  }
  arrowsSeenThisFrame = new Set();
}

export function resetArrowAnimFlag() {
  arrowsAnimating = false;
}

export function markArrowsAnimating() {
  arrowsAnimating = true;
}

export function arrowsNeedFrame(): boolean {
  return arrowsAnimating;
}

export function arrowBetween(ctx: CanvasRenderingContext2D, a: Vec, b: Vec, stroke: Stroke, key: string) {
  const t = arrowProgress(key);
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
