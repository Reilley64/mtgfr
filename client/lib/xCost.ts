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

const COLOR_BRACE = ["W", "U", "B", "R", "G"] as const;

/** Arena-style brace string for a resolved cost — safe for generics outside mana-font. */
export function costText(cost: WireCost): string {
  const parts: string[] = [];
  if (cost.has_x) {
    const n = cost.x_symbols ?? 1;
    for (let i = 0; i < n; i++) parts.push("{X}");
  }
  if (cost.generic > 0) parts.push(`{${cost.generic}}`);
  for (let i = 0; i < 5; i++) {
    const n = cost.colored[i] ?? 0;
    for (let k = 0; k < n; k++) parts.push(`{${COLOR_BRACE[i]}}`);
  }
  if (parts.length === 0) return "{0}";
  return parts.join("");
}
