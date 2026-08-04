/**
 * Rules-text layout for the card-frame renderer. Pure: the caller supplies the measurement
 * function, so these run in a test with no canvas and in the renderer with `ctx.measureText`.
 *
 * A line is a list of pieces rather than a string because a printed rules line mixes three inks —
 * roman prose, italic reminder text, and mana pips — and each measures and draws differently.
 */

import { type OraclePart, splitOracleText } from "../oracleText";

/**
 * Line height as a multiple of font size. A printed card's rules lines step about 40px at a 35px
 * body — measured between line midpoints on a wrapped three-line card.
 */
export const LINE_HEIGHT = 1.16;

/** How far `fitOracleSize` will shrink before it lets the text overhang. */
const MIN_SCALE = 0.6;

/**
 * A mana pip's advance, as a multiple of the font size: the disk plus the hair of air around it. A
 * printed `{T}:` sets the colon all but against the disk, so this stays close to the disk itself.
 */
export const SYMBOL_EM = 0.92;

/** One drawable run: a word (carrying its own trailing space) or a mana symbol. */
export type Piece =
  | { kind: "text"; value: string; reminder: boolean }
  | { kind: "symbol"; code: string; ms: string; reminder: boolean };

export type Measure = (piece: Piece, fontPx: number) => number;

/**
 * One piece per word, each keeping its trailing space, so the pieces of a line rejoin into exactly
 * the printed sentence and a break can only fall between words.
 */
function pieces(parts: OraclePart[]): Piece[] {
  const out: Piece[] = [];
  for (const part of parts) {
    if (part.kind === "symbol") {
      out.push({ kind: "symbol", code: part.code, ms: part.ms, reminder: !!part.reminder });
      continue;
    }
    for (const word of part.text.split(/(?<=\s)/)) {
      if (word !== "") out.push({ kind: "text", value: word, reminder: !!part.reminder });
    }
  }
  return out;
}

/** Greedy word wrap that honours the card's own newlines (one printed ability per line). */
export function wrapOracle(text: string, maxWidth: number, fontPx: number, measure: Measure): Piece[][] {
  const lines: Piece[][] = [];
  for (const paragraph of text.split("\n")) {
    let line: Piece[] = [];
    let width = 0;
    for (const piece of pieces(splitOracleText(paragraph))) {
      const advance = measure(piece, fontPx);
      // A single word wider than the box still ships — better an overhang than a dropped word.
      if (line.length > 0 && width + advance > maxWidth) {
        lines.push(line);
        line = [];
        width = 0;
      }
      line.push(piece);
      width += advance;
    }
    lines.push(line);
  }
  return lines;
}

/**
 * The largest size whose wrapped text fits the box, never below 60% of `maxFontPx`. A card with
 * more text than its box holds overhangs at that floor rather than shrinking to illegibility —
 * which is what an over-full printed text box does too.
 */
export function fitOracleSize(
  text: string,
  box: { w: number; h: number },
  maxFontPx: number,
  measure: Measure,
): number {
  const floor = maxFontPx * MIN_SCALE;
  for (let size = maxFontPx; size > floor; size -= 0.5) {
    if (wrapOracle(text, box.w, size, measure).length * size * LINE_HEIGHT <= box.h) return size;
  }
  return floor;
}
