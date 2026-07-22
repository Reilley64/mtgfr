import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { init } from "./init";
import { NavigationCompleted, ReceivedMe } from "./messages";
import {
  DeckRoute,
  HomeRoute,
  PlayRoute,
  pathWithSearch,
  routeFromUrl,
  routePath,
  TableRoute,
} from "./routes";
import { update } from "./update";

/** Foldkit `Url.search` is without a leading `?` (e.g. `deck=-1`). */
const url = (pathname: string, search = "") => ({
  protocol: "http:",
  host: "localhost",
  port: Option.none<string>(),
  pathname,
  search: search === "" ? Option.none<string>() : Option.some(search),
  hash: Option.none<string>(),
});

test("parses the Foldkit shell routes", () => {
  expect(routeFromUrl(url("/"))).toEqual(HomeRoute());
  expect(routeFromUrl(url("/decks/abc"))).toEqual(DeckRoute({ id: "abc" }));
  expect(routeFromUrl(url("/play"))).toEqual(PlayRoute());
  expect(routeFromUrl(url("/play/table-1"))).toEqual(TableRoute({ table: "table-1" }));
});

test("builds typed route paths", () => {
  expect(routePath(DeckRoute({ id: "abc" }))).toBe("/decks/abc");
  expect(routePath(TableRoute({ table: "table-1" }))).toBe("/play/table-1");
});

test("pathWithSearch inserts ? for Foldkit search without a leading ?", () => {
  expect(pathWithSearch(url("/play", "deck=-1"))).toBe("/play?deck=-1");
});

test("pathWithSearch returns pathname only when search is empty", () => {
  expect(pathWithSearch(url("/play"))).toBe("/play");
  expect(pathWithSearch(url("/play", ""))).toBe("/play");
});

test("PlayRoute entry with currentPath /play?deck=-1 sets lobby.selectedDeckId to -1", () => {
  const [base] = init(url("/play", "deck=-1"));
  expect(base.currentPath).toBe("/play?deck=-1");

  const [model] = update(
    base,
    ReceivedMe({ me: { id: 1, email: "alice@example.com", username: "alice" } }),
  );

  expect(model.route).toEqual(PlayRoute());
  expect(model.lobby.selectedDeckId).toBe(-1);
});

test("redirects unsigned protected routes with query string preserved", () => {
  const [model] = init(url("/play", "deck=abc"));
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2Fplay%3Fdeck%3Dabc" },
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
