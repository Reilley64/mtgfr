// Deck/catalog/lobby/game endpoints on the generated wire client: request shape (path, method,
// body/query) and the error model (typed `MtgfrError` for a declared 422 body; `HttpClientError`
// with a status otherwise). Auth is covered in api.test.ts.

import * as Result from "effect/Result";
import { beforeAll, describe, expect, it } from "vitest";
import type { DeckDetail, DeckError } from "~/api/generated";
import { statusOf } from "~/effect/client";
import { bodyOf, json, recordingFetch, run, runEither, status, stubLocation } from "~/effect/test-support";

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
    const r = await run(recordingFetch(json(deck)).fetch, (c) => c.createDeck({ payload: req }));
    expect(r).toEqual(deck);
  });

  it("fails with the CreateDeck422 tagged error carrying the DeckError on a 422", async () => {
    const err: DeckError = { problems: ["Commander must be a legendary creature."] };
    const r = await runEither(recordingFetch(json(err, 422)).fetch, (c) => c.createDeck({ payload: req }));
    expect(Result.isFailure(r) && (r.failure as { _tag: string })._tag).toBe("CreateDeck422");
    expect(Result.isFailure(r) && (r.failure as { cause: DeckError }).cause).toEqual(err);
  });

  it("fails with an HttpClientError (status 500) on a 500", async () => {
    const r = await runEither(recordingFetch(status(500)).fetch, (c) => c.createDeck({ payload: req }));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(500);
  });
});

describe("client.listDecks", () => {
  it("decodes the deck summary list", async () => {
    const decks = [{ id: 1, name: "Deck", commander: "Some Legend" }];
    const r = await run(recordingFetch(json(decks)).fetch, (c) => c.listDecks({}));
    expect(r).toEqual(decks);
  });
});

describe("client.joinTable", () => {
  it("POSTs the table id and deck id as the request body", async () => {
    const { fetch, calls } = recordingFetch(json({ table_id: "t1", started: false, seats: [] }));
    await run(fetch, (c) => c.joinTable({ payload: { table_id: "t1", deck_id: 7 } }));
    expect(calls).toHaveLength(1);
    const [url, init] = calls[0];
    expect(url.pathname).toBe("/api/tables/join/v1");
    expect(init?.method).toBe("POST");
    expect(bodyOf(init)).toEqual({ table_id: "t1", deck_id: 7 });
  });
});

describe("client.submitIntent", () => {
  it("POSTs the given envelope verbatim and returns the ack", async () => {
    const envelope = {
      table_id: "t1",
      client_seq: 1,
      intent: { kind: "pass_priority", player: 0 },
    } as const;
    const { fetch, calls } = recordingFetch(json({ accepted: true }));
    const r = await run(fetch, (c) => c.submitIntent({ payload: envelope }));
    expect(r).toEqual({ accepted: true });
    const [url, init] = calls[0];
    expect(url.pathname).toBe("/api/intent/v1");
    expect(bodyOf(init)).toEqual(envelope);
  });

  // Board's session-expired banner branches on this status (see rejectMessageFor).
  it("surfaces an expired-session 401 as an HttpClientError with status 401", async () => {
    const envelope = {
      table_id: "t1",
      client_seq: 1,
      intent: { kind: "pass_priority", player: 0 },
    } as const;
    const r = await runEither(recordingFetch(status(401)).fetch, (c) => c.submitIntent({ payload: envelope }));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(401);
  });
});

describe("client.searchCards", () => {
  it("GETs /cards/search with the query and paging", async () => {
    const { fetch, calls } = recordingFetch(json([]));
    await run(fetch, (c) => c.searchCards({ params: { q: "goblin red", limit: 50, offset: 10 } }));
    const [url] = calls[0];
    expect(url.pathname).toBe("/api/cards/search/v1");
    expect(url.searchParams.get("q")).toBe("goblin red");
    expect(url.searchParams.get("limit")).toBe("50");
    expect(url.searchParams.get("offset")).toBe("10");
  });
});

describe("client.lookupCards", () => {
  it("GETs /cards/lookup with one repeated ids param per card", async () => {
    const { fetch, calls } = recordingFetch(json([]));
    await run(fetch, (c) => c.lookupCards({ params: { ids: ["breena-id", "forest-id"] } }));
    const [url] = calls[0];
    expect(url.pathname).toBe("/api/cards/lookup/v1");
    expect(url.searchParams.getAll("ids")).toEqual(["breena-id", "forest-id"]);
  });
});
