import { describe, expect, it } from "vitest";
import type { CatalogCard } from "../wire/types";
import { DECK_SIZE } from "./cards";
import { commanderMenuItems, poolMenuItems, rowMenuItems } from "./menu";

function card(overrides: Partial<CatalogCard> = {}): CatalogCard {
  return {
    color_identity: [],
    cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    default_print: `${overrides.id ?? "card"}-print`,
    id: "card",
    keywords: [],
    kind: { kind: "artifact" },
    legendary: false,
    name: "Card",
    otags: [],
    set: "tst",
    subtypes: [],
    summary: "",
    ...overrides,
  };
}

describe("poolMenuItems", () => {
  it("offers bulk add and fill for basics, plus choose print when not in deck", () => {
    const island = card({ id: "island", kind: { kind: "land", colors: [1] }, name: "Island" });
    const items = poolMenuItems({ card: island, inDeck: false, total: 90 });
    expect(items.map((i) => i.label)).toEqual(["Add One", "Add Two", "Add Five", "Fill deck", "Choose print"]);
    expect(items.find((i) => i.label === "Fill deck")?.action).toEqual({
      kind: "fill",
      cardId: "island",
      count: DECK_SIZE - 90,
    });
  });

  it("offers set-as-commander for legendary creatures and hides choose print when already in deck", () => {
    const commander = card({
      id: "cmd",
      kind: { kind: "creature", power: 2, toughness: 2 },
      legendary: true,
      name: "Commander",
    });
    const items = poolMenuItems({ card: commander, inDeck: true, total: 0 });
    expect(items.map((i) => i.label)).toEqual(["Add One", "Set As Commander"]);
  });

  it("offers a single add for ordinary non-basics", () => {
    const solRing = card({ id: "sol-ring", name: "Sol Ring" });
    expect(poolMenuItems({ card: solRing, inDeck: false, total: 0 }).map((i) => i.label)).toEqual([
      "Add One",
      "Choose print",
    ]);
  });
});

describe("rowMenuItems", () => {
  it("adds bulk remove for basics plus choose print", () => {
    const island = card({ id: "island", kind: { kind: "land", colors: [1] }, name: "Island" });
    const items = rowMenuItems({ card: island, total: 50 });
    expect(items.map((i) => i.label)).toEqual(["Fill deck", "Remove 1", "Remove 2", "Remove 5", "Choose print"]);
  });

  it("offers only choose print for non-basics", () => {
    const solRing = card({ id: "sol-ring", name: "Sol Ring" });
    expect(rowMenuItems({ card: solRing, total: 0 }).map((i) => i.label)).toEqual(["Choose print"]);
  });
});

describe("commanderMenuItems", () => {
  it("only offers choose print", () => {
    expect(commanderMenuItems({ cardId: "cmd" }).map((i) => i.label)).toEqual(["Choose print"]);
  });
});
