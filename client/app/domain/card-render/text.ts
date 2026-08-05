/**
 * Rules-text layout for the card-frame renderer. Pure: the caller supplies the measurement
 * function, so these run in a test with no canvas and in the renderer with `ctx.measureText`.
 *
 * A line is a list of pieces rather than a string because a printed rules line mixes three inks —
 * roman prose, italic reminder text, and mana pips — and each measures and draws differently.
 */

import { emphasisSpans, type OraclePart, splitOracleText } from "../oracleText";

/**
 * Line height as a multiple of font size, *within* one printed paragraph. A printed M15 card's
 * rules lines step about 37px at a 35px body — measured row by row off Scryfall's png for Llanowar
 * Elves (`fdn`), whose flavor block wraps to four lines at a 36.7px pitch.
 */
export const LINE_HEIGHT = 1.06;

/**
 * Extra air where a new ability opens, as a multiple of font size — print sets a multi-ability box
 * with visibly more room between abilities than between the wrapped lines of one. Swept against
 * printed cards with `just client-card-diff` (Karmic Guide `15090117`, Solemn Simulacrum
 * `b89aae48`, both two abilities and flavor): the text box scores best a little over half a line,
 * and the whole sweep from 0 up beats no gap at all.
 */
export const PARA_GAP = 0.6;

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

/** Greedy wrap of one paragraph's pieces: a break can only fall between words. */
function wrapPieces(paragraph: Piece[], maxWidth: number, fontPx: number, measure: Measure): Piece[][] {
  const lines: Piece[][] = [];
  let line: Piece[] = [];
  let width = 0;
  for (const piece of paragraph) {
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
  return lines;
}

/** Rules text: roman prose, italic reminder text in its parentheses, pips as pips. */
const rulesPieces = (paragraph: string): Piece[] => pieces(splitOracleText(paragraph));

/**
 * Flavor text: italic throughout — which is what a reminder piece already draws as — except where
 * Scryfall's `*…*` marks emphasis, which print leans back to roman.
 */
const flavorPieces = (paragraph: string): Piece[] =>
  emphasisSpans(paragraph).flatMap((span) =>
    pieces(splitOracleText(span.text)).map((piece) => ({ ...piece, reminder: !span.emphasis })),
  );

/** Greedy word wrap that honours the card's own newlines (one printed ability per line). */
export function wrapOracle(text: string, maxWidth: number, fontPx: number, measure: Measure): Piece[][] {
  return text.split("\n").flatMap((paragraph) => wrapPieces(rulesPieces(paragraph), maxWidth, fontPx, measure));
}

/**
 * Everything the text box sets: the rules lines, then the divider row, then the flavor lines. The
 * divider is the index of a blank line the caller rules across; `null` when the card prints only
 * one of the two blocks. `starts` holds the lines that open a new printed ability, which print sets
 * an extra [`PARA_GAP`] above.
 */
export type TextBlock = { lines: Piece[][]; divider: number | null; starts: ReadonlySet<number> };

/** One block's lines, plus the index each paragraph after the first opens at. */
function wrapBlock(
  text: string,
  toPieces: (paragraph: string) => Piece[],
  maxWidth: number,
  fontPx: number,
  measure: Measure,
): { lines: Piece[][]; starts: number[] } {
  const lines: Piece[][] = [];
  const starts: number[] = [];
  for (const paragraph of text.split("\n")) {
    if (lines.length > 0) starts.push(lines.length);
    lines.push(...wrapPieces(toPieces(paragraph), maxWidth, fontPx, measure));
  }
  return { lines, starts };
}

export function cardTextBlock(
  oracle: string,
  flavor: string,
  maxWidth: number,
  fontPx: number,
  measure: Measure,
): TextBlock {
  const rules = oracle === "" ? { lines: [], starts: [] } : wrapBlock(oracle, rulesPieces, maxWidth, fontPx, measure);
  if (flavor === "") return { lines: rules.lines, divider: null, starts: new Set(rules.starts) };

  // Flavor sets as one unbroken italic block: print runs an attribution straight on under its quote
  // at the plain pitch, and the divider row above already opens more air than a paragraph break.
  const flavorLines = wrapBlock(flavor, flavorPieces, maxWidth, fontPx, measure).lines;
  if (rules.lines.length === 0) return { lines: flavorLines, divider: null, starts: new Set() };

  return {
    lines: [...rules.lines, [], ...flavorLines],
    divider: rules.lines.length,
    starts: new Set(rules.starts),
  };
}

/** The advance from the previous line's centre to this one's, wider where a new ability opens. */
export const lineStep = (block: TextBlock, index: number, fontPx: number): number =>
  fontPx * (LINE_HEIGHT + (block.starts.has(index) ? PARA_GAP : 0));

/** How tall the block sets at this size — every line, plus the air between abilities. */
export const blockHeight = (block: TextBlock, fontPx: number): number =>
  fontPx * (block.lines.length * LINE_HEIGHT + block.starts.size * PARA_GAP);

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
    if (blockHeight(cardTextBlock(oracle, flavor, box.w, size, measure), size) <= box.h) return size;
  }
  return floor;
}
