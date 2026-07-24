import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { init, update } from "../../main-exports";
import type { Model } from "../../model";
import { PlayRoute, TableRoute } from "../../routes";
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

function playLobbyModel(overrides: Partial<Model>): Model {
  const [model] = init();
  return {
    ...model,
    route: PlayRoute({ deckId: "0" }),
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
  lobbyView(model.lobby, model.decks.list.decks, model.decks.list.loading, model.apiVersion);

test("shows deck picker when no deck is selected yet", () => {
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
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-host"]')).toExist(),
    Scene.expect(Scene.text("Pick a deck to play first (Your decks → Play).")).not.toExist(),
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
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).not.toExist(),
    Scene.expect(Scene.text("your deck")).toExist(),
  );
});

test("entry with pre-picked deck shows Bring text and Back, not a select", () => {
  const other = {
    id: 9,
    name: "Tokens",
    commander: "rhys",
    commander_print: undefined as string | undefined,
  };
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck, other] },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-bring")).toExist(),
    Scene.expect(Scene.text("Tokens")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.text("Back")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
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
    Scene.expect(Scene.selector('[data-testid="lobby-bring"]')).toExist(),
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

test("host redirect preserves ?deck= on the table URL", () => {
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
  expect(redirect?.args?.path).toBe("/play/7/XYZ789?deck=7");
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
