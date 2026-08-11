import { HAND_BAR_PEEK } from "./handBarHit";

export const HAND_FACE_W = 208;
export const HAND_VISIBLE_H = 178;
export const HAND_DESIGN_VIEWPORT = { width: 1440, height: 900 } as const;
const HAND_PIP_ROW_H = 24;
const CARD_ASPECT = 0.716;

export type ViewportSize = { width: number; height: number };

export type HandMetrics = {
  scale: number;
  cardW: number;
  cardH: number;
  peek: number;
  overlap: number;
  visibleH: number;
  pipRowH: number;
  pipSize: number;
  barH: number;
  stickyBand: number;
  playSlack: number;
};

export function handUiScale(viewport: ViewportSize): number {
  const raw = Math.min(viewport.width / HAND_DESIGN_VIEWPORT.width, viewport.height / HAND_DESIGN_VIEWPORT.height);
  if (!(raw > 0)) return 1;
  return Math.max(0.75, Math.min(1.5, raw));
}

export function handMetrics(viewport: ViewportSize): HandMetrics {
  const scale = handUiScale(viewport);
  const cardW = Math.round(HAND_FACE_W * scale);
  const cardH = Math.round(cardW / CARD_ASPECT);
  const peek = Math.round(HAND_BAR_PEEK * scale);
  const visibleH = Math.round(HAND_VISIBLE_H * scale);
  const pipRowH = Math.round(HAND_PIP_ROW_H * scale);
  const barH = visibleH + pipRowH + Math.round(16 * scale);
  return {
    scale,
    cardW,
    cardH,
    peek,
    overlap: cardW - peek,
    visibleH,
    pipRowH,
    pipSize: Math.round(14 * scale),
    barH,
    stickyBand: barH - visibleH + cardH,
    playSlack: Math.round(96 * scale),
  };
}

export const HAND_BASE_METRICS = handMetrics(HAND_DESIGN_VIEWPORT);
export const HAND_BAR_H = HAND_BASE_METRICS.barH;
