import { describe, expect, it } from "vitest";
import { commanderPrintForRow, formatReleasedAt, reconcileEntries } from "~/lib/deckBuilderPrint";

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

describe("commanderPrintForRow", () => {
  it("returns the new print when the row is the commander", () => {
    expect(commanderPrintForRow("cmd", "cmd", "new-print")).toBe("new-print");
  });

  it("returns null for non-commander rows", () => {
    expect(commanderPrintForRow("cmd", "other", "new-print")).toBeNull();
  });
});
