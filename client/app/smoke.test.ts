import { Scene } from "foldkit/test";
import { describe, expect, it } from "vitest";
import { BindCardArt } from "../lib/ui/card-art";
import { init, Model, update } from "./main-exports";
import { CardArtTick, PortraitGateCancelled } from "./messages";
import type { Model as AppModel } from "./model";
import { HomeRoute, PlayRoute } from "./routes";
import { ClosedDeckListMenu } from "./shell/decks/list/messages";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape } from "./shell/decks/list/view";
import { view } from "./view";

const me = { id: 1, email: "alice@example.com", username: "alice" };

function playModel(overrides: Partial<AppModel>): AppModel {
  const [model] = init();

  return {
    ...model,
    route: PlayRoute(),
    portraitGate: { open: false },
    ...overrides,
  };
}

function homeWithDecks(): AppModel {
  const [model] = init();
  return {
    ...model,
    route: HomeRoute(),
    portraitGate: { open: false },
    sessionLoaded: true,
    session: { me },
    decks: {
      ...model.decks,
      list: {
        ...model.decks.list,
        loading: false,
        decks: [
          {
            commander: "atraxa",
            commander_print: "atraxa-print",
            id: 1,
            name: "Superfriends",
          },
        ],
      },
    },
  };
}

describe("foldkit scaffold", () => {
  it("init returns a ready model", () => {
    const [model] = init();

    expect(Model.make(model).ready).toBe(true);
    expect(update).toBeTypeOf("function");
  });

  it("does not render protected route content before the session loads", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: false, session: { me: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="session-gate"]')).toExist(),
      Scene.expect(Scene.text("Sign in")).not.toExist(),
      Scene.expect(Scene.text("Play")).not.toExist(),
    );
  });

  it("does not render protected route content for an unsigned loaded session", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: true, session: { me: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="session-gate"]')).toExist(),
      Scene.expect(Scene.text("Sign in")).not.toExist(),
    );
  });

  it("renders protected route content after authorization", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: true, session: { me } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).toExist(),
    );
  });

  it("treats CardArtTick as a no-op so BindCardArt mounts do not crash", () => {
    const [model] = init();
    const [next, commands] = update(model, CardArtTick());
    expect(next).toBe(model);
    expect(commands).toEqual([]);
  });

  it("survives BindCardArt mount on the post-login home deck list", () => {
    Scene.scene(
      { update, view },
      Scene.with(homeWithDecks()),
      Scene.expect(Scene.selector("[data-art-url]")).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("opens the portrait gate through a mount instead of an open attribute", () => {
    const portraitGateModal = { name: "OpenPortraitGateModal" };

    Scene.scene(
      { update, view },
      Scene.with(playModel({ portraitGate: { open: true } })),
      Scene.expect(Scene.selector("#portrait-gate")).not.toHaveAttr("open"),
      Scene.expect(Scene.selector("#portrait-gate")).toHaveHook("insert"),
      Scene.Mount.expectExact(portraitGateModal),
      Scene.Mount.resolve(portraitGateModal, PortraitGateCancelled()),
    );
  });
});
