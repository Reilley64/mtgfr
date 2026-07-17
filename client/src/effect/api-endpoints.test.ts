// Deck/catalog/game endpoints on `/api/rpc/**` (auth in api.test.ts).

import * as Result from "effect/Result";
import { beforeAll, describe, expect, it } from "vitest";
import { statusOf } from "~/effect/client";
import { bodyOf, json, recordingFetch, run, runEither, status, stubLocation } from "~/effect/test-support";
import type { DeckDetail, DeckError } from "~/wire/types";

beforeAll(stubLocation);

const req = {
  name: "Deck",
  commander: "some-legend-id",
  commander_print: "some-legend-print",
  cards: [{ id: "forest-id", count: 40, print: "forest-print" }],
};

describe("client.createDeck", () => {
  it("succeeds on a 200, returning the saved deck", async () => {
    const deck: DeckDetail = {
      id: 1,
      name: "Deck",
      commander: "some-legend-id",
      commander_print: "some-legend-print",
      cards: [],
    };
    const r = await run(recordingFetch(json(deck)).fetch, (c) => c.createDeck(req));
    expect(r).toEqual(deck);
  });

  it("fails with the CreateDeck422 tagged error carrying the DeckError on a 422", async () => {
    const err: DeckError = { problems: ["Commander must be a legendary creature."] };
    const r = await runEither(recordingFetch(json(err, 422)).fetch, (c) => c.createDeck(req));
    expect(Result.isFailure(r) && (r.failure as { _tag: string })._tag).toBe("CreateDeck422");
    expect(Result.isFailure(r) && (r.failure as { cause: DeckError }).cause).toEqual(err);
  });

  it("fails with an HttpClientError (status 500) on a 500", async () => {
    const r = await runEither(recordingFetch(status(500)).fetch, (c) => c.createDeck(req));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(500);
  });

  it("POSTs to /api/rpc/decks", async () => {
    const { fetch, calls } = recordingFetch(json({ id: 1, ...req }));
    await run(fetch, (c) => c.createDeck(req));
    expect(calls[0][0].pathname).toBe("/api/rpc/decks");
    expect(bodyOf(calls[0][1])).toEqual(req);
  });
});

describe("client.listDecks", () => {
  it("decodes the deck summary list", async () => {
    const decks = [{ id: 1, name: "Deck", commander: "Some Legend" }];
    const r = await run(recordingFetch(json(decks)).fetch, (c) => c.listDecks());
    expect(r).toEqual(decks);
  });
});

describe("client.submitIntent", () => {
  it("POSTs the given envelope to the table's game path and returns the ack", async () => {
    const envelope = {
      table_id: "t1",
      client_seq: 1,
      intent: { kind: "pass_priority", player: 0 },
    } as const;
    const { fetch, calls } = recordingFetch(json({ accepted: true }));
    const r = await run(fetch, (c) => c.submitIntent("t1", envelope));
    expect(r).toEqual({ accepted: true });
    const [url, init] = calls[0];
    expect(url.pathname).toBe("/api/rpc/game/t1/intent");
    expect(bodyOf(init)).toEqual(envelope);
  });

  // Board's session-expired banner branches on this status (see rejectMessageFor).
  it("surfaces an expired-session 401 as an HttpClientError with status 401", async () => {
    const envelope = {
      table_id: "t1",
      client_seq: 1,
      intent: { kind: "pass_priority", player: 0 },
    } as const;
    const r = await runEither(recordingFetch(status(401)).fetch, (c) => c.submitIntent("t1", envelope));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(401);
  });
});

describe("client.searchCards", () => {
  it("GETs /api/rpc/cards/search with the query and paging", async () => {
    const { fetch, calls } = recordingFetch(json([]));
    await run(fetch, (c) => c.searchCards({ q: "goblin red", limit: 50, offset: 10 }));
    const [url] = calls[0];
    expect(url.pathname).toBe("/api/rpc/cards/search");
    expect(url.searchParams.get("q")).toBe("goblin red");
    expect(url.searchParams.get("limit")).toBe("50");
    expect(url.searchParams.get("offset")).toBe("10");
  });
});

describe("client.lookupCards", () => {
  it("GETs /api/rpc/cards/lookup with one repeated ids param per card", async () => {
    const { fetch, calls } = recordingFetch(json([]));
    await run(fetch, (c) => c.lookupCards(["breena-id", "forest-id"]));
    const [url] = calls[0];
    expect(url.pathname).toBe("/api/rpc/cards/lookup");
    expect(url.searchParams.getAll("ids")).toEqual(["breena-id", "forest-id"]);
  });
});
