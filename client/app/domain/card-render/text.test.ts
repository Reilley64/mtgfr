import { describe, expect, it } from "vitest";
import { fitFontSize, LINE_HEIGHT, splitSymbols, wrapLines } from "./text";

/** A fake metric: every glyph is half the font size wide. Deterministic, no canvas. */
const measure = (text: string, fontPx: number) => text.length * fontPx * 0.5;

describe("wrapLines", () => {
  it("breaks on spaces so no line exceeds the box", () => {
    const lines = wrapLines("flying vigilance trample haste", 100, 10, measure);
    for (const line of lines) expect(measure(line, 10)).toBeLessThanOrEqual(100);
    expect(lines.join(" ")).toBe("flying vigilance trample haste");
  });

  it("keeps the card's own line breaks — one ability per line", () => {
    const lines = wrapLines("Flying\n{T}: Add {G}.", 400, 10, measure);
    expect(lines).toEqual(["Flying", "{T}: Add {G}."]);
  });

  it("does not drop a word longer than the box", () => {
    const lines = wrapLines("antidisestablishmentarianism", 20, 10, measure);
    expect(lines.join("")).toContain("antidisestablishmentarianism");
  });

  it("keeps a card's blank line between abilities", () => {
    expect(wrapLines("Flying\n\nTrample", 400, 10, measure)).toEqual(["Flying", "", "Trample"]);
  });
});

describe("fitFontSize", () => {
  it("keeps the maximum size when the text already fits", () => {
    expect(fitFontSize("Flying", { w: 400, h: 200 }, 20, measure)).toBe(20);
  });

  it("shrinks until the wrapped text fits the box height", () => {
    const long = "Whenever this creature attacks, ".repeat(3);
    const fitted = fitFontSize(long, { w: 200, h: 60 }, 20, measure);
    expect(fitted).toBeLessThan(20);
    expect(wrapLines(long, 200, fitted, measure).length * fitted * LINE_HEIGHT).toBeLessThanOrEqual(60);
  });

  it("stops shrinking at 60% of the maximum rather than vanishing", () => {
    // No size in range fits this much text in this box; the floor holds and the text overhangs,
    // which is what a real card does when its text box is over-full.
    const wall = "Whenever this creature attacks, ".repeat(12);
    expect(fitFontSize(wall, { w: 200, h: 60 }, 20, measure)).toBe(12);
  });
});

describe("splitSymbols", () => {
  it("separates mana symbols from prose", () => {
    expect(splitSymbols("{T}: Add {G}.")).toEqual([
      { kind: "symbol", value: "T" },
      { kind: "text", value: ": Add " },
      { kind: "symbol", value: "G" },
      { kind: "text", value: "." },
    ]);
  });

  it("keeps hybrid and Phyrexian pips whole", () => {
    expect(splitSymbols("{G/W}{U/P}")).toEqual([
      { kind: "symbol", value: "G/W" },
      { kind: "symbol", value: "U/P" },
    ]);
  });

  it("passes prose with no symbols through untouched", () => {
    expect(splitSymbols("Flying")).toEqual([{ kind: "text", value: "Flying" }]);
  });

  it("leaves an unclosed brace as prose rather than swallowing the rest of the line", () => {
    expect(splitSymbols("Sacrifice {a creature")).toEqual([{ kind: "text", value: "Sacrifice {a creature" }]);
  });
});
