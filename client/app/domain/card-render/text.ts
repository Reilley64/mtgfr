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
 * Extra air where a new ability opens, as a multiple of font size. Print opens a hair, not a blank
 * line: on Scryfall's png for Guard Gomazoa (`pca`) the row scan puts `Defender, flying` and the
 * ability under it 40px apart, against 37px between that ability's own two wrapped lines. The
 * region score cannot see this — paper texture swamps it, and a sweep from 0.3 to 0.8 moves the
 * text box under 1% either way — so it is set off the printed pitch.
 */
export const PARA_GAP = 0.1;

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

/** The prefix a printed mode hangs its wrapped lines under: the bullet and the space after it. */
const BULLET: Piece = { kind: "text", value: "• ", reminder: false };

/** How far in a mode's wrapped lines sit — print aligns them under the mode's own text, not the bullet. */
export const hangIndent = (fontPx: number, measure: Measure): number => measure(BULLET, fontPx);

/**
 * Greedy wrap of one paragraph's pieces: a break can only fall between words. `hang` narrows every
 * line after the first by that much, which is the room a mode's hanging indent takes.
 */
function wrapPieces(paragraph: Piece[], maxWidth: number, fontPx: number, measure: Measure, hang = 0): Piece[][] {
  const lines: Piece[][] = [];
  let line: Piece[] = [];
  let width = 0;
  for (const piece of paragraph) {
    const advance = measure(piece, fontPx);
    // A single word wider than the box still ships — better an overhang than a dropped word.
    if (line.length > 0 && width + advance > maxWidth - (lines.length > 0 ? hang : 0)) {
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

/**
 * Print sets typographic quotes where Scryfall's text is plain ASCII. A single mark is always an
 * apostrophe on a card — `don't`, `Gaea's` — and a double alternates open then close along the
 * line, which is how a quoted line of flavor reads.
 */
export function smartQuotes(text: string): string {
  let opening = true;
  return text.replace(/["']/g, (mark) => {
    if (mark === "'") return "’";
    opening = !opening;
    return opening ? "”" : "“";
  });
}

/** Rules text: roman prose, italic reminder text in its parentheses, pips as pips. */
const rulesPieces = (paragraph: string): Piece[] => pieces(splitOracleText(smartQuotes(paragraph)));

/**
 * Flavor text: italic throughout — which is what a reminder piece already draws as — except where
 * Scryfall's `*…*` marks emphasis, which print leans back to roman.
 */
const flavorPieces = (paragraph: string): Piece[] =>
  emphasisSpans(smartQuotes(paragraph)).flatMap((span) =>
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
 * an extra [`PARA_GAP`] above; `hangs` the lines a mode wraps onto, which print sets in by
 * [`hangIndent`].
 */
export type TextBlock = {
  lines: Piece[][];
  divider: number | null;
  starts: ReadonlySet<number>;
  hangs: ReadonlySet<number>;
};

/** One block's lines, plus the index each paragraph after the first opens at. */
function wrapBlock(
  text: string,
  toPieces: (paragraph: string) => Piece[],
  maxWidth: number,
  fontPx: number,
  measure: Measure,
): { lines: Piece[][]; starts: number[]; hangs: number[] } {
  const lines: Piece[][] = [];
  const starts: number[] = [];
  const hangs: number[] = [];
  for (const paragraph of text.split("\n")) {
    // The modes of a modal spell are one ability: print runs `Choose one —` and its bullets at the
    // plain pitch, and hangs a mode's wrapped lines under its own first word.
    const mode = paragraph.startsWith("•");
    if (lines.length > 0 && !mode) starts.push(lines.length);
    const hang = mode ? hangIndent(fontPx, measure) : 0;
    const wrapped = wrapPieces(toPieces(paragraph), maxWidth, fontPx, measure, hang);
    for (let i = 1; i < wrapped.length && mode; i++) hangs.push(lines.length + i);
    lines.push(...wrapped);
  }
  return { lines, starts, hangs };
}

export function cardTextBlock(
  oracle: string,
  flavor: string,
  maxWidth: number,
  fontPx: number,
  measure: Measure,
): TextBlock {
  const rules =
    oracle === "" ? { lines: [], starts: [], hangs: [] } : wrapBlock(oracle, rulesPieces, maxWidth, fontPx, measure);
  const hangs = new Set(rules.hangs);
  if (flavor === "") return { lines: rules.lines, divider: null, starts: new Set(rules.starts), hangs };

  // Flavor sets as one unbroken italic block: print runs an attribution straight on under its quote
  // at the plain pitch, and the divider row above already opens more air than a paragraph break.
  const flavorLines = wrapBlock(flavor, flavorPieces, maxWidth, fontPx, measure).lines;
  if (rules.lines.length === 0) return { lines: flavorLines, divider: null, starts: new Set(), hangs: new Set() };

  return {
    lines: [...rules.lines, [], ...flavorLines],
    divider: rules.lines.length,
    starts: new Set(rules.starts),
    hangs,
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
