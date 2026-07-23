// Shared stack geometry for the DOM overlay and canvas aim origins.

export { TARGET_COLOR } from "../action/targeting";

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
/** Server base hold + max dwell extension (ms) — bar denominator cap. */
export const STACK_HOLD_MAX_MS = 5000;

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

/** Screen-space center of the top card in a right-edge pile of `count` cards. */
export function stackPileAimOrigin(
  viewportW: number,
  viewportH: number,
  count: number,
  peek = STACK_PEEK,
): { x: number; y: number } {
  const n = Math.max(1, count);
  const cardH = stackCardH();
  const pileH = cardH + (n - 1) * peek;
  return {
    x: viewportW - STACK_ANCHOR_FROM_RIGHT,
    y: viewportH / 2 + pileH / 2 - (n - 1) * peek - cardH / 2,
  };
}

/** Screen-space center of stack face at `row` (0 = bottom) for the active presentation. */
export function stackFaceScreenOrigin(opts: {
  presentation: StackPresentation;
  viewportW: number;
  viewportH: number;
  count: number;
  row: number;
  peek?: number;
}): { x: number; y: number } {
  const n = Math.max(1, opts.count);
  const row = Math.max(0, Math.min(n - 1, opts.row));
  if (opts.presentation === "pile") {
    const peek = opts.peek ?? STACK_PEEK;
    const top = stackPileAimOrigin(opts.viewportW, opts.viewportH, n, peek);
    const fromTop = n - 1 - row;
    return { x: top.x, y: top.y + fromTop * peek };
  }

  const hPeek =
    opts.presentation === "full"
      ? STACK_STRIP_MIN_PEEK
      : Math.max(STACK_STRIP_MIN_PEEK, stackStripPeek(n, opts.viewportW));
  const perRow = opts.presentation === "full" ? stackFullPerRow(opts.viewportW) : n;
  const cardH = stackCardH();
  const col = row % perRow;
  const rowY = Math.floor(row / perRow);
  const cols = Math.min(n, perRow);
  const rows = Math.ceil(n / perRow);
  const stripW = Math.min(opts.viewportW - STACK_HORIZONTAL_MARGIN, STACK_CARD_W + Math.max(0, cols - 1) * hPeek);
  const stripH = cardH + Math.max(0, rows - 1) * (cardH * 0.35);
  // Approximate header row above the face strip (`Stack · N` + collapse).
  const headerH = 28;
  const gap = 8;
  const columnH = headerH + gap + stripH;
  const columnTop = opts.viewportH / 2 - columnH / 2;
  const stripLeft =
    opts.presentation === "full" ? opts.viewportW / 2 - stripW / 2 : opts.viewportW - STACK_OVERLAY_RIGHT - stripW;
  const faceLeft = stripLeft + col * hPeek;
  const faceTop = columnTop + headerH + gap + rowY * cardH * 0.35;
  return { x: faceLeft + STACK_CARD_W / 2, y: faceTop + cardH / 2 };
}
