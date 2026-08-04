import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { init } from "./init";
import {
  GotAuthMessage,
  GotCoverageMessage,
  GotLeaderboardMessage,
  NavigationCompleted,
  ReceivedLeaderboardPage,
  ReceivedMeGravatarHash,
  UrlChanged,
} from "./messages";
import {
  CoverageRoute,
  DeckRoute,
  GameTableRoute,
  HomeRoute,
  LeaderboardRoute,
  normalizeAppRoute,
  PlayRoute,
  PregameTableRoute,
  pathWithSearch,
  routeFromUrl,
  routePath,
} from "./routes";
import * as Auth from "./shell/auth";
import {
  ChangedCoverageQuery,
  CoverageLoadFailed,
  ReceivedCoverageMeta,
  RequestedCoverageRefresh,
} from "./shell/coverage/messages";
import { FetchCoverage } from "./shell/coverage/update";
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

test("parses the Foldkit shell routes", () => {
  expect(routeFromUrl(url("/"))).toEqual(HomeRoute());
  expect(routeFromUrl(url("/leaderboard"))).toEqual(LeaderboardRoute());
  expect(routeFromUrl(url("/coverage"))).toEqual(CoverageRoute());
  expect(routeFromUrl(url("/decks/abc"))).toEqual(DeckRoute({ id: "abc" }));
});

test("parses play routes with required deckId", () => {
  expect(routeFromUrl(url("/play/7"))).toEqual(PlayRoute({ deckId: "7" }));
  expect(routeFromUrl(url("/play/-1/ABC123"))).toEqual(PregameTableRoute({ deckId: "-1", table: "ABC123" }));
});

test("numeric /play/:deckId is PlayRoute", () => {
  const raw = routeFromUrl(url("/play/7"));

  expect(normalizeAppRoute(raw, "/play/7")).toEqual(PlayRoute({ deckId: "7" }));
});

test("hex /play/:table is GameTableRoute", () => {
  const raw = routeFromUrl(url("/play/ABC123DEF"));

  expect(normalizeAppRoute(raw, "/play/ABC123DEF")).toEqual(GameTableRoute({ table: "ABC123DEF" }));
});

test("mixed six-character table code normalizes to GameTableRoute", () => {
  const raw = routeFromUrl(url("/play/ABC123"));

  expect(normalizeAppRoute(raw, "/play/ABC123")).toEqual(GameTableRoute({ table: "ABC123" }));
});

test("all-digit six-character play segment still normalizes to PlayRoute", () => {
  const raw = routeFromUrl(url("/play/234567"));

  expect(normalizeAppRoute(raw, "/play/234567")).toEqual(PlayRoute({ deckId: "234567" }));
});

test("pregame /play/:deckId/:table stays two-segment", () => {
  const raw = routeFromUrl(url("/play/7/ABC123"));

  expect(normalizeAppRoute(raw, "/play/7/ABC123")).toEqual(PregameTableRoute({ deckId: "7", table: "ABC123" }));
});

test("bare /play is not found", () => {
  expect(routeFromUrl(url("/play"))._tag).toBe("NotFoundRoute");
});

test("builds typed route paths", () => {
  expect(routePath(CoverageRoute())).toBe("/coverage");
  expect(routePath(DeckRoute({ id: "abc" }))).toBe("/decks/abc");
  expect(routePath(LeaderboardRoute())).toBe("/leaderboard");
  expect(routePath(PlayRoute({ deckId: "7" }))).toBe("/play/7");
  expect(routePath(PregameTableRoute({ deckId: "7", table: "ABC123" }))).toBe("/play/7/ABC123");
  expect(routePath(GameTableRoute({ table: "ABC123" }))).toBe("/play/ABC123");
});

test("pathWithSearch inserts ? for Foldkit search without a leading ?", () => {
  expect(pathWithSearch(url("/play", "deck=-1"))).toBe("/play?deck=-1");
});

