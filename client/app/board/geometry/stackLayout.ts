// Shared stack geometry for the DOM overlay, canvas arrows, and flight aim origins.

import { HAND_FACE_W, handMetrics, handUiScale, type ViewportSize } from "./handMetrics";

export { TARGET_COLOR } from "../action/targeting";

export const STACK_COMPACT_VISIBLE = 4;
export const STACK_OVERLAY_RIGHT = 16;
const STACK_FAN_STRIDE_RATIO = 0.28;
const STACK_FAN_MAX_ROTATION = 6;
const STACK_ACTION_LANE_BASE = 120;
const STACK_ACTION_GAP_BASE = 12;
/** Expanded rocker label + track + padding in unscaled CSS pixels. */
export const STACK_ACTION_COLUMN_WIDTH = 220;
/** `right-md` from the action bar's layout token. */
const STACK_ACTION_RIGHT = 10;
/** Comfortable design-space horizontal peek for expanded stack cards. */
export const STACK_PEEK = 34;
/** Tightest design-space horizontal peek before escalating to full stack view. */
export const STACK_STRIP_MIN_PEEK = 20;
/** Design-space horizontal inset when measuring strip/full width. */
export const STACK_HORIZONTAL_MARGIN = 48;
/** Server base hold + max dwell extension (ms) — bar denominator cap. */
export const STACK_HOLD_MAX_MS = 5000;

export type StackPresentation = "pile" | "expanded" | "full";

export type ScreenRect = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

export type StackOverflowBadgeLayout = {
  left: number;
  top: number;
  width: number;
  height: number;
  placement: "above" | "inside";
};

export type StackFanLayout = {
  cardW: number;
  cardH: number;
  stride: number;
  visibleFrom: number;
  visibleCount: number;
  hiddenCount: number;
  fanW: number;
  left: number;
  top: number;
  maxRotation: number;
  maxRise: number;
};

export function stackActionLane(viewport: ViewportSize): number {
  return Math.round((STACK_ACTION_LANE_BASE + STACK_ACTION_GAP_BASE) * handUiScale(viewport));
}

/** Conservative right-side rectangle reserved for primary actions and the fully opened rocker. */
export function stackReservedActionRect(viewport: ViewportSize): ScreenRect {
  const metrics = handMetrics(viewport);
  const right = viewport.width - STACK_ACTION_RIGHT;
  const bottom = viewport.height - metrics.barH;
  return {
    left: right - STACK_ACTION_COLUMN_WIDTH,
    top: bottom - stackActionLane(viewport),
    right,
    bottom,
  };
}

function rotatedCardWidth(cardW: number, cardH: number, degrees: number): number {
  const radians = (Math.abs(degrees) * Math.PI) / 180;
  return cardW * Math.cos(radians) + cardH * Math.sin(radians);
}

function rotatedCardHeight(cardW: number, cardH: number, degrees: number): number {
  const radians = (Math.abs(degrees) * Math.PI) / 180;
  return cardW * Math.sin(radians) + cardH * Math.cos(radians);
}

export function stackFanLayout(viewport: ViewportSize, count: number): StackFanLayout {
  const metrics = handMetrics(viewport);
  const visibleCount = Math.min(Math.max(0, count), STACK_COMPACT_VISIBLE);
  const visibleFrom = Math.max(0, count - visibleCount);
  const stride = Math.round(metrics.cardW * STACK_FAN_STRIDE_RATIO);
  const fanW = metrics.cardW + Math.max(0, visibleCount - 1) * stride;
  const naturalTop = (viewport.height - metrics.cardH) / 2;
  const normalMaxRotation = visibleCount <= 1 ? 0 : STACK_FAN_MAX_ROTATION;
  const normalRotatedHeight = rotatedCardHeight(metrics.cardW, metrics.cardH, normalMaxRotation);
  const normalRotationOverflow = Math.ceil((normalRotatedHeight - metrics.cardH) / 2);
  const maxTop = viewport.height - metrics.barH - stackActionLane(viewport) - metrics.cardH - normalRotationOverflow;
  const normalMinTop = Math.round(16 * metrics.scale);
  const verticalClearanceImpossible = maxTop < normalMinTop;
  // A rotated 156×218 phone-landscape face is ~233.6px tall, but only 226px exists above
  // the hand at 844×390. Preserve the exact hand-card size and flatten the compact transform
  // instead of trying to place an AABB that mathematically cannot fit in that band.
  const maxRotation = verticalClearanceImpossible ? 0 : normalMaxRotation;
  const maxRise = verticalClearanceImpossible ? 0 : 8 * (metrics.cardW / HAND_FACE_W);
  const top = verticalClearanceImpossible ? 0 : Math.max(normalMinTop, Math.min(naturalTop, maxTop));

  const rotatedWidth = rotatedCardWidth(metrics.cardW, metrics.cardH, maxRotation);
  const horizontalOverflow = Math.ceil((rotatedWidth - metrics.cardW) / 2);
  const normalLeft = viewport.width - STACK_OVERLAY_RIGHT - horizontalOverflow - fanW;
  const actionLeft = stackReservedActionRect(viewport).left;
  const fallbackLeft = actionLeft - STACK_ACTION_GAP_BASE * metrics.scale - fanW - horizontalOverflow;
  const left = verticalClearanceImpossible
    ? Math.max(horizontalOverflow, Math.min(normalLeft, fallbackLeft))
    : normalLeft;

  return {
    cardW: metrics.cardW,
    cardH: metrics.cardH,
    stride,
    visibleFrom,
    visibleCount,
    hiddenCount: visibleFrom,
    fanW,
    left,
    top,
    maxRotation,
    maxRise,
  };
}

