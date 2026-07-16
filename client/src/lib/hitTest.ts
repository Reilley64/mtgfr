// Screen→world hit-testing: given a screen point and the world-space card rects,
// find the topmost card under the cursor. Pure and renderer-agnostic — it shares
// the camera transform with the canvas, so hits line up exactly with what is drawn.

import { AVATAR_R } from "~/layout";
import { type Camera, screenToWorld } from "~/lib/camera";

export interface CardRect {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
  tapped?: boolean;
  /** Extra tilt about card center (radians) — permanent-cluster fan. */
  fanAngle?: number;
}

// A tapped card is drawn rotated 90° about its center, so its clickable footprint is the upright
// rect turned on its side — same center, w and h swapped. (The opponent's 180° rotation maps the
// rect onto itself, so it needs no adjustment.) Mid-tap-animation frames are counted as upright:
// the rotation is a few hundred ms and the two rects overlap heavily throughout.
function footprint(c: CardRect): { x: number; y: number; w: number; h: number; angle: number } {
  const angle = c.fanAngle ?? 0;
  if (!c.tapped) return { x: c.x, y: c.y, w: c.w, h: c.h, angle };
  return {
    x: c.x + (c.w - c.h) / 2,
    y: c.y + (c.h - c.w) / 2,
    w: c.h,
    h: c.w,
    angle,
  };
}

function contains(fp: { x: number; y: number; w: number; h: number; angle: number }, px: number, py: number): boolean {
  if (fp.angle === 0) {
    return px >= fp.x && px <= fp.x + fp.w && py >= fp.y && py <= fp.y + fp.h;
  }
  const cx = fp.x + fp.w / 2;
  const cy = fp.y + fp.h / 2;
  const dx = px - cx;
  const dy = py - cy;
  const cos = Math.cos(-fp.angle);
  const sin = Math.sin(-fp.angle);
  const localX = dx * cos - dy * sin;
  const localY = dx * sin + dy * cos;
  return Math.abs(localX) <= fp.w / 2 && Math.abs(localY) <= fp.h / 2;
}

// Cards are drawn in array order, so later cards paint on top. Return the last
// (topmost) card whose world-space footprint contains the cursor, or null.
export function hitTest(cam: Camera, screenX: number, screenY: number, cards: readonly CardRect[]): number | null {
  const p = screenToWorld(cam, screenX, screenY);
  for (let i = cards.length - 1; i >= 0; i--) {
    if (contains(footprint(cards[i]), p.x, p.y)) return cards[i].id;
  }
  return null;
}

// Hit-test avatar circles (life orbs). Given avatar world-space centers, return the seat
// number of an avatar under the screen point, or null. Avatars are tested in iteration order.
export function hitAvatar(
  cam: Camera,
  screenX: number,
  screenY: number,
  avatars: Record<number, { x: number; y: number }>,
): number | null {
  const p = screenToWorld(cam, screenX, screenY);
  for (const [seat, worldPos] of Object.entries(avatars)) {
    const dist = Math.hypot(p.x - worldPos.x, p.y - worldPos.y);
    if (dist <= AVATAR_R) {
      return parseInt(seat, 10);
    }
  }
  return null;
}
