import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { init } from "./init";
import {
  ClosedAccountMenu,
  GotAuthMessage,
  GotDeckListMessage,
  GotLeaderboardMessage,
  NavigationCompleted,
  ReceivedLeaderboardPage,
  ReceivedMeGravatarHash,
  ToggledAccountMenu,
} from "./messages";
import type { Model } from "./model";
import {
  DeckRoute,
  HomeRoute,
  LeaderboardRoute,
  PlayRoute,
  pathWithSearch,
  routeFromUrl,
  routePath,
  TableRoute,
} from "./routes";
import * as Auth from "./shell/auth";
import * as DeckList from "./shell/decks/list";
import { RequestedLeaderboardRefresh } from "./shell/leaderboard/messages";
import { FetchLeaderboard } from "./shell/leaderboard/update";
import { HashMeGravatar, update } from "./update";

/** Foldkit `Url.search` is without a leading `?` (e.g. `deck=-1`). */
const url = (pathname: string, search = "") => ({
  protocol: "http:",
  host: "localhost",
  port: Option.none<string>(),
  pathname,
  search: search === "" ? Option.none<string>() : Option.some(search),
  hash: Option.none<string>(),
});

const me = { id: 1, email: "alice@example.com", username: "alice" };

function homeModel(overrides: Partial<Model> = {}): Model {
  const [base] = init(url("/"));
  return {
    ...base,
    route: HomeRoute(),
    session: { me, meGravatarHash: null },
    sessionLoaded: true,
    ...overrides,
  };
}

test("parses the Foldkit shell routes", () => {
  expect(routeFromUrl(url("/"))).toEqual(HomeRoute());
  expect(routeFromUrl(url("/leaderboard"))).toEqual(LeaderboardRoute());
  expect(routeFromUrl(url("/decks/abc"))).toEqual(DeckRoute({ id: "abc" }));
});

test("parses play routes with required deckId", () => {
  expect(routeFromUrl(url("/play/7"))).toEqual(PlayRoute({ deckId: "7" }));
  expect(routeFromUrl(url("/play/-1/ABC123"))).toEqual(TableRoute({ deckId: "-1", table: "ABC123" }));
});

test("bare /play is not found", () => {
  expect(routeFromUrl(url("/play"))._tag).toBe("NotFoundRoute");
});

test("builds typed route paths", () => {
  expect(routePath(DeckRoute({ id: "abc" }))).toBe("/decks/abc");
  expect(routePath(LeaderboardRoute())).toBe("/leaderboard");
  expect(routePath(PlayRoute({ deckId: "7" }))).toBe("/play/7");
  expect(routePath(TableRoute({ deckId: "7", table: "ABC123" }))).toBe("/play/7/ABC123");
});

test("pathWithSearch inserts ? for Foldkit search without a leading ?", () => {
  expect(pathWithSearch(url("/play", "deck=-1"))).toBe("/play?deck=-1");
});

test("pathWithSearch returns pathname only when search is empty", () => {
  expect(pathWithSearch(url("/play"))).toBe("/play");
  expect(pathWithSearch(url("/play", ""))).toBe("/play");
});

test("non-integer play deckId becomes NotFound after normalize", () => {
  const raw = routeFromUrl(url("/play/table-1"));
  expect(raw).toEqual(PlayRoute({ deckId: "table-1" }));

  const [base] = init(url("/play/table-1"));

  expect(base.route._tag).toBe("NotFoundRoute");
});

test("PlayRoute /play/-1 sets lobby.selectedDeckId to -1", () => {
  const [base] = init(url("/play/-1"));

  const [model] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));

  expect(model.route).toEqual(PlayRoute({ deckId: "-1" }));
  expect(model.lobby.selectedDeckId).toBe(-1);
});

test("LeaderboardRoute loads the first page on protected route entry", () => {
  const [model] = init(url("/leaderboard"));
  const load = FetchLeaderboard({ limit: 50, offset: 0 });
  const page = ReceivedLeaderboardPage({
    leaderboard: { entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }], total: 1 },
    offset: 0,
  });

  Story.story(
    update,
    Story.with(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) })),
    Story.Command.expectExact(load, HashMeGravatar({ email: me.email })),
    Story.Command.resolve(load, page),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.model((m) => {
      expect(m.leaderboard.status).toBe("ready");
      expect(m.leaderboard.entries).toEqual([{ rank: 1, rating: 1200, user_id: 1, username: "alice" }]);
    }),
  );
});

