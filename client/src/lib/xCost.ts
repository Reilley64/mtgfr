import type { WireCost } from "~/wire/types";

export function clampX(value: number, min: number, max: number): number {
  if (max < min) return min;
  const n = Math.floor(Number.isFinite(value) ? value : min);
  return Math.min(max, Math.max(min, n));
}

export function costWithChosenX(cost: WireCost, x: number): WireCost {
  const symbols = cost.x_symbols ?? (cost.has_x ? 1 : 0);
  return {
    generic: cost.generic + clampX(x, 0, Number.MAX_SAFE_INTEGER) * symbols,
    colored: [...cost.colored] as WireCost["colored"],
    has_x: false,
    x_symbols: 0,
  };
}
