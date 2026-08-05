import { describe, expect, it } from "vitest";
import {
  blockHeight,
  cardTextBlock,
  fitCardText,
  hangIndent,
  LINE_HEIGHT,
  lineStep,
  type Measure,
  PARA_GAP,
  type Piece,
  wrapOracle,
} from "./text";

/** A fake metric: every glyph is half the font size wide, every pip one em. No canvas. */
const measure: Measure = (piece, fontPx) => (piece.kind === "symbol" ? fontPx : piece.value.length * fontPx * 0.5);

const width = (line: Piece[], fontPx: number) => line.reduce((sum, piece) => sum + measure(piece, fontPx), 0);
const prose = (line: Piece[]) =>
  line.map((piece) => (piece.kind === "symbol" ? `{${piece.code}}` : piece.value)).join("");

describe("wrapOracle", () => {
  it("breaks on spaces so no line exceeds the box", () => {
    const lines = wrapOracle("flying vigilance trample haste", 100, 10, measure);
    for (const line of lines) expect(width(line, 10)).toBeLessThanOrEqual(100);
    expect(lines.map(prose).join("")).toBe("flying vigilance trample haste");
  });

  it("keeps the card's own line breaks — one ability per line", () => {
    const lines = wrapOracle("Flying\n{T}: Add {G}.", 400, 10, measure);
    expect(lines.map(prose)).toEqual(["Flying", "{T}: Add {G}."]);
  });

  it("does not drop a word longer than the box", () => {
    const lines = wrapOracle("antidisestablishmentarianism", 20, 10, measure);
    expect(lines.map(prose).join("")).toContain("antidisestablishmentarianism");
  });

  it("keeps a card's blank line between abilities", () => {
    expect(wrapOracle("Flying\n\nTrample", 400, 10, measure).map(prose)).toEqual(["Flying", "", "Trample"]);
  });

  it("hands a pip out as its own piece so the renderer can draw a disk", () => {
    const [line] = wrapOracle("Add {G}.", 400, 10, measure);
    expect(line).toContainEqual({ kind: "symbol", code: "G", ms: "g", reminder: false });
  });

  it("marks reminder text so it can print italic", () => {
    const [line] = wrapOracle("Flying (It can't be blocked.)", 400, 10, measure);
    expect(prose(line.filter((piece) => piece.reminder))).toBe("(It can’t be blocked.)");
  });
});

describe("fitCardText", () => {
  it("keeps the maximum size when the text already fits", () => {
    expect(fitCardText("Flying", "", { w: 400, h: 200 }, 20, measure)).toBe(20);
  });

  it("shrinks until the wrapped text fits the box height", () => {
    const long = "Whenever this creature attacks, ".repeat(3);
    const fitted = fitCardText(long, "", { w: 200, h: 60 }, 20, measure);
    expect(fitted).toBeLessThan(20);
    expect(wrapOracle(long, 200, fitted, measure).length * fitted * LINE_HEIGHT).toBeLessThanOrEqual(60);
  });

  it("stops shrinking at 60% of the maximum rather than vanishing", () => {
    // No size in range fits this much text in this box; the floor holds and the text overhangs,
    // which is what a real card does when its text box is over-full.
    const wall = "Whenever this creature attacks, ".repeat(12);
    expect(fitCardText(wall, "", { w: 200, h: 60 }, 20, measure)).toBe(12);
  });
});