const STACK_OVERFLOW_BADGE_WIDTH = 48;
const STACK_OVERFLOW_BADGE_HEIGHT = 28;
const STACK_OVERFLOW_BADGE_INSET = 8;

/** Viewport-safe overflow affordance placement shared by compact geometry and the DOM view. */
export function stackOverflowBadgeLayout(layout: StackFanLayout): StackOverflowBadgeLayout {
  if (layout.top >= STACK_OVERFLOW_BADGE_HEIGHT) {
    return {
      left: layout.left,
      top: layout.top - STACK_OVERFLOW_BADGE_HEIGHT,
      width: STACK_OVERFLOW_BADGE_WIDTH,
      height: STACK_OVERFLOW_BADGE_HEIGHT,
      placement: "above",
    };
  }

  return {
    left: layout.left + STACK_OVERFLOW_BADGE_INSET,
    top: layout.top + STACK_OVERFLOW_BADGE_INSET,
    width: STACK_OVERFLOW_BADGE_WIDTH,
    height: STACK_OVERFLOW_BADGE_HEIGHT,
    placement: "inside",
  };
}

export function stackFanPlacement(
  layout: StackFanLayout,
  row: number,
): { x: number; y: number; rotation: number } | null {
  const index = row - layout.visibleFrom;
  if (index < 0 || index >= layout.visibleCount) return null;
  if (layout.visibleCount <= 1) return { x: layout.left, y: layout.top, rotation: 0 };

  const progress = index / (layout.visibleCount - 1);
  const centered = progress * 2 - 1;
  const rise = Math.round(layout.maxRise * (1 - centered * centered));
  return {
    x: layout.left + index * layout.stride,
    y: layout.top - rise,
    rotation: -layout.maxRotation + progress * layout.maxRotation * 2,
  };
}

/** Axis-aligned screen bounds after applying the compact face's center-origin rotation. */
export function stackFanVisualBounds(layout: StackFanLayout, row: number): ScreenRect | null {
  const placement = stackFanPlacement(layout, row);
  if (placement == null) return null;
  const width = rotatedCardWidth(layout.cardW, layout.cardH, placement.rotation);
  const height = rotatedCardHeight(layout.cardW, layout.cardH, placement.rotation);
  const centerX = placement.x + layout.cardW / 2;
  const centerY = placement.y + layout.cardH / 2;
  return {
    left: centerX - width / 2,
    top: centerY - height / 2,
    right: centerX + width / 2,
    bottom: centerY + height / 2,
  };
}

/** Overflow expansion is available once compact mode hides an older object. */
export function stackExpandAvailable(count: number): boolean {
  return count > STACK_COMPACT_VISIBLE;
}

/** Auto-collapse expand/full when only compact rows remain, unless a staged target is live. */
export function shouldAutoCollapseStackExpand(opts: {
  expanded: boolean;
  count: number;
  peek?: number;
  staged: boolean;
}): boolean {
  if (!opts.expanded) return false;
  if (opts.count <= 0) return true;
  if (opts.staged) return false;
  return !stackExpandAvailable(opts.count);
}

/** Whether a horizontal strip of `count` cards fits at the responsive minimum peek. */
export function stackStripFits(count: number, viewport: ViewportSize, cardW = handMetrics(viewport).cardW): boolean {
  if (count <= 1) return true;
  const scale = handUiScale(viewport);
  const minPeek = STACK_STRIP_MIN_PEEK * scale;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  return cardW + (count - 1) * minPeek <= viewport.width - margin;
}

