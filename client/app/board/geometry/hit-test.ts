// Screen→world hit-testing: given a screen point and the world-space card rects,
// find the topmost card under the cursor. Pure and renderer-agnostic — it shares
// the camera transform with the canvas, so hits line up exactly with what is drawn.

import { type Camera, screenToWorld } from "./camera";
import { AVATAR_R } from "./layout";

export interface CardRect {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

// Every rotation a card is drawn with — the opponent's 180°, the tapped tile's slight tilt — leaves
// it centred on its upright rect, so the upright rect is the footprint. ponytail: the tilted
// corners poke a few px outside it; nobody aims at a corner.
function contains(c: CardRect, px: number, py: number): boolean {
  return px >= c.x && px <= c.x + c.w && py >= c.y && py <= c.y + c.h;
}

// Cards are drawn in array order, so later cards paint on top. Return the last
// (topmost) card whose world-space footprint contains the cursor, or null.
export function hitTest(cam: Camera, screenX: number, screenY: number, cards: readonly CardRect[]): number | null {
  const p = screenToWorld(cam, screenX, screenY);
  for (let i = cards.length - 1; i >= 0; i--) {
    if (contains(cards[i], p.x, p.y)) return cards[i].id;
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
