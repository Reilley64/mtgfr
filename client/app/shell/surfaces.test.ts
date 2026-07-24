/**
 * Shell surface coverage — every auth / decks / lobby / 404 panel must appear here
 * (or in a focused sibling Scene test) with a data-testid or unique-copy assertion.
 * See AGENTS.md: "Client UI: every surface gets a Scene test."
 */
import { Scene } from "foldkit/test";
import { describe, it } from "vitest";
import { BindCardArt, CardArtTick } from "../../lib/ui/card-art";
import { ModalOpened, OpenDialogAsModal } from "../../lib/ui/confirmDialog";
import type { CatalogCard } from "../../lib/wire/types";
import { init, update } from "../main-exports";
import type { Model as AppModel } from "../model";
import { HomeRoute, LoginRoute, NewDeckRoute, NotFoundRoute, PlayRoute, TableRoute } from "../routes";
import { view } from "../view";
import { ClearedBuilderHover } from "./decks/builder/messages";
import { initialDeckBuilderSubmodel } from "./decks/builder/submodel";
import { BindBuilderCardPointer } from "./decks/builder/view";
import { ClosedDeckListMenu } from "./decks/list/messages";
import { BindDeckListContextMenu, BindDeckListContextMenuEscape } from "./decks/list/view";
import { initialLobbySlice } from "./lobby/submodel";

const me = { id: 1, email: "alice@example.com", username: "alice" };

const atraxa = card({
  color_identity: [2, 4, 5],
  cost: { colored: [0, 0, 1, 1, 1], generic: 4 },
  default_print: "atraxa-print",
  id: "atraxa",
  kind: { kind: "creature", power: 4, toughness: 4 },
  legendary: true,
  name: "Atraxa, Praetors' Voice",
  oracle: "Flying, vigilance, deathtouch, lifelink",
  set: "c16",
  subtypes: ["Angel", "Horror"],
});

const solRing = card({
  default_print: "sol-ring-print",
  id: "sol-ring",
  name: "Sol Ring",
});

const deck = {
  commander: "atraxa",
  commander_print: "atraxa-print",
  id: 1,
  name: "Superfriends",
};

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

function loginModel(overrides: Partial<AppModel> = {}): AppModel {
  const [model] = init();
  return {
    ...model,
    route: LoginRoute(),
    portraitGate: { open: false },
    sessionLoaded: true,
    session: { me: null },
    ...overrides,
  };
}

function authedModel(route: AppModel["route"], overrides: Partial<AppModel> = {}): AppModel {
  const [model] = init();
  return {
    ...model,
    route,
    portraitGate: { open: false },
    sessionLoaded: true,
    session: { me },
    ...overrides,
  };
}

