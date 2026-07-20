import { describe, expect, it } from "vitest";
import { manaFontClass, splitOracleText } from "~/lib/oracleText";

describe("manaFontClass", () => {
  it("maps pip letters and numbers to lowercase ms codes", () => {
    expect(manaFontClass("G")).toBe("g");
    expect(manaFontClass("C")).toBe("c");
    expect(manaFontClass("X")).toBe("x");
    expect(manaFontClass("2")).toBe("2");
  });

  it("maps tap/untap and strips hybrid slashes", () => {
    expect(manaFontClass("T")).toBe("tap");
    expect(manaFontClass("Q")).toBe("untap");
    expect(manaFontClass("U/R")).toBe("ur");
    expect(manaFontClass("2/W")).toBe("2w");
    expect(manaFontClass("W/P")).toBe("wp");
  });

  it("accepts either hybrid letter order (wire WUBRG vs mana-font canonical)", () => {
    // Engine COLOR_PAIRS emit White/Red as W/R; mana-font's class is .ms-rw.
    expect(manaFontClass("W/R")).toBe("rw");
    expect(manaFontClass("W/G")).toBe("gw");
    expect(manaFontClass("U/G")).toBe("gu");
    expect(manaFontClass("R/W")).toBe("rw");
  });

  it("returns null for unknown brace contents", () => {
    expect(manaFontClass("FOO")).toBeNull();
    expect(manaFontClass("")).toBeNull();
  });
});

describe("splitOracleText", () => {
  it("returns plain text unchanged when there are no symbols", () => {
    expect(splitOracleText("Flying")).toEqual([{ kind: "text", text: "Flying" }]);
  });

  it("splits brace symbols into mana-font parts", () => {
    expect(splitOracleText("{1}, {T}: Draw a card.")).toEqual([
      { kind: "symbol", code: "1", ms: "1" },
      { kind: "text", text: ", " },
      { kind: "symbol", code: "T", ms: "tap" },
      { kind: "text", text: ": Draw a card." },
    ]);
  });

  it("keeps unknown braces as literal text", () => {
    expect(splitOracleText("Pay {FOO}.")).toEqual([{ kind: "text", text: "Pay {FOO}." }]);
  });

  it("preserves newlines between symbols", () => {
    expect(splitOracleText("{T}: Add {C}.\n{T}: Add {G}.")).toEqual([
      { kind: "symbol", code: "T", ms: "tap" },
      { kind: "text", text: ": Add " },
      { kind: "symbol", code: "C", ms: "c" },
      { kind: "text", text: ".\n" },
      { kind: "symbol", code: "T", ms: "tap" },
      { kind: "text", text: ": Add " },
      { kind: "symbol", code: "G", ms: "g" },
      { kind: "text", text: "." },
    ]);
  });

  it("marks parenthetical reminder text (with the parens) for italic rendering", () => {
    expect(
      splitOracleText("This creature becomes prepared. (While it's prepared, you may cast a copy of its spell.)"),
    ).toEqual([
      { kind: "text", text: "This creature becomes prepared. " },
      {
        kind: "text",
        text: "(While it's prepared, you may cast a copy of its spell.)",
        reminder: true,
      },
    ]);
  });

  it("still splits mana symbols inside reminder text", () => {
    expect(splitOracleText("Equip {1} ({T}: Attach to target creature you control.)")).toEqual([
      { kind: "text", text: "Equip " },
      { kind: "symbol", code: "1", ms: "1" },
      { kind: "text", text: " " },
      { kind: "text", text: "(", reminder: true },
      { kind: "symbol", code: "T", ms: "tap", reminder: true },
      { kind: "text", text: ": Attach to target creature you control.)", reminder: true },
    ]);
  });
});
