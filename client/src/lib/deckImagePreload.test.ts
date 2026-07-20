import { describe, expect, it } from "vitest";
import { printIdsFromDeck } from "~/lib/deckImagePreload";
import type { DeckDetail } from "~/wire/types";

describe("printIdsFromDeck", () => {
  it("collects unique commander and card prints, skipping empties", () => {
    const deck: DeckDetail = {
      id: 1,
      name: "Test",
      commander: "oracle-1",
      commander_print: "print-cmd",
      cards: [
        { id: "a", count: 1, print: "print-a" },
        { id: "b", count: 2, print: "print-a" },
        { id: "c", count: 1, print: "" },
        { id: "d", count: 1, print: "print-d" },
      ],
    };

    expect(printIdsFromDeck(deck).sort()).toEqual(["print-a", "print-cmd", "print-d"]);
  });
});