test("pathWithSearch returns pathname only when search is empty", () => {
  expect(pathWithSearch(url("/play"))).toBe("/play");
  expect(pathWithSearch(url("/play", ""))).toBe("/play");
});

test("non-numeric single-segment play path becomes GameTableRoute after normalize", () => {
  const raw = routeFromUrl(url("/play/table-1"));
  expect(raw).toEqual(PlayRoute({ deckId: "table-1" }));

  const [base] = init(url("/play/table-1"));

  expect(base.route).toEqual(GameTableRoute({ table: "table-1" }));
});

test("PlayRoute /play/-1 sets lobby.selectedDeckId to -1", () => {
  const [base] = init(url("/play/-1"));

  const [model] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));

  expect(model.route).toEqual(PlayRoute({ deckId: "-1" }));
  expect(model.lobby.selectedDeckId).toBe(-1);
});

test("navigating from PlayRoute to GameTableRoute clears lobby.selectedDeckId and seeds an active game slice", () => {
  const [base] = init(url("/play/7"));
  const [authed] = update(base, GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) }));

  const [model] = update(authed, UrlChanged({ url: url("/play/ABC123") }));

  expect(authed.route).toEqual(PlayRoute({ deckId: "7" }));
  expect(authed.lobby.selectedDeckId).toBe(7);
  expect(model.route).toEqual(GameTableRoute({ table: "ABC123" }));
  expect(model.game).not.toBeNull();
  expect(model.game?.tableId).toBe("ABC123");
  expect(model.game?.active).toBe(true);
  expect(model.lobby.tableId).toBe("ABC123");
  expect(model.lobby.selectedDeckId).toBeNull();
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
    Story.given(model),
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

test("CoverageRoute loads set coverage on protected route entry", () => {
  const [model] = init(url("/coverage"));
  const load = FetchCoverage();
  const page = ReceivedCoverageMeta({
    faithfulCount: 662,
    oracleTotal: 28412,
    sets: [
      {
        code: "soc",
        name: "Secrets of Strixhaven",
        releasedAt: "2026-04-01",
        faithful: 10,
        oracleTotal: 400,
      },
    ],
  });

  Story.story(
    update,
    Story.given(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) })),
    Story.Command.expectExact(load, HashMeGravatar({ email: me.email })),
    Story.Command.resolve(load, page),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.model((m) => {
      expect(m.coverage.status).toBe("ready");
      expect(m.coverage.faithfulCount).toBe(662);
      expect(m.coverage.oracleTotal).toBe(28412);
      expect(m.coverage.sets).toEqual([
        {
          code: "soc",
          name: "Secrets of Strixhaven",
          releasedAt: "2026-04-01",
          faithful: 10,
          oracleTotal: 400,
        },
      ]);
    }),
  );
});

