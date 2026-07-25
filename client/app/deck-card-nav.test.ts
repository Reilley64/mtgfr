/**
 * @vitest-environment happy-dom
 */
import { afterEach, expect, test } from "vitest";
import {
  captureDeckCardFlipForNav,
  resetDeckCardFlipForTests,
  runDeckCardFlip,
  shouldAnimateDeckCardNav,
  takePendingDeckCardFlip,
} from "./deck-card-nav";

afterEach(() => {
  resetDeckCardFlipForTests();
  document.body.replaceChildren();
});

test("shouldAnimateDeckCardNav only for home ↔ play deck entry", () => {
  expect(shouldAnimateDeckCardNav("/", "/play/7")).toBe(true);
  expect(shouldAnimateDeckCardNav("/play/7", "/")).toBe(true);
  expect(shouldAnimateDeckCardNav("/", "/play/7/ABC")).toBe(false);
  expect(shouldAnimateDeckCardNav("/decks/1", "/play/7")).toBe(false);
});

test("captureDeckCardFlipForNav snapshots the flip chrome before play nav", () => {
  const chrome = document.createElement("div");
  chrome.dataset.deckCardFlip = "7";
  Object.defineProperty(chrome, "getBoundingClientRect", {
    value: () => ({ top: 10, left: 20, width: 200, height: 100, right: 220, bottom: 110, x: 20, y: 10, toJSON() {} }),
  });
  document.body.append(chrome);

  captureDeckCardFlipForNav("/", "/play/7");
  expect(takePendingDeckCardFlip(7)).toEqual({
    deckId: 7,
    top: 10,
    left: 20,
    width: 200,
    height: 100,
  });
  expect(takePendingDeckCardFlip(7)).toBeNull();
});

test("runDeckCardFlip animates from the captured rect", () => {
  const el = document.createElement("div");
  Object.defineProperty(el, "getBoundingClientRect", {
    value: () => ({ top: 40, left: 80, width: 100, height: 50, right: 180, bottom: 90, x: 80, y: 40, toJSON() {} }),
  });
  const calls: unknown[] = [];
  const animate = function (this: HTMLElement, keyframes: unknown, opts: unknown) {
    calls.push({ keyframes, opts });
    return {} as Animation;
  };

  runDeckCardFlip(
    el,
    { deckId: 7, top: 10, left: 20, width: 200, height: 100 },
    animate as typeof Element.prototype.animate,
  );

  expect(calls).toHaveLength(1);
  const first = calls[0] as { keyframes: Array<{ transform: string }>; opts: { duration: number } };
  expect(first.keyframes[0]?.transform).toContain("translate(-60px, -30px)");
  expect(first.keyframes[0]?.transform).toContain("scale(2, 2)");
  expect(first.opts.duration).toBe(280);
});
