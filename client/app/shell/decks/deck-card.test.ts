import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { BindCardArt, CardArtTick } from "../../../lib/ui/card-art";
import { renderDeckCard } from "./deck-card";

const h = html<never>();

const view = () =>
  renderDeckCard(
    h,
    {
      id: 7,
      name: "Superfriends",
      commander: "atraxa",
      commanderName: "Atraxa, Praetors' Voice",
      print: "atraxa-print",
      colorIdentity: [0, 1, 2, 3, 4],
    },
    { mode: "static", testId: "lobby-deck-card-7" },
  );

test("static deck card exposes testid, name, commander, precon chip absent", () => {
  Scene.scene(
    { update: (m) => [m, []], view: () => ({ title: "t", body: view() }) },
    Scene.with(null),
    Scene.expect(Scene.testId("lobby-deck-card-7")).toExist(),
    Scene.expect(Scene.text("Superfriends")).toExist(),
    Scene.expect(Scene.text("Atraxa, Praetors' Voice")).toExist(),
    Scene.expect(Scene.text("Precon")).not.toExist(),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("link deck card uses an anchor with href and view transition", () => {
  Scene.scene(
    {
      update: (m) => [m, []],
      view: () => ({
        title: "t",
        body: renderDeckCard(
          h,
          {
            id: -2,
            name: "Lorehold Legacies",
            commander: "osgir",
            commanderName: "Osgir, the Reconstructor",
            print: "",
            colorIdentity: [0, 3],
          },
          { mode: "link", href: "/play/-2", testId: "deck-tile--2" },
        ),
      }),
    },
    Scene.with(null),
    Scene.expect(Scene.selector('a[data-testid="deck-tile--2"][href="/play/-2"]')).toExist(),
    Scene.expect(Scene.testId("deck-tile--2")).toHaveStyle("view-transition-name", "deck-card--2"),
    Scene.expect(Scene.text("Precon")).toExist(),
    Scene.expect(Scene.text("Osgir, the Reconstructor")).toExist(),
  );
});
