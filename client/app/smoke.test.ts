import { readFileSync } from "node:fs";
import { Scene } from "foldkit/test";
import { describe, expect, it } from "vitest";
import { BindDeckCardFlip, DeckCardFlipTick } from "./deck-card-nav";
import { BindCardArt } from "./domain/ui/card-art";
import { init, Model, update } from "./main-exports";
import { CardArtTick } from "./messages";
import type { Model as AppModel } from "./model";
import { HomeRoute, NewDeckRoute, PlayRoute } from "./routes";
import { MeasuredPoolGrid } from "./shell/decks/builder/messages";
import { ObservePoolWidth } from "./shell/decks/builder/view";
import { ClosedDeckListMenu } from "./shell/decks/list/messages";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape } from "./shell/decks/list/view";
import { view } from "./view";

const me = { id: 1, email: "alice@example.com", username: "alice" };
const deck = {
  commander: "atraxa",
  commander_print: "atraxa-print",
  id: 1,
  name: "Superfriends",
};

function playModel(overrides: Partial<AppModel>): AppModel {
  const [model] = init();

  return {
    ...model,
    route: PlayRoute({ deckId: "1" }),
    landscapeRotate: { active: false },
    decks: {
      ...model.decks,
      list: {
        ...model.decks.list,
        loading: false,
        decks: [deck],
      },
    },
    lobby: { ...model.lobby, selectedDeckId: 1 },
    ...overrides,
  };
}

function homeWithDecks(): AppModel {
  const [model] = init();
  return {
    ...model,
    route: HomeRoute(),
    landscapeRotate: { active: false },
    sessionLoaded: true,
    session: { me, meGravatarHash: null },
    decks: {
      ...model.decks,
      list: {
        ...model.decks.list,
        loading: false,
        decks: [deck],
      },
    },
  };
}

/** The key snabbdom uses to decide whether a route change reuses the previous surface's DOM. */
function surfaceKeyOf(html: unknown): unknown {
  return (html as { children?: ReadonlyArray<{ key?: unknown }> }).children?.[0]?.key;
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
      Scene.with(playModel({ sessionLoaded: false, session: { me: null, meGravatarHash: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="session-gate"]')).toExist(),
      Scene.expect(Scene.text("Sign in")).not.toExist(),
      Scene.expect(Scene.text("Play")).not.toExist(),
    );
  });

  it("does not render protected route content for an unsigned loaded session", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: true, session: { me: null, meGravatarHash: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="session-gate"]')).toExist(),
      Scene.expect(Scene.text("Sign in")).not.toExist(),
    );
  });

  it("renders protected route content after authorization", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ sessionLoaded: true, session: { me, meGravatarHash: null } })),
      Scene.expect(Scene.selector('[data-testid="lobby"]')).toExist(),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
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
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  // Regression: every shell route roots at the same unkeyed `<main>`, so a route change patched the
  // outgoing surface's elements into the incoming one. `h.OnMount` runs on element creation, so the
  // reused elements never started their mounts — after a client-side navigation the deck builder's
  // pool grid was never measured and collapsed to one column. Distinct keys force a real remount.
  it("gives the deck list and the deck builder different surface keys, so navigating remounts", () => {
    const list: { key?: unknown } = {};
    const builder: { key?: unknown } = {};

    Scene.scene(
      { update, view },
      Scene.with(homeWithDecks()),
      Scene.tap((simulation) => {
        list.key = surfaceKeyOf(simulation.html);
      }),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
    Scene.scene(
      { update, view },
      Scene.with({ ...homeWithDecks(), route: NewDeckRoute() }),
      Scene.tap((simulation) => {
        builder.key = surfaceKeyOf(simulation.html);
      }),
      Scene.Mount.resolve(ObservePoolWidth(), MeasuredPoolGrid({ width: 800 })),
    );

    expect(list.key).toBe("deck-list");
    expect(builder.key).toBe("deck-builder");
  });

  it("applies landscape rotate class instead of a portrait dialog", () => {
    Scene.scene(
      { update, view },
      Scene.with(playModel({ landscapeRotate: { active: true } })),
      Scene.expect(Scene.selector("#portrait-gate")).not.toExist(),
      Scene.expect(Scene.selector('[data-testid="landscape-root"]')).toHaveClass("landscape-rotate-root"),
    );
  });

  it("keeps the mobile safe-area contract for landscape rotate", () => {
    const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
    const globalCss = readFileSync(new URL("../styles/global.css", import.meta.url), "utf8");

    expect(indexHtml).toContain("viewport-fit=cover");
    expect(globalCss).toContain("env(safe-area-inset-top)");
    expect(globalCss).toContain("env(safe-area-inset-right)");
    expect(globalCss).toContain("env(safe-area-inset-bottom)");
    expect(globalCss).toContain("env(safe-area-inset-left)");
  });

  it("disables shell stage enter animation under reduced motion", () => {
    const globalCss = readFileSync(new URL("../styles/global.css", import.meta.url), "utf8");
    expect(globalCss).toContain(".shell-stage-enter");
    expect(globalCss).toMatch(
      /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*\.shell-stage-enter\s*\{\s*animation:\s*none;/,
    );
    expect(globalCss).toContain("var(--ease-state)");
  });
});
