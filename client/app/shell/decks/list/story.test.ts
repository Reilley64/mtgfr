import { Submodel } from "foldkit";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { BindCardArt, CardArtTick } from "../../../../lib/ui/card-art";
import type { CatalogCard } from "../../../../lib/wire/types";
import { ClearedDeckListHover } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { update } from "./update";
import { BindDeckListCommanderHover, view } from "./view";

const listView = Submodel.defineView<ReturnType<typeof initialDeckListSubmodel>, never>((model) =>
  view(model, "alice", null),
);
const listProgram = { update, view: listView };

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

test("commander hover preview renders when model carries hover state", () => {
  Scene.scene(
    { update: (m) => [m, []], view: listView },
    Scene.with({
      ...initialDeckListSubmodel(),
      hover: { id: "atraxa", print: "atraxa-print", x: 100, y: 200 },
      knownCommanders: {
        atraxa: {
          color_identity: [2, 4, 5],
          cost: { colored: [0, 0, 1, 1, 1], generic: 4 },
          default_print: "atraxa-print",
          id: "atraxa",
          keywords: [],
          kind: { kind: "creature", power: 4, toughness: 4 },
          legendary: true,
          name: "Atraxa, Praetors' Voice",
          oracle: "Flying, vigilance, deathtouch, lifelink",
          otags: [],
          set: "c16",
          subtypes: ["Angel", "Horror"],
          summary: "",
        },
      },
      decks: [
        {
          commander: "atraxa",
          commander_print: "atraxa-print",
          id: 1,
          name: "Superfriends",
        },
      ],
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-hover-preview"]')).toExist(),
    Scene.Mount.resolve(
      BindDeckListCommanderHover({ cardId: "atraxa", print: "atraxa-print" }),
      ClearedDeckListHover(),
    ),
    Scene.Mount.resolveAll([BindCardArt, CardArtTick()], [BindCardArt, CardArtTick()]),
  );
});

test("tile Play href uses ?deck= and search filters tiles", () => {
  const knownCommanders = {
    atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", color_identity: [0, 1, 2, 4] }),
    breena: card({ id: "breena", name: "Breena, the Demagogue" }),
    riku: card({ id: "riku", name: "Riku of Two Reflections" }),
  };
  const decks = [
    { id: 1, name: "Superfriends", commander: "atraxa", commander_print: "" },
    { id: -1, name: "Silverquill Influence", commander: "breena", commander_print: "" },
    { id: -9, name: "Mirror Mastery", commander: "riku", commander_print: "" },
  ];

  Scene.scene(
    listProgram,
    Scene.with({ ...initialDeckListSubmodel(), decks, knownCommanders }),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"][href="/play?deck=1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--1"]')).toExist(),
    Scene.Mount.resolveAll(
      [BindDeckListCommanderHover, ClearedDeckListHover()],
      [BindDeckListCommanderHover, ClearedDeckListHover()],
      [BindDeckListCommanderHover, ClearedDeckListHover()],
    ),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "mirror"),
    Scene.Mount.expectEnded(BindDeckListCommanderHover, BindDeckListCommanderHover),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.type(Scene.selector('[data-testid="deck-list-search"]'), "zzzz"),
    Scene.Mount.expectEnded(BindDeckListCommanderHover),
    Scene.expect(Scene.text("No decks match.")).toExist(),
  );
});
