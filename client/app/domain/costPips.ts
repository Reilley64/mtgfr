// Expand a WireCost into ordered mana-font pips for the hand-bar cost strip (Arena-style).

import type { WireCost } from "~/wire/types";
import { manaFontClass } from "./oracleText";

const COLOR_PIP = ["W", "U", "B", "R", "G"] as const;
/** The ten unordered colour pairs in the wire's `hybrid` order — `engine::COLOR_PAIRS`. */
export const HYBRID_PIP = ["W/U", "W/B", "W/R", "W/G", "U/B", "U/R", "U/G", "B/R", "B/G", "R/G"] as const;

/** Opaque disk fills — same hexes as mana-font `.ms-cost` (Arena-readable on dark felt). */
const PLATE_GENERIC = "#beb9b2";
const PLATE_BY_CODE: Record<string, string> = {
  W: "#f0f2c0",
  U: "#b5cde3",
  B: "#aca29a",
  R: "#db8664",
  G: "#93b483",
};

export type CostPip = { ms: string; code: string };

/**
 * Cast-cost pips in printed order: X, generic number, WUBRG, then the hybrid (CR 107.4e) and
 * Phyrexian (CR 107.4f) pips — one glyph per pip. Empty costs (typical lands) yield `[]` unless
 * `showZero` forces a `{0}` pip; a cost of nothing but hybrids is not empty.
 */
export function costPips(cost: WireCost, opts?: { showZero?: boolean }): CostPip[] {
  const out: CostPip[] = [];
  const xSymbols = cost.x_symbols ?? (cost.has_x ? 1 : 0);
  for (let i = 0; i < xSymbols; i++) push(out, "X");
  if (cost.generic > 0) push(out, String(cost.generic));
  repeat(out, cost.colored, (i) => COLOR_PIP[i]);
  repeat(out, cost.hybrid, (i) => HYBRID_PIP[i]);
  repeat(out, cost.phyrexian, (i) => `${COLOR_PIP[i]}/P`);
  if (out.length === 0 && opts?.showZero) push(out, "0");
  return out;
}

/** One pip per counted symbol: `counts[i]` copies of whatever `code` names slot `i`. */
function repeat(out: CostPip[], counts: Array<number> | undefined, code: (i: number) => string | undefined) {
  for (const [i, n] of (counts ?? []).entries()) {
    for (let k = 0; k < n; k++) push(out, code(i));
  }
}

/** Solid plate colour for a pip code (`2`, `W`, `X`, …). */
export function costPipPlate(code: string): string {
  return PLATE_BY_CODE[code.toUpperCase()] ?? PLATE_GENERIC;
}

function push(out: CostPip[], code: string | undefined) {
  if (code == null) return;
  const ms = manaFontClass(code);
  if (!ms) return;
  out.push({ ms, code });
}
