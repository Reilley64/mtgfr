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
const STACK_ACTION_COLUMN_WIDTH = 220;
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
  const maxRotation = visibleCount <= 1 ? 0 : STACK_FAN_MAX_ROTATION;
  const rotatedHeight = rotatedCardHeight(metrics.cardW, metrics.cardH, maxRotation);
  const rotationOverflow = Math.ceil((rotatedHeight - metrics.cardH) / 2);
  const maxTop = viewport.height - metrics.barH - stackActionLane(viewport) - metrics.cardH - rotationOverflow;
  const normalMinTop = Math.round(16 * metrics.scale);
  const verticalClearanceImpossible = maxTop < normalMinTop;
  const top = verticalClearanceImpossible ? rotationOverflow : Math.max(normalMinTop, Math.min(naturalTop, maxTop));

  const normalLeft = viewport.width - STACK_OVERLAY_RIGHT - fanW;
  const rotatedWidth = rotatedCardWidth(metrics.cardW, metrics.cardH, maxRotation);
  const horizontalOverflow = Math.ceil((rotatedWidth - metrics.cardW) / 2);
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
  const rise = Math.round(8 * (layout.cardW / HAND_FACE_W) * (1 - centered * centered));
  return {
    x: layout.left + index * layout.stride,
    y: layout.top - rise,
    rotation: -STACK_FAN_MAX_ROTATION + progress * STACK_FAN_MAX_ROTATION * 2,
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

  const metrics = handMetrics(opts.viewport);
  const scale = metrics.scale;
  const minPeek = STACK_STRIP_MIN_PEEK * scale;
  const hPeek =
    opts.presentation === "full" ? minPeek : Math.max(minPeek, stackStripPeek(n, opts.viewport, metrics.cardW));
  const perRow = opts.presentation === "full" ? stackFullPerRow(opts.viewport, metrics.cardW) : n;
  const col = row % perRow;
  const rowY = Math.floor(row / perRow);
  const cols = Math.min(n, perRow);
  const rows = Math.ceil(n / perRow);
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  const stripW = Math.min(opts.viewport.width - margin, metrics.cardW + Math.max(0, cols - 1) * hPeek);
  const stripH = metrics.cardH + Math.max(0, rows - 1) * (metrics.cardH * 0.35);
  const headerH = 28 * scale;
  const gap = 8 * scale;
  const columnH = headerH + gap + stripH;
  const columnTop = opts.viewport.height / 2 - columnH / 2;
  const right = STACK_OVERLAY_RIGHT * scale;
  const stripLeft =
    opts.presentation === "full" ? opts.viewport.width / 2 - stripW / 2 : opts.viewport.width - right - stripW;
  const faceLeft = stripLeft + col * hPeek;
  const faceTop = columnTop + headerH + gap + rowY * metrics.cardH * 0.35;
  return { x: faceLeft + metrics.cardW / 2, y: faceTop + metrics.cardH / 2 };
}
