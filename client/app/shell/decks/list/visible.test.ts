import { describe, expect, it } from "vitest";
import type { CatalogCard } from "../../../../lib/wire/types";
import { deckListContextMenuAllowed, identityPipCodes, visibleDecks } from "./visible";

const card = (id: string, name: string, color_identity: number[] = []): CatalogCard => ({
  color_identity,
  cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
  default_print: `${id}-print`,
  id,
  keywords: [],
  kind: { kind: "creature", power: 1, toughness: 1 },
  legendary: true,
  name,
  oracle: "",
  otags: [],
  set: "tst",
  subtypes: [],
  summary: "",
});

describe("visibleDecks", () => {
  const decks = [
    { id: 2, name: "Beta", commander: "b", commander_print: "" },
    { id: -1, name: "Silverquill Influence", commander: "s", commander_print: "" },
    { id: 1, name: "Alpha", commander: "a", commander_print: "" },
    { id: -9, name: "Mirror Mastery", commander: "m", commander_print: "" },
    { id: -5, name: "Quandrix Unlimited", commander: "q", commander_print: "" },
  ];
  const known = {
    a: card("a", "Atraxa, Praetors' Voice"),
    b: card("b", "Beledros Witherbloom"),
    m: card("m", "Riku of Two Reflections"),
    s: card("s", "Breena, the Demagogue"),
    q: card("q", "Adrix and Nev, Timelocked"),
  };

  it("puts customs first preserving relative order, then precons by ascending id", () => {
    const ids = visibleDecks(decks, known, "").map((d) => d.id);
    expect(ids).toEqual([2, 1, -9, -5, -1]);
  });

  it("filters by deck name case-insensitively", () => {
    expect(visibleDecks(decks, known, "mirror").map((d) => d.id)).toEqual([-9]);
  });

  it("filters by commander display name", () => {
    expect(visibleDecks(decks, known, "atraxa").map((d) => d.id)).toEqual([1]);
  });

  it("falls back to commander id when unknown", () => {
    const orphan = [{ id: 9, name: "Orphan", commander: "mystery-id", commander_print: "" }];
    expect(visibleDecks(orphan, {}, "mystery").map((d) => d.id)).toEqual([9]);
  });

  it("returns empty when nothing matches a non-empty library filter", () => {
    expect(visibleDecks(decks, known, "zzzz").map((d) => d.id)).toEqual([]);
  });
});

describe("identityPipCodes", () => {
  it("maps WUBRG indices in order given", () => {
    expect(identityPipCodes([0, 2, 4])).toEqual(["W", "B", "G"]);
  });
  it("skips out-of-range indices", () => {
    expect(identityPipCodes([-1, 5, 1])).toEqual(["U"]);
  });
});

describe("deckListContextMenuAllowed", () => {
  it("allows owned decks only", () => {
    expect(deckListContextMenuAllowed(1)).toBe(true);
    expect(deckListContextMenuAllowed(-1)).toBe(false);
    expect(deckListContextMenuAllowed(0)).toBe(false);
  });
});
