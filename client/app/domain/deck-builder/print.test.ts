import { describe, expect, it } from "vitest";
import { commanderPrintForRow, deckArtUrls, formatReleasedAt, reconcileEntries } from "./print";
import { imageUrlByPrint } from "./scryfall";

describe("formatReleasedAt", () => {
  it("returns the release year from a Scryfall date", () => {
    expect(formatReleasedAt("2024-03-15")).toBe("2024");
  });

  it("returns an em dash when the date is missing or malformed", () => {
    expect(formatReleasedAt(undefined)).toBe("—");
    expect(formatReleasedAt("")).toBe("—");
    expect(formatReleasedAt("bad")).toBe("—");
  });
});

describe("reconcileEntries", () => {
  it("maps deck lines by Card id", () => {
    expect(
      reconcileEntries([
        { id: "oracle-a", count: 1, print: "print-a" },
        { id: "oracle-b", count: 3, print: "print-b" },
      ]),
    ).toEqual({
      "oracle-a": { count: 1, print: "print-a" },
      "oracle-b": { count: 3, print: "print-b" },
    });
  });
});

describe("deckArtUrls", () => {
  it("lists the commander print and every card print at board art size", () => {
    expect(
      deckArtUrls({
        id: 4,
        name: "Atraxa",
        commander: "oracle-cmd",
        commander_print: "print-cmd",
        cards: [
          { id: "oracle-a", count: 1, print: "print-a" },
          { id: "oracle-b", count: 9, print: "print-b" },
        ],
      }),
    ).toEqual([
      imageUrlByPrint("print-cmd", "art"),
      imageUrlByPrint("print-a", "art"),
      imageUrlByPrint("print-b", "art"),
    ]);
  });

  it("warms the art crop, not the printed image the default size would fetch", () => {
    // The lobby warm used to fetch `display` while every rendered face asks for `art`, so the board
    // still went to the network for its first paint.
    const [url] = deckArtUrls({
      id: 4,
      name: "Atraxa",
      commander: "oracle-cmd",
      commander_print: "print-cmd",
      cards: [],
    });
    expect(url).toContain("/art/");
    expect(url).not.toBe(imageUrlByPrint("print-cmd"));
  });
});

describe("commanderPrintForRow", () => {
  it("returns the new print when the row is the commander", () => {
    expect(commanderPrintForRow("cmd", "cmd", "new-print")).toBe("new-print");
  });

  it("returns null for non-commander rows", () => {
    expect(commanderPrintForRow("cmd", "other", "new-print")).toBeNull();
  });
});
