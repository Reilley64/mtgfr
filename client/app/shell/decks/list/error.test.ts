import * as Menu from "@foldkit/ui/menu";
import { Scene } from "foldkit/test";
import { test } from "vitest";
import { ClosedDeckListMenu } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { BindDeckListContextMenuEscape, type ViewMessage, view } from "./view";

const emptyChrome = { version: null, faithfulCount: null, oracleTotal: null, coverageHref: null };

test("deck list errors use reconnect rust label styling", () => {
  Scene.scene<Record<string, never>, ViewMessage>(
    {
      update: (model) => [model, []],
      view: (_model, h) =>
        view(
          { ...initialDeckListSubmodel(), error: "Couldn't load decks." },
          {
            username: "alice",
            meGravatarHash: null,
            chrome: emptyChrome,
            accountMenu: Menu.init({ id: "account-menu" }),
          },
          h,
        ),
    },
    Scene.given({}),
    Scene.expect(Scene.selector('[role="alert"]')).toHaveClass("text-reconnect-rust"),
    Scene.expect(Scene.selector('[role="alert"]')).toHaveClass("text-label"),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});
