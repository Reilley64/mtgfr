import { Submodel } from "foldkit";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { BindCardArt, CardArtTick } from "../../../../lib/ui/card-art";
import { ClearedDeckListHover } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { BindDeckListCommanderHover, view } from "./view";

const listView = Submodel.defineView<ReturnType<typeof initialDeckListSubmodel>, never>((model) =>
  view(model, "alice", null),
);

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
