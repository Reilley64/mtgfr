/**
 * Text layout for the card-frame renderer. Pure: the caller supplies the measurement function, so
 * these run in a test with no canvas and in the renderer with `ctx.measureText`.
 */

export type Measure = (text: string, fontPx: number) => number;

/** Line height as a multiple of font size — MPlantin sets tight on a real card. */
export const LINE_HEIGHT = 1.2;

/** How far `fitFontSize` will shrink before it lets the text overhang. */
const MIN_SCALE = 0.6;

/** Greedy word wrap that honours the card's own newlines (one printed ability per line). */
export function wrapLines(text: string, maxWidth: number, fontPx: number, measure: Measure): string[] {
  const out: string[] = [];
  for (const paragraph of text.split("\n")) {
    if (paragraph === "") {
      out.push("");
      continue;
    }
    let line = "";
    for (const word of paragraph.split(" ")) {
      const candidate = line === "" ? word : `${line} ${word}`;
      if (line !== "" && measure(candidate, fontPx) > maxWidth) {
        out.push(line);
        line = word;
        continue;
      }
      line = candidate;
    }
    // A single word wider than the box still ships — better an overhang than a dropped word.
    out.push(line);
  }
  return out;
}

/**
 * The largest size whose wrapped text fits the box, never below 60% of `maxFontPx`. A card with
 * more text than its box holds overhangs at that floor rather than shrinking to illegibility —
 * which is what an over-full printed text box does too.
 */
export function fitFontSize(text: string, box: { w: number; h: number }, maxFontPx: number, measure: Measure): number {
  const floor = maxFontPx * MIN_SCALE;
  for (let size = maxFontPx; size > floor; size -= 0.5) {
    const lines = wrapLines(text, box.w, size, measure);
    if (lines.length * size * LINE_HEIGHT <= box.h) return size;
  }
  return floor;
}

export type Token = { kind: "text"; value: string } | { kind: "symbol"; value: string };

/** `{T}`, `{G/W}`, `{U/P}` — a brace run with no brace or newline inside it. */
const SYMBOL = /\{([^{}\n]+)\}/g;

/** Splits `{T}: Add {G}.` into symbol and prose runs so each can be drawn with its own font. */
export function splitSymbols(text: string): Token[] {
  const out: Token[] = [];
  let last = 0;
  for (const match of text.matchAll(SYMBOL)) {
    const at = match.index ?? 0;
    if (at > last) out.push({ kind: "text", value: text.slice(last, at) });
    out.push({ kind: "symbol", value: match[1] });
    last = at + match[0].length;
  }
  if (last < text.length) out.push({ kind: "text", value: text.slice(last) });
  return out;
}