describe("shell surface scenes", () => {
  it("renders auth login surfaces from the app view", () => {
    Scene.scene(
      { update, view },
      Scene.with(loginModel({ apiVersion: "1.2.3" })),
      Scene.expect(Scene.selector('[data-testid="auth-panel"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-form"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-email"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-password"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-submit"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="app-version"]')).toExist(),
      Scene.expect(Scene.text("API 1.2.3")).toExist(),
      Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
      Scene.expect(Scene.text("mtgfr")).not.toExist(),
    );
  });

  it("renders auth signup surfaces and auth errors", () => {
    const [model] = init();

    Scene.scene(
      { update, view },
      Scene.with(
        loginModel({
          auth: {
            ...model.auth,
            error: "Email already in use.",
            mode: "signup",
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="auth-username"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-toggle-mode"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="auth-error"]')).toExist(),
      Scene.expect(Scene.text("Email already in use.")).toExist(),
    );
  });

  it("renders deck list chrome and tiles", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              decks: [deck],
              knownCommanders: { atraxa },
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="decks-page"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="delete-deck-1"]')).not.toExist(),
      Scene.expect(Scene.text("Your decks")).toExist(),
      Scene.expect(Scene.text("Superfriends")).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders the deck delete confirmation dialog", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              confirmingDeleteId: 1,
              decks: [deck],
              knownCommanders: { atraxa },
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="confirm-delete-dialog"]')).toExist(),
      Scene.expect(Scene.text('Delete "Superfriends"?')).toExist(),
      Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
      Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders deck list empty copy", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              decks: [],
              loading: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.text("No decks yet — build one to get started.")).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders deck list loading copy", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(HomeRoute(), {
          decks: {
            ...init()[0].decks,
            list: {
              ...init()[0].decks.list,
              loading: true,
            },
          },
        }),
      ),
      Scene.expect(Scene.text("Loading decks…")).toExist(),
      Scene.Mount.resolve(BindDeckListContextMenuEscape(), ClosedDeckListMenu()),
    );
  });

  it("renders deck builder chrome, problems, and builder mounts", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(NewDeckRoute(), {
          decks: {
            ...init()[0].decks,
            builder: {
              ...initialDeckBuilderSubmodel(),
              atEnd: true,
              confirmingDiscard: true,
              known: { "sol-ring": solRing },
              pool: [solRing],
              preferredPrint: { "sol-ring": "sol-ring-print" },
              problems: ["Choose a commander first."],
              searching: false,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="deck-builder-page"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-name"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="save-deck"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="builder-cancel"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="builder-pool-hint"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="deck-problems"]')).toExist(),
      Scene.expect(Scene.text("Choose a commander first.")).toExist(),
      Scene.Mount.resolve(BindBuilderCardPointer({ cardId: "sol-ring", kind: "pool" }), ClearedBuilderHover()),
      Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
    );
  });

  it("renders lobby entry join surfaces with decks", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(PlayRoute({ deckId: "1" }), {
          decks: {
            ...init()[0].decks,
            list: { ...init()[0].decks.list, decks: [deck], knownCommanders: { atraxa }, loading: false },
          },
          lobby: { ...initialLobbySlice(), selectedDeckId: 1 },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="lobby-deck-card"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-deck-card-1"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-join-code"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-join"]')).toExist(),
      Scene.expect(Scene.text("Lobby")).toExist(),
      Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
      Scene.expect(Scene.text("mtgfr")).not.toExist(),
      Scene.Mount.resolve(BindCardArt, CardArtTick()),
    );
  });

  it("renders lobby table chrome, seats, and errors", () => {
    Scene.scene(
      { update, view },
      Scene.with(
        authedModel(TableRoute({ deckId: "1", table: "ABC123" }), {
          decks: {
            ...init()[0].decks,
            list: { ...init()[0].decks.list, decks: [deck], knownCommanders: { atraxa }, loading: false },
          },
          lobby: {
            ...initialLobbySlice(),
            error: "UnknownTable",
            selectedDeckId: 1,
            tableId: "ABC123",
            view: {
              error: null,
              seats: [
                {
                  claimed: true,
                  deck_id: 1,
                  deck_name: "Superfriends",
                  is_host: true,
                  is_you: true,
                  player: 0,
                  ready: false,
                  username: "alice",
                },
              ],
              start_error: "NeedTwoPlayers",
              started: false,
              table_id: "ABC123",
              you: 0,
            },
          },
        }),
      ),
      Scene.expect(Scene.selector('[data-testid="lobby-table-code"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-copy-code"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-seats"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-seat-0"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-start-error"]')).toExist(),
      Scene.expect(Scene.selector('[data-testid="lobby-error"]')).toExist(),
      Scene.expect(Scene.text("Need at least two players.")).toExist(),
      Scene.expect(Scene.text("No such table.")).toExist(),
    );
  });

  it("renders the app not-found route", () => {
    Scene.scene(
      { update, view },
      Scene.with(authedModel(NotFoundRoute({ path: "/missing" }))),
      Scene.expect(Scene.text("Not found")).toExist(),
      Scene.expect(Scene.text("No Foldkit route for /missing.")).toExist(),
    );
  });

  // The board-mount placeholder is unreachable through routeBody today:
  // PlayRoute/TableRoute only call boardMount when model.game?.active === true,
  // and boardMount immediately renders the board submodel whenever model.game exists.
});
