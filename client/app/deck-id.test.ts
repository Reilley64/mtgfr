import { expect, test } from "vitest";
import { deckCardViewTransitionName, parseDeckIdParam, playDeckAccess } from "./deck-id";

test("parseDeckIdParam accepts integers including negative precons", () => {
  expect(parseDeckIdParam("7")).toBe(7);
  expect(parseDeckIdParam("-1")).toBe(-1);
  expect(parseDeckIdParam("0")).toBe(0);
});

test("parseDeckIdParam rejects non-integers", () => {
  expect(parseDeckIdParam("")).toBeNull();
  expect(parseDeckIdParam("abc")).toBeNull();
  expect(parseDeckIdParam("1.5")).toBeNull();
  expect(parseDeckIdParam("01")).toBe(1);
});

test("deckCardViewTransitionName is keyed by id", () => {
  expect(deckCardViewTransitionName(7)).toBe("deck-card-7");
  expect(deckCardViewTransitionName(-1)).toBe("deck-card--1");
});

test("playDeckAccess reflects loading, known, and missing decks", () => {
  expect(playDeckAccess(7, [], true)).toBe("loading");
  expect(playDeckAccess(7, [{ id: 7 }], false)).toBe("ok");
  expect(playDeckAccess(7, [{ id: 1 }], false)).toBe("missing");
  expect(playDeckAccess(null, [{ id: 1 }], false)).toBe("missing");
});

test("playDeckAccess preserves a deck load error instead of treating an empty list as missing", () => {
  expect(playDeckAccess(7, [], false, "Could not load decks.")).toBe("error");
});
