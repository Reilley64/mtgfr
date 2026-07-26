import { Option } from "effect";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { BindDeckCardFlip, DeckCardFlipTick } from "../../deck-card-nav";
import { BindCardArt, CardArtTick } from "../../domain/ui/card-art";
import type { CatalogCard } from "../../domain/wire/types";
import { init, update } from "../../main-exports";
import { GotAuthMessage, GotDeckListMessage, GotLobbyMessage, type Message } from "../../messages";
import type { Model } from "../../model";
import { PlayRoute, PregameTableRoute } from "../../routes";
import { view as appView } from "../../view";
import * as Auth from "../auth";
import * as DeckList from "../decks/list";
import { LobbyTableCreated, RequestedLobbyCancelJoin, RequestedLobbyOpenJoin } from "./messages";
import { initialLobbySlice } from "./submodel";
import { type ViewMessage as LobbyViewMessage, view as lobbyView } from "./view";

const me = { id: 1, email: "alice@example.com", username: "alice" };
const h = html<Message>();

const url = (pathname: string, search = "") => ({
  protocol: "http:",
  host: "localhost",
  port: Option.none<string>(),
  pathname,
  search: search === "" ? Option.none<string>() : Option.some(search),
  hash: Option.none<string>(),
});

function toParentLobbyMessage(message: LobbyViewMessage): Message {
  switch (message._tag) {
    case "CardArtTick":
    case "DeckCardFlipTick":
      return message;
    default:
      return GotLobbyMessage({ message });
  }
}

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
    summary: [],
    ...overrides,
  };
}

function playLobbyModel(overrides: Partial<Model>): Model {
  const [model] = init();
  return {
    ...model,
    route: PlayRoute({ deckId: "7" }),
    sessionLoaded: true,
    session: { me, meGravatarHash: null },
    ...overrides,
  };
}

function tableLobbyModel(overrides: Partial<Model>): Model {
  const [model] = init();
  return {
    ...model,
    route: PregameTableRoute({ deckId: "7", table: "ABC123" }),
    sessionLoaded: true,
    session: { me, meGravatarHash: null },
    ...overrides,
  };
}

const lobbyAppView = (model: Model) =>
  h.submodel({
    slotId: "lobby-test",
    model: model.lobby,
    view: lobbyView,
    viewInputs: {
      decks: model.decks.list.decks,
      decksLoading: model.decks.list.loading,
      knownCommanders: model.decks.list.knownCommanders,
      chrome: {
        version: model.apiVersion,
        faithfulCount: model.faithfulCount,
        oracleTotal: model.oracleTotal,
        coverageHref: "/coverage",
      },
      surface: model.route._tag === "PregameTableRoute" || model.route._tag === "GameTableRoute" ? "table" : "entry",
    },
    toParentMessage: toParentLobbyMessage,
  });

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

