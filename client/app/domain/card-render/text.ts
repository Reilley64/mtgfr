/**
 * Rules-text layout for the card-frame renderer. Pure: the caller supplies the measurement
 * function, so these run in a test with no canvas and in the renderer with `ctx.measureText`.
 *
 * A line is a list of pieces rather than a string because a printed rules line mixes three inks —
 * roman prose, italic reminder text, and mana pips — and each measures and draws differently.
 */

import { type OraclePart, splitOracleText } from "../oracleText";

/**
 * Line height as a multiple of font size. A printed M15 card's rules lines step about 37px at a
 * 35px body — measured row by row off Scryfall's png for Llanowar Elves (`fdn`), whose flavor block
 * wraps to four lines at a 36.7px pitch.
 *
 * ponytail: one pitch for the whole box. Print adds about a third of a line of air *between*
 * abilities (43px against 32px within one) — modelling that needs paragraph breaks threaded through
 * `wrapOracle` → `cardTextBlock` → `fitCardText`, and it only shows on a multi-ability card.
 */
export const LINE_HEIGHT = 1.06;

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
 * Everything the text box sets: the rules lines, then the divider row, then the flavor lines. The
 * divider is the index of a blank line the caller rules across; `null` when the card prints only
 * one of the two blocks.
 */
export type TextBlock = { lines: Piece[][]; divider: number | null };

/** Flavor text is set in italics, which is what a reminder piece already draws as. */
const italic = (line: Piece[]): Piece[] => line.map((piece) => ({ ...piece, reminder: true }));

export function cardTextBlock(
  oracle: string,
  flavor: string,
  maxWidth: number,
  fontPx: number,
  measure: Measure,
): TextBlock {
  const rules = oracle === "" ? [] : wrapOracle(oracle, maxWidth, fontPx, measure);
  if (flavor === "") return { lines: rules, divider: null };
  const flavorLines = wrapOracle(flavor, maxWidth, fontPx, measure).map(italic);
  if (rules.length === 0) return { lines: flavorLines, divider: null };
  return { lines: [...rules, [], ...flavorLines], divider: rules.length };
}

/**
 * The largest size whose wrapped block fits the box, never below 60% of `maxFontPx`. A card with
 * more text than its box holds overhangs at that floor rather than shrinking to illegibility —
 * which is what an over-full printed text box does too.
 */
export function fitCardText(
  oracle: string,
  flavor: string,
  box: { w: number; h: number },
  maxFontPx: number,
  measure: Measure,
): number {
  const floor = maxFontPx * MIN_SCALE;
  for (let size = maxFontPx; size > floor; size -= 0.5) {
    const { lines } = cardTextBlock(oracle, flavor, box.w, size, measure);
    if (lines.length * size * LINE_HEIGHT <= box.h) return size;
  }
  return floor;
}
