import { Effect } from "effect";
import { expect, test } from "vitest";
import { pushUrlMaybeViewTransition, shouldAnimateDeckCardNav } from "./view-transition";

function settledViewTransition(): ViewTransition {
  return {
    finished: Promise.resolve(),
    ready: Promise.resolve(),
    types: new Set<string>(),
    updateCallbackDone: Promise.resolve(),
    skipTransition() {},
  };
}

function fakeStartViewTransition(onStart: () => void): typeof document.startViewTransition {
  return (callbackOptions) => {
    onStart();
    const callback = typeof callbackOptions === "function" ? callbackOptions : callbackOptions?.update;
    void callback?.();
    return settledViewTransition();
  };
}

test("shouldAnimateDeckCardNav only for home to play deck entry", () => {
  expect(shouldAnimateDeckCardNav("/", "/play/7")).toBe(true);
  expect(shouldAnimateDeckCardNav("/play/7", "/")).toBe(true);
  expect(shouldAnimateDeckCardNav("/", "/play/7/ABC")).toBe(false);
  expect(shouldAnimateDeckCardNav("/decks/1", "/play/7")).toBe(false);
  expect(shouldAnimateDeckCardNav("/play/7", "/play/8")).toBe(false);
});

test("pushUrlMaybeViewTransition uses startViewTransition when animating", async () => {
  let started = false;
  let pushed = false;

  await Effect.runPromise(
    pushUrlMaybeViewTransition("/play/7", "/", {
      prefersReducedMotion: false,
      startViewTransition: fakeStartViewTransition(() => {
        started = true;
      }),
      pushUrl: () =>
        Effect.sync(() => {
          pushed = true;
        }),
    }),
  );

  expect(started).toBe(true);
  expect(pushed).toBe(true);
});

test("pushUrlMaybeViewTransition skips view transitions when reduced motion is preferred", async () => {
  let started = false;

  await Effect.runPromise(
    pushUrlMaybeViewTransition("/play/7", "/", {
      prefersReducedMotion: true,
      startViewTransition: fakeStartViewTransition(() => {
        started = true;
      }),
      pushUrl: () => Effect.void,
    }),
  );

  expect(started).toBe(false);
});