test("entry choose mode shows Host and Join destinations with deck on Host", () => {
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
    Scene.expect(Scene.testId("lobby-entry-choose")).toExist(),
    Scene.expect(Scene.testId("lobby-host")).toExist(),
    Scene.expect(Scene.testId("lobby-open-join")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card-9")).toExist(),
    Scene.expect(Scene.text("Tokens")).toExist(),
    Scene.expect(Scene.text("Rhys the Redeemed")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.testId("lobby-join-code")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-join")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 9 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("opening Join shows focused panel with Bringing strip and hides destinations", () => {
  const base = playLobbyModel({
    lobby: { ...initialLobbySlice(), selectedDeckId: 7 },
    decks: {
      ...init()[0].decks,
      list: {
        ...init()[0].decks.list,
        decks: [deck],
        knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa" }) },
        loading: false,
      },
    },
  });

  const [joined] = update(base, GotLobbyMessage({ message: RequestedLobbyOpenJoin() }));
  expect(joined.lobby.entryMode).toBe("join");

  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(joined),
    Scene.expect(Scene.testId("lobby-entry-join")).toExist(),
    Scene.expect(Scene.testId("lobby-bringing")).toExist(),
    Scene.expect(Scene.text("Superfriends")).toExist(),
    Scene.expect(Scene.testId("lobby-join-code")).toExist(),
    Scene.expect(Scene.testId("lobby-join")).toExist(),
    Scene.expect(Scene.testId("lobby-join-cancel")).toExist(),
    Scene.expect(Scene.testId("lobby-entry-choose")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-open-join")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("Cancel returns to choose and clears the table code", () => {
  const open = playLobbyModel({
    lobby: {
      ...initialLobbySlice(),
      selectedDeckId: 7,
      entryMode: "join",
      code: "ABC123",
      error: "UnknownTable",
    },
    decks: {
      ...init()[0].decks,
      list: { ...init()[0].decks.list, decks: [deck], loading: false },
    },
  });

  const [next] = update(open, GotLobbyMessage({ message: RequestedLobbyCancelJoin() }));
  expect(next.lobby.entryMode).toBe("choose");
  expect(next.lobby.code).toBe("");
  expect(next.lobby.error).toBeNull();

  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(next),
    Scene.expect(Scene.testId("lobby-entry-choose")).toExist(),
    Scene.expect(Scene.testId("lobby-entry-join")).toBeAbsent(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
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
    GotDeckListMessage({ message: DeckList.Message.ReceivedDecks({ decks: [deck] }) }),
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

test("PregameTableRoute cold load resets stale lobby entry state through the parent route entry", () => {
  const [base] = init(url("/play/9/XYZ789"));
  const [next, commands] = update(
    {
      ...base,
      lobby: {
        ...initialLobbySlice(),
        tableId: "OLD123",
        selectedDeckId: 7,
        code: "OLD123",
        entryMode: "join",
        started: true,
        error: "UnknownTable",
        copied: true,
        clipboardFallback: true,
        submitting: true,
      },
    },
    GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }),
  );

  expect(next.route).toEqual(PregameTableRoute({ deckId: "9", table: "XYZ789" }));
  expect(next.decks.list.loading).toBe(true);
  expect(next.lobby).toEqual({
    ...initialLobbySlice(),
    tableId: "XYZ789",
    selectedDeckId: 9,
  });
  expect(commands).toMatchObject([{ name: "FetchDecks" }, { name: "HashMeGravatar", args: { email: me.email } }]);
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
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
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
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
  );
});

test("unknown table explains that the link is stale", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      tableLobbyModel({
        lobby: {
          ...initialLobbySlice(),
          error: "UnknownTable",
          tableId: "GONE",
          selectedDeckId: 7,
        },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck], loading: false },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-error")).toExist(),
    Scene.expect(Scene.text("That table link is stale or expired. Ask the host for a new code.")).toExist(),
    Scene.expect(Scene.text("No such table.")).not.toExist(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
  );
});

test("watchers are told to stay on the table link for spectator view", () => {
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
                claimed: true,
                username: "alice",
                deck_name: "Superfriends",
                deck_id: 7,
                ready: true,
                is_host: true,
                is_you: false,
              },
            ],
          },
        },
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck], loading: false },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-watch-note")).toContainText(
      "Stay on this table link: if you don't claim a seat before the host starts, you'll enter spectator view.",
    ),
    Scene.expect(Scene.testId("lobby-claim")).toExist(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
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

  const [, commands] = update(withDeck, GotLobbyMessage({ message: LobbyTableCreated({ tableId: "XYZ789" }) }));
  const redirect = commands.find((c) => c.name === "Redirect") as { args?: { path?: string } } | undefined;
  expect(redirect?.args?.path).toBe("/play/7/XYZ789");
});

test("host handoff on PlayRoute keeps entry UI (no claim-seat flash)", () => {
  const withDeck = playLobbyModel({
    lobby: { ...initialLobbySlice(), selectedDeckId: 7 },
    decks: {
      ...init()[0].decks,
      list: { ...init()[0].decks.list, decks: [deck], loading: false },
    },
  });

  const [afterCreate] = update(withDeck, GotLobbyMessage({ message: LobbyTableCreated({ tableId: "XYZ789" }) }));
  expect(afterCreate.route._tag).toBe("PlayRoute");
  expect(afterCreate.lobby.tableId).toBe("XYZ789");

  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(afterCreate),
    Scene.expect(Scene.testId("lobby-entry-choose")).toExist(),
    Scene.expect(Scene.testId("lobby-host")).toExist(),
    Scene.expect(Scene.testId("lobby-claim")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-table-code")).toBeAbsent(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 7 }), DeckCardFlipTick()),
  );
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
    Scene.expect(Scene.selector('[data-testid="lobby-start-error"].text-caution-amber')).toExist(),
    Scene.expect(Scene.text("Need at least two players.")).toExist(),
  );
});

test("NotAllReady start gate uses waiting copy and caution amber", () => {
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
            start_error: "NotAllReady",
            seats: [
              {
                player: 0,
                claimed: true,
                username: "alice",
                deck_name: "Superfriends",
                deck_id: 7,
                ready: true,
                is_host: true,
                is_you: true,
              },
              {
                player: 1,
                claimed: true,
                username: "bob",
                deck_name: "Lorehold",
                deck_id: -2,
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
    Scene.expect(Scene.selector('[data-testid="lobby-start-error"].text-caution-amber')).toExist(),
    Scene.expect(Scene.text("Waiting for everyone to Ready…")).toExist(),
  );
});