/** Horizontal peek for the expanded strip (compresses to fit; caller escalates if below min). */
export function stackStripPeek(count: number, viewport: ViewportSize, cardW = handMetrics(viewport).cardW): number {
  const scale = handUiScale(viewport);
  const comfortablePeek = STACK_PEEK * scale;
  if (count <= 1) return comfortablePeek;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  const budget = viewport.width - margin - cardW;
  return Math.min(comfortablePeek, Math.max(0, budget / (count - 1)));
}

/** Active stack presentation given expand open-state and viewport. */
export function stackPresentation(opts: {
  count: number;
  expandedOpen: boolean;
  viewportW: number;
  viewportH: number;
}): StackPresentation {
  if (!opts.expandedOpen || opts.count <= 0) return "pile";
  const viewport = { width: opts.viewportW, height: opts.viewportH };
  if (!stackStripFits(opts.count, viewport)) return "full";
  return "expanded";
}

/** Max cards per row in full view at responsive minimum horizontal peek (wrap capacity). */
export function stackFullPerRow(viewport: ViewportSize, cardW = handMetrics(viewport).cardW): number {
  const scale = handUiScale(viewport);
  const minPeek = STACK_STRIP_MIN_PEEK * scale;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  return Math.max(1, Math.floor((viewport.width - margin - cardW) / minPeek) + 1);
}

export type StackExpandedLayout = {
  presentation: "expanded" | "full";
  cardW: number;
  cardH: number;
  stripW: number;
  stripH: number;
  left: number;
  right: number;
  top: number;
  headerH: number;
  gap: number;
  peek: number;
  rowStride: number;
  perRow: number;
  rows: number;
};

const STACK_EXPANDED_HEADER_H = 28;
const STACK_EXPANDED_GAP = 8;

/** Exact expanded/full presentation geometry consumed by both DOM placement and screen origins. */
export function stackExpandedLayout(opts: {
  presentation: "expanded" | "full";
  viewport: ViewportSize;
  count: number;
}): StackExpandedLayout {
  const metrics = handMetrics(opts.viewport);
  const count = Math.max(1, opts.count);
  const minPeek = STACK_STRIP_MIN_PEEK * metrics.scale;
  const peek =
    opts.presentation === "full" ? minPeek : Math.max(minPeek, stackStripPeek(count, opts.viewport, metrics.cardW));
  const perRow = opts.presentation === "full" ? stackFullPerRow(opts.viewport, metrics.cardW) : count;
  const cols = Math.min(count, perRow);
  const rows = Math.ceil(count / perRow);
  const maxW = opts.viewport.width - STACK_HORIZONTAL_MARGIN * metrics.scale;
  const stripW = Math.min(maxW, metrics.cardW + Math.max(0, cols - 1) * peek);
  const rowStride = metrics.cardH * 0.35;
  const stripH = metrics.cardH + Math.max(0, rows - 1) * rowStride;
  const headerH = STACK_EXPANDED_HEADER_H;
  const gap = STACK_EXPANDED_GAP;
  const top = opts.viewport.height / 2 - (headerH + gap + stripH) / 2;
  const left =
    opts.presentation === "full"
      ? opts.viewport.width / 2 - stripW / 2
      : opts.viewport.width - STACK_OVERLAY_RIGHT - stripW;
  return {
    presentation: opts.presentation,
    cardW: metrics.cardW,
    cardH: metrics.cardH,
    stripW,
    stripH,
    left,
    right: opts.viewport.width - left - stripW,
    top,
    headerH,
    gap,
    peek,
    rowStride,
    perRow,
    rows,
  };
}

/** Screen-space center of stack face at `row` (0 = bottom) for the active presentation. */
export function stackFaceScreenOrigin(opts: {
  presentation: StackPresentation;
  viewport: ViewportSize;
  count: number;
  row: number;
}): { x: number; y: number } {
  const n = Math.max(1, opts.count);
  const row = Math.max(0, Math.min(n - 1, opts.row));
  if (opts.presentation === "pile") {
    const layout = stackFanLayout(opts.viewport, n);
    const placement = stackFanPlacement(layout, row) ?? stackFanPlacement(layout, layout.visibleFrom);
    if (placement == null) {
      return {
        x: layout.left + layout.cardW / 2,
        y: layout.top + layout.cardH / 2,
      };
    }
    return {
      x: placement.x + layout.cardW / 2,
      y: placement.y + layout.cardH / 2,
    };
  }

  const layout = stackExpandedLayout({
    presentation: opts.presentation,
    viewport: opts.viewport,
    count: n,
  });
  const col = row % layout.perRow;
  const rowY = Math.floor(row / layout.perRow);
  return {
    x: layout.left + col * layout.peek + layout.cardW / 2,
    y: layout.top + layout.headerH + layout.gap + rowY * layout.rowStride + layout.cardH / 2,
  };
}
