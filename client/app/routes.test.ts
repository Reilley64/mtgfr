import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { init } from "./init";
import {
  NavigationCompleted,
  ReceivedDeckListCommanders,
  ReceivedDeckListLeaderboardTeaser,
  ReceivedDecks,
  ReceivedLeaderboardPage,
  ReceivedMe,
  ReceivedMeGravatarHash,
  RequestedLeaderboardRefresh,
} from "./messages";
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
import { FetchDeckListLeaderboardTeaser, FetchDecks, LookupDeckListCommanders } from "./shell/decks/list/update";
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

  const [model] = update(base, ReceivedMe({ me }));

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
    Story.message(ReceivedMe({ me })),
    Story.Command.expectExact(load, HashMeGravatar({ email: me.email })),
    Story.Command.resolve(load, page),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.model((m) => {
      expect(m.leaderboard.status).toBe("ready");
      expect(m.leaderboard.entries).toEqual([{ rank: 1, rating: 1200, user_id: 1, username: "alice" }]);
    }),
  );
});

test("HomeRoute loads decks and the leaderboard teaser on protected route entry", () => {
  const [model] = init(url("/"));
  const loadTeaser = FetchDeckListLeaderboardTeaser({ limit: 5, offset: 0 });
  const decks = [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }];
  const teaser = [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }];

  Story.story(
    update,
    Story.with(model),
    Story.message(ReceivedMe({ me })),
    Story.Command.expectExact(FetchDecks, loadTeaser, HashMeGravatar({ email: me.email })),
    Story.Command.resolve(FetchDecks, ReceivedDecks({ decks })),
    Story.Command.resolve(loadTeaser, ReceivedDeckListLeaderboardTeaser({ entries: teaser })),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.Command.resolve(LookupDeckListCommanders({ ids: ["atraxa"] }), ReceivedDeckListCommanders({ cards: [] })),
    Story.model((m) => {
      expect(m.decks.list.decks).toEqual(decks);
      expect(m.decks.list.leaderboardTeaser).toEqual(teaser);
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
      entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }],
      error: "Could not load the leaderboard.",
      status: "error",
      total: 2,
    },
  };

  Story.story(
    update,
    Story.with(model),
    Story.message(RequestedLeaderboardRefresh()),
    Story.Command.expectExact(load),
    Story.model((m) => {
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
    Story.message(ReceivedMe({ me: null })),
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
    Story.message(ReceivedMe({ me: null })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});
