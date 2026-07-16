// Format a mana pool as chips for the per-seat mana tray (hide zeros).

import type { PlayerView } from "~/api/generated";
import { manaFontClass } from "~/lib/oracleText";

type ManaPool = PlayerView["mana_pool"];

const COLOR_PIP = ["W", "U", "B", "R", "G"] as const;

export function emptyManaPool(): ManaPool {
  return { colored: [0, 0, 0, 0, 0], colorless: 0, any: 0, either: [], of_colors: [] };
}

/** Non-zero pip chips as `{ symbol, amount }` (legacy / tests). */
export function manaPipChips(pool: ManaPool): { symbol: string; amount: number }[] {
  const out: { symbol: string; amount: number }[] = [];
  for (let i = 0; i < 5; i++) {
    const n = pool.colored[i] ?? 0;
    if (n > 0) out.push({ symbol: COLOR_PIP[i], amount: n });
  }
  if (pool.colorless > 0) out.push({ symbol: "C", amount: pool.colorless });
  if (pool.any > 0) out.push({ symbol: "★", amount: pool.any });
  for (const e of pool.either ?? []) {
    if (e.amount > 0) out.push({ symbol: `${COLOR_PIP[e.a]}/${COLOR_PIP[e.b]}`, amount: e.amount });
  }
  for (const o of pool.of_colors ?? []) {
    if (o.amount <= 0) continue;
    const parts: string[] = [];
    for (let i = 0; i < 5; i++) {
      if (o.mask & (1 << i)) parts.push(COLOR_PIP[i]);
    }
    out.push({ symbol: parts.join(""), amount: o.amount });
  }
  return out;
}

/**
 * Mana-font chip for the DOM mana tray:
 * - `glyph` — cost pip (WUBRG/C) or automatic split hybrid (`either`)
 * - `any` — duo multicolor symbol ("one mana of any color")
 * - `ci` — color indicator pie (`of_colors` mask)
 * - `text` — last-resort fallback
 */
export type ManaTrayChip =
  | { kind: "glyph"; ms: string; code: string; amount: number }
  | { kind: "any"; amount: number }
  | { kind: "ci"; n: number; suffix: string; code: string; amount: number }
  | { kind: "text"; text: string; amount: number };

/** WUBRG letters present in `mask`, in wire color order (matches mana-font `.ms-ci-*` suffixes). */
function ofColorsParts(mask: number): { letters: string; code: string; n: number } {
  const letters: string[] = [];
  const code: string[] = [];
  for (let i = 0; i < 5; i++) {
    if (!(mask & (1 << i))) continue;
    letters.push(COLOR_PIP[i].toLowerCase());
    code.push(COLOR_PIP[i]);
  }
  return { letters: letters.join(""), code: code.join(""), n: letters.length };
}

/** Non-empty pool credits as tray chips using mana-font cost / split / duo / CI attributes. */
export function manaTrayChips(pool: ManaPool): ManaTrayChip[] {
  const out: ManaTrayChip[] = [];
  for (let i = 0; i < 5; i++) {
    const n = pool.colored[i] ?? 0;
    if (n <= 0) continue;
    const code = COLOR_PIP[i];
    const ms = manaFontClass(code);
    if (ms) out.push({ kind: "glyph", ms, code, amount: n });
    else out.push({ kind: "text", text: code, amount: n });
  }
  if (pool.colorless > 0) {
    const n = pool.colorless;
    // Always `{C}` — never a generic number pip (`ms-2`); count sits inside the circle in the tray.
    const ms = manaFontClass("C");
    if (ms) out.push({ kind: "glyph", ms, code: "C", amount: n });
    else out.push({ kind: "text", text: "C", amount: n });
  }
  if (pool.any > 0) out.push({ kind: "any", amount: pool.any });
  for (const e of pool.either ?? []) {
    if (e.amount <= 0) continue;
    const code = `${COLOR_PIP[e.a]}/${COLOR_PIP[e.b]}`;
    const ms = manaFontClass(code);
    if (ms) out.push({ kind: "glyph", ms, code, amount: e.amount });
    else out.push({ kind: "text", text: code, amount: e.amount });
  }
  for (const o of pool.of_colors ?? []) {
    if (o.amount <= 0) continue;
    const { letters, code, n } = ofColorsParts(o.mask);
    if (n === 0) continue;
    // Color indicators, not hybrid cost pips — of_colors is a filter set, not {U/B}.
    out.push({ kind: "ci", n, suffix: letters, code, amount: o.amount });
  }
  return out;
}
