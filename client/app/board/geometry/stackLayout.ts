// Shared stack geometry for the DOM overlay, canvas arrows, and flight aim origins.

import { HAND_FACE_W, handMetrics, handUiScale, type ViewportSize } from "./handMetrics";

export { TARGET_COLOR } from "../action/targeting";

export const STACK_COMPACT_VISIBLE = 4;
/** Compatibility name for the first count that offers expansion. */
export const STACK_EXPAND_COUNT = STACK_COMPACT_VISIBLE + 1;
/** Base card width retained for the current DOM presenter until it adopts `stackFanLayout`. */
export const STACK_CARD_W = HAND_FACE_W;
export const STACK_OVERLAY_RIGHT = 16;
const STACK_FAN_STRIDE_RATIO = 0.28;
const STACK_FAN_MAX_ROTATION = 6;
const STACK_ACTION_LANE_BASE = 120;
const STACK_ACTION_GAP_BASE = 12;
/** Comfortable design-space horizontal peek for expanded stack cards. */
export const STACK_PEEK = 34;
/** Tightest design-space horizontal peek before escalating to full stack view. */
export const STACK_STRIP_MIN_PEEK = 20;
/** Design-space horizontal inset when measuring strip/full width. */
export const STACK_HORIZONTAL_MARGIN = 48;
/** Compatibility reserve for the current DOM presenter. */
export const STACK_VERTICAL_RESERVED = 120;
/** Server base hold + max dwell extension (ms) — bar denominator cap. */
export const STACK_HOLD_MAX_MS = 5000;

export type StackPresentation = "pile" | "expanded" | "full";

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

export function stackCardH(cardW = STACK_CARD_W): number {
  return cardW / 0.716;
}

/** Compatibility vertical pile peek for the current DOM presenter. */
export function stackPeekFor(count: number, viewportH: number, reserved = STACK_VERTICAL_RESERVED): number {
  const n = Math.max(1, count);
  if (n <= 1) return STACK_PEEK;
  const cardH = stackCardH();
  const maxPileH = Math.max(cardH, viewportH - reserved);
  return Math.min(STACK_PEEK, Math.max(0, (maxPileH - cardH) / (n - 1)));
}

export function stackActionLane(viewport: ViewportSize): number {
  return Math.round((STACK_ACTION_LANE_BASE + STACK_ACTION_GAP_BASE) * handUiScale(viewport));
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
  const rotatedHeight = rotatedCardHeight(metrics.cardW, metrics.cardH, STACK_FAN_MAX_ROTATION);
  const rotationOverflow = Math.ceil((rotatedHeight - metrics.cardH) / 2);
  const maxTop = viewport.height - metrics.barH - stackActionLane(viewport) - metrics.cardH - rotationOverflow;
  return {
    cardW: metrics.cardW,
    cardH: metrics.cardH,
    stride,
    visibleFrom,
    visibleCount,
    hiddenCount: visibleFrom,
    fanW,
    left: viewport.width - STACK_OVERLAY_RIGHT - fanW,
    top: Math.max(Math.round(16 * metrics.scale), Math.min(naturalTop, maxTop)),
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

/** Magnifier / expand control: compact mode already exposes the newest four objects. */
export function stackExpandAvailable(count: number, _peek?: number): boolean {
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

function stackViewport(viewport: ViewportSize | number): ViewportSize {
  return typeof viewport === "number" ? { width: viewport, height: 900 } : viewport;
}

/** Whether a horizontal strip of `count` cards fits at the responsive minimum peek. */
export function stackStripFits(count: number, viewportInput: ViewportSize | number, cardW?: number): boolean {
  const viewport = stackViewport(viewportInput);
  const faceW = cardW ?? (typeof viewportInput === "number" ? STACK_CARD_W : handMetrics(viewport).cardW);
  if (count <= 1) return true;
  const scale = typeof viewportInput === "number" ? 1 : handUiScale(viewport);
  const minPeek = STACK_STRIP_MIN_PEEK * scale;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  return faceW + (count - 1) * minPeek <= viewport.width - margin;
}

/** Horizontal peek for the expanded strip (compresses to fit; caller escalates if below min). */
export function stackStripPeek(count: number, viewportInput: ViewportSize | number, cardW?: number): number {
  const viewport = stackViewport(viewportInput);
  const faceW = cardW ?? (typeof viewportInput === "number" ? STACK_CARD_W : handMetrics(viewport).cardW);
  const scale = typeof viewportInput === "number" ? 1 : handUiScale(viewport);
  const comfortablePeek = STACK_PEEK * scale;
  if (count <= 1) return comfortablePeek;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  const budget = viewport.width - margin - faceW;
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
export function stackFullPerRow(viewportInput: ViewportSize | number, cardW?: number): number {
  const viewport = stackViewport(viewportInput);
  const faceW = cardW ?? (typeof viewportInput === "number" ? STACK_CARD_W : handMetrics(viewport).cardW);
  const scale = typeof viewportInput === "number" ? 1 : handUiScale(viewport);
  const minPeek = STACK_STRIP_MIN_PEEK * scale;
  const margin = STACK_HORIZONTAL_MARGIN * scale;
  return Math.max(1, Math.floor((viewport.width - margin - faceW) / minPeek) + 1);
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
      return { x: layout.left + layout.cardW / 2, y: layout.top + layout.cardH / 2 };
    }
    return { x: placement.x + layout.cardW / 2, y: placement.y + layout.cardH / 2 };
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
