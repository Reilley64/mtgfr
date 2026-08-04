import { describe, expect, it } from "vitest";
import type { CatalogCard } from "~/wire/types";
import { cardTextOf, typeLineOf } from "./card-text";

function card(overrides: Partial<CatalogCard> = {}): CatalogCard {
  return {
    color_identity: [],
    cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    default_print: "print",
    id: "id",
    keywords: [],
    kind: { kind: "instant" },
    legendary: false,
    name: "Card",
    otags: [],
    set: "",
    sets: [],
    subtypes: [],
    summary: [],
    ...overrides,
  };
}

describe("typeLineOf", () => {
  it("reads like the printed line: types, then subtypes after a dash", () => {
    const bear = card({ kind: { kind: "creature", power: 2, toughness: 2 }, subtypes: ["Bear"] });
    expect(typeLineOf(bear)).toBe("Creature — Bear");
  });

  it("keeps a legend's supertype", () => {
    const rubinia = card({
      kind: { kind: "creature", power: 2, toughness: 3 },
      legendary: true,
      subtypes: ["Elf", "Advisor"],
    });
    expect(typeLineOf(rubinia)).toBe("Legendary Creature — Elf Advisor");
  });

  it("drops the dash when a card has no subtypes", () => {
    expect(typeLineOf(card({ kind: { kind: "sorcery" } }))).toBe("Sorcery");
  });
});

describe("cardTextOf", () => {
  it("carries the oracle text through, empty when the catalog has none", () => {
    expect(cardTextOf(card({ oracle: "Deals 3 damage to any target." }))).toEqual({
      typeLine: "Instant",
      oracle: "Deals 3 damage to any target.",
      flavor: "",
      flavorPrint: "print",
    });
    expect(cardTextOf(card({ oracle: null })).oracle).toBe("");
  });

  it("carries the flavor text through, empty when the printing prints none", () => {
    expect(cardTextOf(card({ flavor: "It watches." })).flavor).toBe("It watches.");
    expect(cardTextOf(card({ flavor: null })).flavor).toBe("");
  });
});
