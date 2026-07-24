import { Effect, Option } from "effect";
import { Story } from "foldkit";
import { expect, test } from "vitest";
import { init } from "./init";
import { NavigationCompleted, ReceivedMe } from "./messages";
import { DeckRoute, HomeRoute, PlayRoute, pathWithSearch, routeFromUrl, routePath, TableRoute } from "./routes";
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

  const [model] = update(base, ReceivedMe({ me: { id: 1, email: "alice@example.com", username: "alice" } }));

  expect(model.route).toEqual(PlayRoute({ deckId: "-1" }));
  expect(model.lobby.selectedDeckId).toBe(-1);
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
