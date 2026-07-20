// Shared canvas paint primitives for the board.

export type Vec = { x: number; y: number };

export interface Stroke {
  color: string;
  dash: number[];
}

/** Shared target-arrow / staged-preview accent (canvas stroke + DOM ring). */
export const TARGET_COLOR = "#77CCFF";
/** Legend sample for a permanent that stays bright during stack response. */
export const RESPONSE_COLOR = "#EAFFF0";

export const ATTACK_STROKE: Stroke = { color: "#FF5555", dash: [] };
export const BLOCK_STROKE: Stroke = { color: "#66FF99", dash: [10, 6] };
export const TARGET_STROKE: Stroke = { color: TARGET_COLOR, dash: [2, 6] };
export const SELECT_STROKE: Stroke = { color: "#FFD76A", dash: [] };
export const CARD_OUTLINE = "#1a1a1a";
/** Black veil over non-activatable permanents — ~same weight as hand `brightness-[0.55]`. */
export const DIM_CARD_VEIL = 0.45;

export function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
}
