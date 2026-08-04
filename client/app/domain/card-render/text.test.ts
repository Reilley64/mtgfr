import { describe, expect, it } from "vitest";
import { cardTextBlock, fitCardText, LINE_HEIGHT, type Measure, type Piece, wrapOracle } from "./text";

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
    expect(prose(line.filter((piece) => piece.reminder))).toBe("(It can't be blocked.)");
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

  it("rules no divider when the card prints only one of the two", () => {
    expect(cardTextBlock("Flying", "", 400, 20, measure).divider).toBeNull();
    expect(cardTextBlock("", "It watches.", 400, 20, measure).divider).toBeNull();
  });

  it("counts the divider row against the fit, so flavor cannot overhang the box", () => {
    // Three lines of text plus a divider row: the fit has to shrink further than the text alone.
    const box = { w: 200, h: 90 };
    expect(fitCardText("Whenever this creature attacks, ", "It watches. ", box, 20, measure)).toBeLessThan(
      fitCardText("Whenever this creature attacks, ", "", box, 20, measure),
    );
  });
});
