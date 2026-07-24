import { expect, test } from "vitest";
import { deckCardViewTransitionName, parseDeckIdParam } from "./deck-id";

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