describe("cardTextBlock", () => {
  it("rules the divider between the rules text and the flavor", () => {
    const block = cardTextBlock("Flying", "It watches.", 400, 20, measure);

    expect(block.divider).toBe(1);
    expect(block.lines[1]).toEqual([]); // the row the divider is drawn across
    expect(prose(block.lines[2] ?? [])).toBe("It watches.");
  });

  it("sets the flavor in italics, the way a printed card does", () => {
    const block = cardTextBlock("Flying", "It watches.", 400, 20, measure);

    expect(block.lines[0]?.every((piece) => piece.reminder)).toBe(false);
    expect(block.lines[2]?.every((piece) => piece.reminder)).toBe(true);
  });

  it("leans emphasis back to roman and never inks the asterisks", () => {
    // Scryfall marks emphasis in flavor with `*…*`; print sets those words upright against italics.
    const block = cardTextBlock("", "—*Phyrexian Scriptures*, vol. 2", 400, 20, measure);
    const [line = []] = block.lines;

    expect(prose(line)).toBe("—Phyrexian Scriptures, vol. 2");
    expect(prose(line.filter((piece) => !piece.reminder))).toBe("Phyrexian Scriptures");
    expect(prose(line.filter((piece) => piece.reminder))).toBe("—, vol. 2");
  });

  it("rules no divider when the card prints only one of the two", () => {
    expect(cardTextBlock("Flying", "", 400, 20, measure).divider).toBeNull();
    expect(cardTextBlock("", "It watches.", 400, 20, measure).divider).toBeNull();
  });

  it("sets air between abilities, and none inside one", () => {
    const two = cardTextBlock("Flying\nTrample", "", 400, 20, measure);
    expect([...two.starts]).toEqual([1]);
    expect(blockHeight(two, 20)).toBeCloseTo(20 * (2 * LINE_HEIGHT + PARA_GAP));
    expect(lineStep(two, 1, 20)).toBeCloseTo(20 * (LINE_HEIGHT + PARA_GAP));

    // The same words wrapping inside one ability step at the plain pitch.
    const wrapped = cardTextBlock("Flying Trample", "", 100, 20, measure);
    expect(wrapped.lines.length).toBe(2);
    expect(blockHeight(wrapped, 20)).toBeCloseTo(20 * 2 * LINE_HEIGHT);
  });

  it("sets quotes the way print does, not as typewriter ticks", () => {
    const block = cardTextBlock("Gaea's Cradle can't be blocked.", '"Watch," she said. "Then go."', 900, 20, measure);
    expect(block.lines.map(prose).join(" ")).toBe("Gaea’s Cradle can’t be blocked.  “Watch,” she said. “Then go.”");
  });

  it("sets a modal spell's modes tight — they are one ability, not several", () => {
    // Print runs `Choose one —` straight into its bullets at the plain pitch (Abrade, `hou`).
    const block = cardTextBlock("Choose one —\n• Deal 3 damage.\n• Destroy target artifact.", "", 400, 20, measure);
    expect([...block.starts]).toEqual([]);
    expect(blockHeight(block, 20)).toBeCloseTo(20 * 3 * LINE_HEIGHT);
  });

  it("hangs a mode's wrapped lines under its own text, clear of the bullet", () => {
    // Abrade's first mode wraps, and print sets `creature.` in line with `Abrade`, not with the dot.
    const block = cardTextBlock("• Abrade deals 3 damage to target creature.", "", 200, 20, measure);
    expect(block.lines.length).toBeGreaterThan(1);
    expect([...block.hangs]).toEqual([...block.lines.keys()].slice(1));
    expect(hangIndent(20, measure)).toBeCloseTo(measure({ kind: "text", value: "• ", reminder: false }, 20));
  });

  it("wraps a hung line short, so the indent cannot push a mode past the box", () => {
    const wide = cardTextBlock("Deal 3 damage to any target now", "", 200, 20, measure);
    const mode = cardTextBlock("• Deal 3 damage to any target now", "", 200, 20, measure);
    // The bullet paragraph has the same words plus a bullet and a narrower second line, so it can
    // only set in as many lines as the plain one, never fewer.
    expect(mode.lines.length).toBeGreaterThanOrEqual(wide.lines.length);
  });

  it("opens no gap at the divider — the blank row is already wider than one", () => {
    const block = cardTextBlock("Flying", "It watches.", 400, 20, measure);
    expect([...block.starts]).toEqual([]);
  });

  it("runs an attribution straight on under its quote", () => {
    // Print sets flavor as one unbroken block: `—Darius, to Kassandra` follows the quote at the
    // plain pitch, not with the air that opens a new ability.
    const block = cardTextBlock("Flying", '"Watch."\n—Darius', 400, 20, measure);
    expect(block.lines.map(prose)).toEqual(["Flying", "", "“Watch.”", "—Darius"]);
    expect([...block.starts]).toEqual([]);
  });

  it("shrinks a multi-ability card sooner than the same words as one ability", () => {
    const box = { w: 400, h: 20 * LINE_HEIGHT * 2 };
    expect(fitCardText("Flying\nTrample", "", box, 20, measure)).toBeLessThan(
      fitCardText("Flying Trample", "", box, 20, measure),
    );
  });

  it("counts the divider row against the fit, so flavor cannot overhang the box", () => {
    // The rules alone wrap to two lines; the divider row and the flavor line take it past a box
    // that only holds three, so the fit has to shrink further than the text alone.
    const box = { w: 200, h: 20 * LINE_HEIGHT * 3.5 };
    expect(fitCardText("Whenever this creature attacks, ", "It watches. ", box, 20, measure)).toBeLessThan(
      fitCardText("Whenever this creature attacks, ", "", box, 20, measure),
    );
  });
});
