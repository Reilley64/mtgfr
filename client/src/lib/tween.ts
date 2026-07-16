// Position tweening for the board canvas: cards glide (world-space x/y) from their previous
// layout spot to the new one instead of snapping — motion conveys the zone change, nothing
// more (DESIGN.md: 150–250ms, ease-out, never celebration). Pure and clock-free: the caller
// passes dt, so every step is deterministic and unit-testable.
//
// The math is exponential smoothing toward the target — a natural ease-out that also handles
// retargeting mid-flight for free (the target array is passed fresh every step, so a card
// re-routed while moving just curves toward the new spot).

export interface Point {
  x: number;
  y: number;
}

/** Anything with an id and a world position — `RenderCard` qualifies. */
export interface TweenTarget {
  id: number;
  x: number;
  y: number;
}

/** Animated world position per card id. */
export type Positions = Map<number, Point>;

/** Exponential time constant: a typical zone move visibly settles in ~150–200ms. */
const TAU_MS = 75;
/** Snap-to-target distance in world units (~half a pixel at table zoom). */
const EPSILON = 0.5;
/** How far *below* its slot a newly-appearing card starts, so it rises into place (reads as
 * "played" — the hand is below the viewer's board). ~half a card height — noticeable at fit-zoom without being floaty. */
const ENTER_RISE = 70;

/** The instant path (reduced motion / first paint): every card exactly at its target. */
export function snapAll(targets: TweenTarget[]): Positions {
  return new Map(targets.map((t) => [t.id, { x: t.x, y: t.y }]));
}

/** Snap-to-target for scalar tweens (tap fraction 0↔1). */
const EPS_SCALAR = 0.02;

/**
 * Exponential 1D lerp of a per-id scalar toward its target — used for the tap fraction (0 = upright,
 * 1 = tapped) so a permanent rotates smoothly when it taps or untaps. A newly-seen id starts *at*
 * its target (a land that enters tapped shouldn't spin); ids absent from `targets` are dropped.
 */
export function stepScalar(
  prev: Map<number, number>,
  targets: Map<number, number>,
  dtMs: number,
): { values: Map<number, number>; settled: boolean } {
  const alpha = 1 - Math.exp(-dtMs / TAU_MS);
  const values = new Map<number, number>();
  let settled = true;
  for (const [id, target] of targets) {
    const cur = prev.get(id);
    if (cur === undefined || Math.abs(target - cur) <= EPS_SCALAR) {
      values.set(id, target);
      continue;
    }
    values.set(id, cur + (target - cur) * alpha);
    settled = false;
  }
  return { values, settled };
}

/**
 * Advance one frame: move each tracked position toward its target by `dtMs` of ease-out.
 * A card appearing on the canvas (id absent from `prev`) enters from `ENTER_RISE` below its slot
 * and glides up — so a played land/creature animates into place rather than snapping. Ids absent
 * from `targets` are dropped. `settled` is true once every card sits exactly on its target, so the
 * caller can stop its rAF loop. (The instant path for first paint / reduced motion is `snapAll`.)
 */
export function stepToward(
  prev: Positions,
  targets: TweenTarget[],
  dtMs: number,
): { positions: Positions; settled: boolean } {
  const alpha = 1 - Math.exp(-dtMs / TAU_MS);
  const positions: Positions = new Map();
  let settled = true;
  for (const t of targets) {
    const cur = prev.get(t.id);
    if (!cur) {
      // Newly on the canvas: start below the slot and let the next frames glide it up.
      positions.set(t.id, { x: t.x, y: t.y + ENTER_RISE });
      settled = false;
      continue;
    }
    const dx = t.x - cur.x;
    const dy = t.y - cur.y;
    if (Math.hypot(dx, dy) <= EPSILON) {
      positions.set(t.id, { x: t.x, y: t.y });
      continue;
    }
    positions.set(t.id, { x: cur.x + dx * alpha, y: cur.y + dy * alpha });
    settled = false;
  }
  return { positions, settled };
}
