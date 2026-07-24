import { Scene } from "foldkit/test";
import { test } from "vitest";
import { ClosedDeckListMenu } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { BindDeckListContextMenuEscape, view } from "./view";

test("deck list errors use reconnect rust label styling", () => {
  Scene.scene(
    {
      update: (model) => [model, []],
      view: () => view({ ...initialDeckListSubmodel(), error: "Couldn't load decks." }, "alice", null),
    },
    Scene.with({}),
    Scene.expect(Scene.selector('[role="alert"]')).toHaveClass("text-reconnect-rust"),
    Scene.expect(Scene.selector('[role="alert"]')).toHaveClass("text-label"),
    Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
  );
});
