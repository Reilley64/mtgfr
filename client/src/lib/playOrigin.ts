/** Pending play-in origins keyed by the hand/command card object id being played. */

export type PlayOrigin = { x: number; y: number };

/** Record where a play-in leg should start for this card id. */
export function notePlayOrigin(map: Map<number, PlayOrigin>, cardId: number, origin: PlayOrigin): void {
  map.set(cardId, origin);
}

/** Consume and return the pending origin for `cardId`, or null. */
export function takePlayOrigin(map: Map<number, PlayOrigin>, cardId: number): PlayOrigin | null {
  const origin = map.get(cardId);
  if (!origin) return null;
  map.delete(cardId);
  return origin;
}

/**
 * Screen-space delta from an absolute from-point to an absolute to-point, for CSS
 * `translate(var(--stack-from-dx), var(--stack-from-dy))` keyframes that end at `transform: none`.
 */
export function stackInFromDelta(from: PlayOrigin, to: PlayOrigin): { dx: number; dy: number } {
  return { dx: from.x - to.x, dy: from.y - to.y };
}
