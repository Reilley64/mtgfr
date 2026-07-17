import * as Effect from "effect/Effect";
import { describe, expect, it } from "vitest";
import { decksEffectForMe } from "~/atoms";
import type { DeckSummary, Me } from "~/wire/types";

const me: Me = { id: 1, email: "a@b.c", username: "a" };
const decks: DeckSummary[] = [{ id: 1, name: "X", commander: "c" }];

describe("decksEffectForMe", () => {
  it("does not run listDecks when not signed in", async () => {
    let called = false;
    const listDecks = Effect.sync(() => {
      called = true;
      return decks;
    });
    const out = await Effect.runPromise(decksEffectForMe(null, listDecks));
    expect(called).toBe(false);
    expect(out).toEqual([]);
  });

  it("runs listDecks when signed in", async () => {
    let called = false;
    const listDecks = Effect.sync(() => {
      called = true;
      return decks;
    });
    const out = await Effect.runPromise(decksEffectForMe(me, listDecks));
    expect(called).toBe(true);
    expect(out).toEqual(decks);
  });
});
