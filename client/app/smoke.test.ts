import { Scene } from "foldkit/test";
import { describe, expect, it } from "vitest";
import { init, Model, update } from "./main-exports";
import { CardArtTick, PortraitGateCancelled } from "./messages";
import type { Model as AppModel } from "./model";
import { PlayRoute } from "./routes";
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
    );
  });

  it("does not render protected route content for an unsigned loaded session", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: true, session: { me: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).not.toExist(),
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
