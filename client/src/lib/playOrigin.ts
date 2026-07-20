/** Screen-space play-in point for CSS stack-in keyframes (DOM path; PlayMotion owns canvas flights). */

export type PlayOrigin = { x: number; y: number };

/**
 * Screen-space delta from an absolute from-point to an absolute to-point, for CSS
 * `translate(var(--stack-from-dx), var(--stack-from-dy))` keyframes that end at `transform: none`.
 */
export function stackInFromDelta(from: PlayOrigin, to: PlayOrigin): { dx: number; dy: number } {
  return { dx: from.x - to.x, dy: from.y - to.y };
}
