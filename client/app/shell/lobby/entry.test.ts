import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { BindCardArt, CardArtTick } from "../../../lib/ui/card-art";
import type { CatalogCard } from "../../../lib/wire/types";
import { ReceivedDecks } from "../../messages";
import { init, update } from "../../main-exports";
import type { Model } from "../../model";
import { PlayRoute, TableRoute } from "../../routes";
import { view as appView } from "../../view";
import { LobbyTableCreated } from "./messages";
import { initialLobbySlice } from "./submodel";
import { view as lobbyView } from "./view";

const me = { id: 1, email: "alice@example.com", username: "alice" };

const deck = {
  id: 7,
  name: "Superfriends",
  commander: "atraxa",
  commander_print: undefined as string | undefined,
};

const other = {
  id: 9,
  name: "Tokens",
  commander: "rhys",
  commander_print: undefined as string | undefined,
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

function playLobbyModel(overrides: Partial<Model>): Model {
  const [model] = init();
  return {
    ...model,
    route: PlayRoute({ deckId: "7" }),
    sessionLoaded: true,
    session: { me },
    ...overrides,
  };
}

function tableLobbyModel(overrides: Partial<Model>): Model {
  const [model] = init();
  return {
    ...model,
    route: TableRoute({ deckId: "7", table: "ABC123" }),
    sessionLoaded: true,
    session: { me },
    ...overrides,
  };
}

const lobbyAppView = (model: Model) =>
  lobbyView(
    model.lobby,
    model.decks.list.decks,
    model.decks.list.loading,
    model.decks.list.knownCommanders,
    model.apiVersion,
  );

test("entry without a route deck asks the player to use deck play", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: null },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck] },
        },
      }),
    ),
    Scene.expect(Scene.text("Pick a deck to play first (Your decks → Play).")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-host"]')).toBeAbsent(),
  );
});

test("shows build-a-deck copy when the player has no decks", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: null },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [], loading: false },
        },
      }),
    ),
    Scene.expect(Scene.text("Build a deck first (Your decks → New deck).")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-host"]')).not.toExist(),
  );
});

test("keeps entry visible while decks load when a deck is selected", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: 7 },
        decks: { ...init()[0].decks, list: { ...init()[0].decks.list, decks: [], loading: true } },
      }),
    ),
    Scene.expect(Scene.selector('[data-testid="lobby-host"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck-card"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).not.toExist(),
    Scene.expect(Scene.text("Loading decks…")).toExist(),
  );
});

test("entry shows deck card and Back, never a select", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        route: PlayRoute({ deckId: "9" }),
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: {
            ...init()[0].decks.list,
            decks: [deck, other],
            knownCommanders: { rhys: card({ id: "rhys", name: "Rhys the Redeemed" }) },
            loading: false,
          },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-deck-card")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card-9")).toExist(),
    Scene.expect(Scene.text("Tokens")).toExist(),
    Scene.expect(Scene.text("Rhys the Redeemed")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.text("Back")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-bring"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("unknown deck after load shows not-found, not lobby", () => {
  const [next] = update(
    playLobbyModel({
      currentPath: "/play/99",
      route: PlayRoute({ deckId: "99" }),
      lobby: { ...initialLobbySlice(), selectedDeckId: 99 },
      decks: {
        ...init()[0].decks,
        list: { ...init()[0].decks.list, decks: [], loading: true },
      },
    }),
    ReceivedDecks({ decks: [deck] }),
  );

  expect(next.route._tag).toBe("NotFoundRoute");
  Scene.scene(
    { update, view: appView },
    Scene.with(next),
    Scene.expect(Scene.text("Not found")).toExist(),
    Scene.expect(Scene.text("No Foldkit route for /play/99.")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby"]')).toBeAbsent(),
  );
});

test("claim seat with a pre-chosen deck has no picker", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      tableLobbyModel({
        lobby: {
          ...initialLobbySlice(),
          tableId: "ABC123",
          selectedDeckId: 7,
          view: {
            table_id: "ABC123",
            you: null,
            started: false,
            error: null,
            start_error: null,
            seats: [
              {
                player: 0,
                claimed: false,
                username: null,
                deck_name: null,
                deck_id: null,
                ready: false,
                is_host: false,
                is_you: false,
              },
            ],
          },
        },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck] },
        },
      }),
    ),
    Scene.expect(Scene.selector('[data-testid="lobby-deck-card"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck-card-7"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-bring"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-back"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-claim"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-ready"]')).not.toExist(),
  );
});

test("claim seat pre-pick includes Back to decks", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      tableLobbyModel({
        lobby: {
          ...initialLobbySlice(),
          tableId: "ABC123",
          selectedDeckId: 7,
          view: {
            table_id: "ABC123",
            you: null,
            started: false,
            error: null,
            start_error: null,
            seats: [
              {
                player: 0,
                claimed: false,
                username: null,
                deck_name: null,
                deck_id: null,
                ready: false,
                is_host: false,
                is_you: false,
              },
            ],
          },
        },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck] },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
  );
});

test("host redirect uses /play/:deckId/:table", () => {
  const [model] = init();
  const withDeck = playLobbyModel({
    lobby: { ...initialLobbySlice(), selectedDeckId: 7 },
    decks: {
      ...model.decks,
      list: { ...model.decks.list, decks: [deck] },
    },
  });

  const [, commands] = update(withDeck, LobbyTableCreated({ tableId: "XYZ789" }));
  const redirect = commands.find((c) => c.name === "Redirect") as { args?: { path?: string } } | undefined;
  expect(redirect?.args?.path).toBe("/play/7/XYZ789");
});

test("joined lobby shows ready/start without a deck picker", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      tableLobbyModel({
        lobby: {
          ...initialLobbySlice(),
          tableId: "ABC123",
          selectedDeckId: 7,
          view: {
            table_id: "ABC123",
            you: 0,
            started: false,
            error: null,
            start_error: "NeedTwoPlayers",
            seats: [
              {
                player: 0,
                claimed: true,
                username: "alice",
                deck_name: "Superfriends",
                deck_id: 7,
                ready: false,
                is_host: true,
                is_you: true,
              },
            ],
          },
        },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck] },
        },
      }),
    ),
    Scene.expect(Scene.selector('[data-testid="lobby-ready"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-start"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-claim"]')).not.toExist(),
  );
});