test("HomeRoute loads decks on protected route entry", () => {
  const [model] = init(url("/"));
  const decks = [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }];

  Story.story(
    update,
    Story.given(model),
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

test("UrlChanged to HomeRoute clears transient deck list UI before loading decks", () => {
  const [base] = init(url("/leaderboard"));

  Story.story(
    update,
    Story.given({
      ...base,
      currentPath: "/leaderboard",
      route: LeaderboardRoute(),
      session: { me, meGravatarHash: null },
      sessionLoaded: true,
      decks: {
        ...base.decks,
        list: {
          ...base.decks.list,
          confirmingDeleteId: 7,
          contextMenu: { deckId: 7, x: 10, y: 20 },
          error: "Could not load decks.",
        },
      },
    }),
    Story.message(UrlChanged({ url: url("/") })),
    Story.Command.expectExact(DeckList.FetchDecks),
    Story.model((m) => {
      expect(m.route).toEqual(HomeRoute());
      expect(m.decks.list.loading).toBe(true);
      expect(m.decks.list.confirmingDeleteId).toBeNull();
      expect(m.decks.list.contextMenu).toBeNull();
      expect(m.decks.list.error).toBeNull();
    }),
    Story.Command.resolve(DeckList.FetchDecks, DeckList.Message.ReceivedDecks({ decks: [] })),
    Story.Command.resolve(
      DeckList.LookupDeckListCommanders({ ids: [] }),
      DeckList.Message.ReceivedDeckListCommanders({ cards: [] }),
    ),
  );
});

test("HomeRoute cold load clears transient deck list UI through the same route entry path", () => {
  const [base] = init(url("/"));

  Story.story(
    update,
    Story.given({
      ...base,
      decks: {
        ...base.decks,
        list: {
          ...base.decks.list,
          confirmingDeleteId: 7,
          contextMenu: { deckId: 7, x: 10, y: 20 },
          error: "Could not load decks.",
        },
      },
    }),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me }) })),
    Story.Command.expectExact(DeckList.FetchDecks, HashMeGravatar({ email: me.email })),
    Story.model((m) => {
      expect(m.route).toEqual(HomeRoute());
      expect(m.decks.list.loading).toBe(true);
      expect(m.decks.list.confirmingDeleteId).toBeNull();
      expect(m.decks.list.contextMenu).toBeNull();
      expect(m.decks.list.error).toBeNull();
    }),
    Story.Command.resolve(DeckList.FetchDecks, DeckList.Message.ReceivedDecks({ decks: [] })),
    Story.Command.resolve(HashMeGravatar, ReceivedMeGravatarHash({ email: me.email, hash: "deadbeef" })),
    Story.Command.resolve(
      DeckList.LookupDeckListCommanders({ ids: [] }),
      DeckList.Message.ReceivedDeckListCommanders({ cards: [] }),
    ),
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
    Story.given(model),
    Story.message(GotLeaderboardMessage({ message: RequestedLeaderboardRefresh() })),
    Story.Command.expectExact(load),
    Story.model((m) => {
      expect(m.leaderboard.entries).toEqual([]);
      expect(m.leaderboard.error).toBeNull();
      expect(m.leaderboard.status).toBe("loading");
    }),
    Story.Command.resolve(load, page),
  );
});

test("coverage retry clears rows and re-enters loading", () => {
  const [base] = init(url("/coverage"));
  const load = FetchCoverage();
  const model = {
    ...base,
    coverage: {
      ...base.coverage,
      status: "error",
      query: "soc",
      sets: [
        {
          code: "soc",
          name: "Secrets of Strixhaven",
          releasedAt: "2026-04-01",
          faithful: 10,
          oracleTotal: 400,
        },
      ],
      faithfulCount: 662,
      oracleTotal: 28412,
      error: "Could not load coverage.",
    },
  };

  Story.story(
    update,
    Story.given(model),
    Story.message(GotCoverageMessage({ message: RequestedCoverageRefresh() })),
    Story.Command.expectExact(load),
    Story.model((m) => {
      expect(m.coverage.status).toBe("loading");
      expect(m.coverage.error).toBeNull();
      expect(m.coverage.sets).toEqual([]);
      expect(m.coverage.query).toBe("soc");
    }),
    Story.Command.resolve(load, CoverageLoadFailed({ message: "Could not load coverage." })),
    Story.model((m) => {
      expect(m.coverage.status).toBe("error");
      expect(m.coverage.error).toBe("Could not load coverage.");
      expect(m.coverage.sets).toEqual([]);
    }),
  );
});

test("coverage query updates in place", () => {
  const [base] = init(url("/coverage"));

  Story.story(
    update,
    Story.given(base),
    Story.message(GotCoverageMessage({ message: ChangedCoverageQuery({ query: "strix" }) })),
    Story.model((m) => {
      expect(m.coverage.query).toBe("strix");
    }),
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
    Story.given(model),
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
    Story.given(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me: null }) })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});

test("redirects unsigned protected coverage route", () => {
  const [model] = init(url("/coverage"));
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2Fcoverage" },
    effect: Effect.succeed(NavigationCompleted()),
  };

  Story.story(
    update,
    Story.given(model),
    Story.message(GotAuthMessage({ message: Auth.Message.ReceivedMe({ me: null }) })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});
