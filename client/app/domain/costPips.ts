// Expand a WireCost into ordered mana-font pips for the hand-bar cost strip (Arena-style).

import type { WireCost } from "~/wire/types";
import { manaFontClass } from "./oracleText";

const COLOR_PIP = ["W", "U", "B", "R", "G"] as const;

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
 * Cast-cost pips in printed order: X, generic number, then WUBRG (one glyph per pip).
 * Empty costs (typical lands) yield `[]` unless `showZero` forces a `{0}` pip.
 */
export function costPips(cost: WireCost, opts?: { showZero?: boolean }): CostPip[] {
  const out: CostPip[] = [];
  if (cost.has_x) push(out, "X");
  if (cost.generic > 0) push(out, String(cost.generic));
  for (let i = 0; i < 5; i++) {
    const n = cost.colored[i] ?? 0;
    for (let k = 0; k < n; k++) push(out, COLOR_PIP[i]);
  }
  if (out.length === 0 && opts?.showZero) push(out, "0");
  return out;
}

/** Solid plate colour for a pip code (`2`, `W`, `X`, …). */
export function costPipPlate(code: string): string {
  return PLATE_BY_CODE[code.toUpperCase()] ?? PLATE_GENERIC;
}

function push(out: CostPip[], code: string) {
  const ms = manaFontClass(code);
  if (!ms) return;
  out.push({ ms, code });
}
