import { Match as M, Option, pipe, Schema as S } from "effect";
import { literal, mapTo, oneOf, parseUrlWithFallback, r, root, slash, string } from "foldkit/route";
import type { Url } from "foldkit/url";
import { parseDeckIdParam } from "./deck-id";

export const HomeRoute = r("HomeRoute");
export const LoginRoute = r("LoginRoute");
export const LeaderboardRoute = r("LeaderboardRoute");
export const CoverageRoute = r("CoverageRoute");
export const NewDeckRoute = r("NewDeckRoute");
export const DeckRoute = r("DeckRoute", { id: S.String });
export const PlayRoute = r("PlayRoute", { deckId: S.String });
export const PregameTableRoute = r("PregameTableRoute", { deckId: S.String, table: S.String });
export const GameTableRoute = r("GameTableRoute", { table: S.String });
export const NotFoundRoute = r("NotFoundRoute", { path: S.String });

export const AppRoute = S.Union([
  HomeRoute,
  LoginRoute,
  LeaderboardRoute,
  CoverageRoute,
  NewDeckRoute,
  DeckRoute,
  PlayRoute,
  PregameTableRoute,
  GameTableRoute,
  NotFoundRoute,
]);
export type AppRoute = typeof AppRoute.Type;

const homeRouter = pipe(root, mapTo(HomeRoute));
const loginRouter = pipe(literal("login"), mapTo(LoginRoute));
const leaderboardRouter = pipe(literal("leaderboard"), mapTo(LeaderboardRoute));
const coverageRouter = pipe(literal("coverage"), mapTo(CoverageRoute));
const newDeckRouter = pipe(literal("decks"), slash(literal("new")), mapTo(NewDeckRoute));
const deckRouter = pipe(literal("decks"), slash(string("id")), mapTo(DeckRoute));
const playRouter = pipe(literal("play"), slash(string("deckId")), mapTo(PlayRoute));
const pregameTableRouter = pipe(
  literal("play"),
  slash(string("deckId")),
  slash(string("table")),
  mapTo(PregameTableRoute),
);

const appRouter = oneOf(
  homeRouter,
  loginRouter,
  leaderboardRouter,
  coverageRouter,
  newDeckRouter,
  deckRouter,
  pregameTableRouter,
  playRouter,
);

export const routeFromUrl = parseUrlWithFallback(appRouter, NotFoundRoute);

export function normalizeAppRoute(route: AppRoute, path: string): AppRoute {
  if (route._tag === "PlayRoute") {
    const deckId = route.deckId;
    if (parseDeckIdParam(deckId) != null) return PlayRoute({ deckId });
    if (deckId.trim() === "") return NotFoundRoute({ path });
    return GameTableRoute({ table: deckId });
  }

  if (route._tag === "GameTableRoute") {
    const table = route.table;
    if (parseDeckIdParam(table) != null) return PlayRoute({ deckId: table });
    if (table.trim() === "") return NotFoundRoute({ path });
    return GameTableRoute({ table });
  }

  if (route._tag !== "PregameTableRoute") return route;
  if (parseDeckIdParam(route.deckId) != null) return route;
  return NotFoundRoute({ path });
}

export function pathWithSearch(url: Url): string {
  const search = Option.getOrUndefined(url.search);
  if (search == null || search === "") return url.pathname;
  const q = search.startsWith("?") ? search : `?${search}`;
  return `${url.pathname}${q}`;
}

export function isProtectedRoute(route: AppRoute): boolean {
  return M.value(route).pipe(
    M.when({ _tag: "LoginRoute" }, () => false),
    M.when({ _tag: "NotFoundRoute" }, () => false),
    M.orElse(() => true),
  );
}

export function routePath(route: AppRoute): string {
  return M.value(route).pipe(
    M.withReturnType<string>(),
    M.tagsExhaustive({
      HomeRoute: () => homeRouter(),
      LoginRoute: () => loginRouter(),
      LeaderboardRoute: () => leaderboardRouter(),
      CoverageRoute: () => coverageRouter(),
      NewDeckRoute: () => newDeckRouter(),
      DeckRoute: ({ id }) => deckRouter({ id }),
      PlayRoute: ({ deckId }) => playRouter({ deckId }),
      PregameTableRoute: ({ deckId, table }) => pregameTableRouter({ deckId, table }),
      GameTableRoute: ({ table }) => playRouter({ deckId: table }),
      NotFoundRoute: ({ path }) => path,
    }),
  );
}

export function nextFromUrl(url: Url): string {
  const search = Option.getOrUndefined(url.search);
  return safeNext(new URLSearchParams(String(search ?? "")).get("next") ?? undefined);
}

export function safeNext(next: string | null | undefined): string {
  if (!next?.startsWith("/") || next.startsWith("//") || next.startsWith("/\\")) return "/";
  if (/^[a-z][a-z\d+.-]*:/i.test(next)) return "/";
  return next;
}
