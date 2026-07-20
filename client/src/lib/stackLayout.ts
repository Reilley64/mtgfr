// Shared stack geometry for the DOM overlay and canvas aim origins.

// Stack overlay geometry — one source for the DOM overlay and the canvas entrance seed.
export const STACK_CARD_W = 180;
export const STACK_OVERLAY_RIGHT = 16;
/** Vertical peek of each lower card in the physical pile. */
export const STACK_PEEK = 34;
/** Screen-x inset from the right edge to the stack card centerline (glide anchor). */
export const STACK_ANCHOR_FROM_RIGHT = STACK_OVERLAY_RIGHT + STACK_CARD_W / 2;
/** Object count at which the magnifier appears even at full peek. */
export const STACK_EXPAND_COUNT = 6;
/** Tightest horizontal peek before escalating to full stack view. */
export const STACK_STRIP_MIN_PEEK = 20;
/** Default vertical chrome reserved under/above the centered pile (captions / hold). */
export const STACK_VERTICAL_RESERVED = 120;
/** Horizontal inset when measuring strip/full width. */
export const STACK_HORIZONTAL_MARGIN = 48;

export function stackCardH(cardW = STACK_CARD_W): number {
  return cardW / 0.716;
}

/**
 * Vertical peek so `cardH + (n-1)*peek` fits the usable viewport band.
 * Never exceeds {@link STACK_PEEK}; may compress to 0.
 */
export function stackPeekFor(count: number, viewportH: number, reserved = STACK_VERTICAL_RESERVED): number {
  const n = Math.max(1, count);
  if (n <= 1) return STACK_PEEK;
  const cardH = stackCardH();
  const maxPileH = Math.max(cardH, viewportH - reserved);
  const peek = (maxPileH - cardH) / (n - 1);
  return Math.min(STACK_PEEK, Math.max(0, peek));
}

/** Magnifier / expand control: reading threshold or compression has started. */
export function stackExpandAvailable(count: number, peek: number): boolean {
  if (count >= STACK_EXPAND_COUNT) return true;
  return peek < STACK_PEEK - 0.01;
}

/**
 * Auto-collapse expand/full when thresholds clear or the stack empties.
 * Keeps expand open while a local staged target is live so aim does not re-arm mid-read.
 */
export function shouldAutoCollapseStackExpand(opts: {
  expanded: boolean;
  count: number;
  peek: number;
  staged: boolean;
}): boolean {
  if (!opts.expanded) return false;
  if (opts.count <= 0) return true;
  if (opts.staged) return false;
  return !stackExpandAvailable(opts.count, opts.peek);
}

/** Whether a horizontal strip of `count` cards fits at {@link STACK_STRIP_MIN_PEEK}. */
export function stackStripFits(
  count: number,
  viewportW: number,
  cardW = STACK_CARD_W,
  minPeek = STACK_STRIP_MIN_PEEK,
  margin = STACK_HORIZONTAL_MARGIN,
): boolean {
  if (count <= 1) return true;
  return cardW + (count - 1) * minPeek <= viewportW - margin;
}

/** Horizontal peek for the expanded strip (compresses to fit; caller escalates if below min). */
export function stackStripPeek(
  count: number,
  viewportW: number,
  cardW = STACK_CARD_W,
  margin = STACK_HORIZONTAL_MARGIN,
): number {
  if (count <= 1) return STACK_PEEK;
  const budget = viewportW - margin - cardW;
  return Math.min(STACK_PEEK, Math.max(0, budget / (count - 1)));
}

export type StackPresentation = "pile" | "expanded" | "full";

/** Active stack presentation given expand open-state and viewport. */
export function stackPresentation(opts: {
  count: number;
  expandedOpen: boolean;
  viewportW: number;
  viewportH: number;
}): StackPresentation {
  if (!opts.expandedOpen || opts.count <= 0) return "pile";
  if (!stackStripFits(opts.count, opts.viewportW)) return "full";
  return "expanded";
}

/**
 * Screen-space center of the top card in a right-edge pile of `count` cards.
 * Assumes the pile box alone is viewport-centered (`top-1/2 -translate-y-1/2`); captions and
 * buttons must sit outside that box so they don't shift the pile.
 */
export function stackAimOrigin(
  viewportW: number,
  viewportH: number,
  count: number,
  peek: number = STACK_PEEK,
): { x: number; y: number } {
  const n = Math.max(1, count);
  const cardH = stackCardH();
  const pileH = cardH + (n - 1) * peek;
  return {
    x: viewportW - STACK_ANCHOR_FROM_RIGHT,
    // Simplifies to viewportH/2 - (n-1)*peek/2.
    y: viewportH / 2 + pileH / 2 - (n - 1) * peek - cardH / 2,
  };
}

/** Aim origin while a hand card is staged in arrow-target mode; `null` for pick/none/expand. */
export function stagingAimFrom(
  viewportW: number,
  viewportH: number,
  stackLen: number,
  arrowStaging: boolean,
  peek: number = STACK_PEEK,
): { x: number; y: number } | null {
  if (!arrowStaging) return null;
  return stackAimOrigin(viewportW, viewportH, stackLen + 1, peek);
}

/** Max cards per row in full view at min horizontal peek (wrap capacity). */
export function stackFullPerRow(
  viewportW: number,
  cardW = STACK_CARD_W,
  minPeek = STACK_STRIP_MIN_PEEK,
  margin = STACK_HORIZONTAL_MARGIN,
): number {
  if (minPeek <= 0) return Math.max(1, Math.floor((viewportW - margin) / cardW));
  return Math.max(1, Math.floor((viewportW - margin - cardW) / minPeek) + 1);
}