test("HomeRoute loads decks on protected route entry", () => {
  const [model] = init(url("/"));
  const decks = [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }];

  Story.story(
    update,
    Story.with(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) })),
    Story.Command.expectExact(DeckList.FetchDecks, HashMeGravatar({ email: me.email })),
    Story.Command.resolve(DeckList.FetchDecks, DeckList.Message.ReceivedDecks({ decks })),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.Command.resolve(
      DeckList.LookupDeckListCommanders({ ids: ["atraxa"] }),
      DeckList.Message.ReceivedDeckListCommanders({ cards: [] }),
    ),
    Story.model((m) => {
      expect(m.decks.list.decks).toEqual(decks);
      expect("leaderboardTeaser" in m.decks.list).toBe(false);
    }),
  );
});

test("HomeRoute toggles the account menu open and clears the deck context menu", () => {
  const model = homeModel();

  Story.story(
    update,
    Story.with({
      ...model,
      decks: {
        ...model.decks,
        list: {
          ...model.decks.list,
          accountMenuOpen: false,
          contextMenu: { deckId: 7, x: 10, y: 20 },
        },
      },
    }),
    Story.message(ToggledAccountMenu()),
    Story.model((m) => {
      expect(m.decks.list.accountMenuOpen).toBe(true);
      expect(m.decks.list.contextMenu).toBeNull();
    }),
  );
});

test("HomeRoute closes the account menu when requested", () => {
  const model = homeModel();

  Story.story(
    update,
    Story.with({
      ...model,
      decks: {
        ...model.decks,
        list: {
          ...model.decks.list,
          accountMenuOpen: true,
        },
      },
    }),
    Story.message(ClosedAccountMenu()),
    Story.model((m) => {
      expect(m.decks.list.accountMenuOpen).toBe(false);
    }),
  );
});

test("HomeRoute opening a deck context menu closes the account menu", () => {
  const model = homeModel();

  Story.story(
    update,
    Story.with({
      ...model,
      decks: {
        ...model.decks,
        list: {
          ...model.decks.list,
          accountMenuOpen: true,
        },
      },
    }),
    Story.message(
      GotDeckListMessage({
        message: DeckList.Message.OpenedDeckListMenu({ deckId: 7, x: 10, y: 20 }),
      }),
    ),
    Story.model((m) => {
      expect(m.decks.list.accountMenuOpen).toBe(false);
      expect(m.decks.list.contextMenu).toEqual({ deckId: 7, x: 10, y: 20 });
    }),
  );
});

test("leaderboard retry refreshes from the first page after an error", () => {
  const [base] = init(url("/leaderboard"));
  const load = FetchLeaderboard({ limit: 50, offset: 0 });
  const page = ReceivedLeaderboardPage({
    leaderboard: { entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }], total: 1 },
    offset: 0,
  });
  const model = {
    ...base,
    leaderboard: {
      ...base.leaderboard,
      accountMenuOpen: true,
      entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }],
      error: "Could not load the leaderboard.",
      status: "error",
      total: 2,
    },
  };

  Story.story(
    update,
    Story.with(model),
    Story.message(GotLeaderboardMessage({ message: RequestedLeaderboardRefresh() })),
    Story.Command.expectExact(load),
    Story.model((m) => {
      expect(m.leaderboard.accountMenuOpen).toBe(false);
      expect(m.leaderboard.entries).toEqual([]);
      expect(m.leaderboard.error).toBeNull();
      expect(m.leaderboard.status).toBe("loading");
    }),
    Story.Command.resolve(load, page),
  );
});

test("redirects unsigned protected play routes with path deck", () => {
  const [model] = init(url("/play/7"));
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2Fplay%2F7" },
    effect: Effect.succeed(NavigationCompleted()),
  };

  Story.story(
    update,
    Story.with(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me: null }) })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});

test("redirects unsigned protected leaderboard route", () => {
  const [model] = init(url("/leaderboard"));
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2Fleaderboard" },
    effect: Effect.succeed(NavigationCompleted()),
  };

  Story.story(
    update,
    Story.with(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me: null }) })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});
