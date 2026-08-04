// The bar draws full card faces, and a face prints words the game state never sends — type line and
// rules text. Those come from the catalog, once per card id.

import { expect, test } from "vitest";
import type { CatalogCard, ObjectView, VisibleState } from "~/wire/types";
import { cardTextOf } from "../domain/card-render/card-text";
import type { GameFoldState } from "../game/fold";
import { ZONE } from "./geometry/layout";
import { CardTextFetched, PrintFlavorFetched } from "./messages";
import { type BoardModel, initialBoardModel, requestBarCardText, updateBoard } from "./submodel";

function object(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: `Card ${id}`,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
    ...overrides,
  };
}

function state(objects: ObjectView[]): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects,
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function fold(objects: ObjectView[]): GameFoldState {
  return { seq: 1, state: state(objects), log: [] } as unknown as GameFoldState;
}

function catalogCard(id: string, overrides: Partial<CatalogCard> = {}): CatalogCard {
  return {
    color_identity: [],
    cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    default_print: "print",
    id,
    keywords: [],
    kind: { kind: "instant" },
    legendary: false,
    name: id,
    otags: [],
    set: "",
    sets: [],
    subtypes: [],
    summary: [],
    ...overrides,
  };
}

test("asks the catalog for the text of the cards the bar draws", () => {
  const [model, commands] = requestBarCardText(
    initialBoardModel(),
    fold([
      object(1, { card_id: "bolt" }),
      object(2, { card_id: "atraxa", zone: ZONE.Command }),
      object(3, { card_id: "swamp", zone: ZONE.Graveyard }),
    ]),
  );

  expect(commands).toHaveLength(1);
  expect(commands[0]?.args).toEqual({ cardIds: ["bolt", "atraxa", "swamp"] });
  expect([...model.cardText.keys()]).toEqual(["bolt", "atraxa", "swamp"]);
});

test("leaves out cards the bar never draws — the battlefield square has no text, and hands are private", () => {
  const [, commands] = requestBarCardText(
    initialBoardModel(),
    fold([object(1, { card_id: "bear", zone: ZONE.Battlefield }), object(2, { card_id: "theirs", owner: 1 })]),
  );

  expect(commands).toEqual([]);
});

test("asks once — a card already asked for is not asked for again on the next fold", () => {
  const cards = fold([object(1, { card_id: "bolt" })]);
  const [asked] = requestBarCardText(initialBoardModel(), cards);
  const [, again] = requestBarCardText(asked, cards);

  expect(again).toEqual([]);
});

test("folds the catalog reply into the text the face draws", () => {
  const model: BoardModel = { ...initialBoardModel(), cardText: new Map([["bolt", null]]) };
  const [next] = updateBoard(
    model,
    CardTextFetched({
      cards: [
        catalogCard("bolt", {
          kind: { kind: "creature", power: 2, toughness: 2 },
          subtypes: ["Elemental"],
          oracle: "Haste.",
          flavor: "Fast as fire.",
        }),
      ],
    }),
    fold([]),
    "T1",
  );

  expect(next.cardText.get("bolt")).toEqual({
    typeLine: "Creature — Elemental",
    oracle: "Haste.",
    flavor: "Fast as fire.",
    flavorPrint: "print",
  });
});

// Flavor is per printing. The catalog only knows the card's default printing, so a deck playing a
// reprint would otherwise set the wrong quote under the art it actually shows.
test("asks for the flavor of the printing the deck plays, and drops the default printing's words", () => {
  const model: BoardModel = {
    ...initialBoardModel(),
    cardText: new Map([
      ["terminate", cardTextOf(catalogCard("terminate", { flavor: "I think, therefore I annihilate!" }))],
    ]),
  };

  const [next, commands] = requestBarCardText(model, fold([object(1, { card_id: "terminate", print: "c11-print" })]));

  expect(commands).toHaveLength(1);
  expect(commands[0]?.args).toEqual({ cards: [{ cardId: "terminate", print: "c11-print" }] });
  expect(next.cardText.get("terminate")?.flavor).toBe("");
});

test("keeps the catalog flavor when the deck plays the card's default printing", () => {
  const model: BoardModel = {
    ...initialBoardModel(),
    cardText: new Map([["bolt", cardTextOf(catalogCard("bolt", { flavor: "Fast as fire." }))]]),
  };

  const [next, commands] = requestBarCardText(model, fold([object(1, { card_id: "bolt", print: "print" })]));

  expect(commands).toEqual([]);
  expect(next.cardText.get("bolt")?.flavor).toBe("Fast as fire.");
});

test("asks for a printing's flavor once — the next fold does not ask again", () => {
  const model: BoardModel = {
    ...initialBoardModel(),
    cardText: new Map([["terminate", cardTextOf(catalogCard("terminate", { flavor: "Old words." }))]]),
  };
  const cards = fold([object(1, { card_id: "terminate", print: "c11-print" })]);
  const [asked] = requestBarCardText(model, cards);
  const [, again] = requestBarCardText(asked, cards);

  expect(again).toEqual([]);
});

test("corrects the printing as soon as the catalog reply lands, without waiting for a state delta", () => {
  const model: BoardModel = { ...initialBoardModel(), cardText: new Map([["terminate", null]]) };

  const [next, commands] = updateBoard(
    model,
    CardTextFetched({ cards: [catalogCard("terminate", { flavor: "I think, therefore I annihilate!" })] }),
    fold([object(1, { card_id: "terminate", print: "c11-print" })]),
    "T1",
  );

  expect(commands).toHaveLength(1);
  expect(commands[0]?.args).toEqual({ cards: [{ cardId: "terminate", print: "c11-print" }] });
  expect(next.cardText.get("terminate")?.flavor).toBe("");
});

test("sets the printing's own flavor when it lands", () => {
  const model: BoardModel = {
    ...initialBoardModel(),
    cardText: new Map([
      ["terminate", { ...cardTextOf(catalogCard("terminate")), flavor: "", flavorPrint: "c11-print" }],
    ]),
  };

  const [next] = updateBoard(
    model,
    PrintFlavorFetched({ flavors: [{ cardId: "terminate", print: "c11-print", flavor: "I've seen death before." }] }),
    fold([]),
    "T1",
  );

  expect(next.cardText.get("terminate")?.flavor).toBe("I've seen death before.");
});
