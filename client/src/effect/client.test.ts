// `orNull` is the one place a wire failure becomes a value. The lobby's actions answer every
// *logical* failure (TableFull, NotHost, …) with a 200 `LobbyView` carrying an `error` field, so
// the Effect only fails when the request never landed — a transport reject, a 5xx, a decode
// failure. Callers want that as `null` to branch on, not as a rejected promise to try/catch
// (ADR 0019: error folding lives in the pipeline, not the component).

import * as Effect from "effect/Effect";
import { beforeAll, describe, expect, it } from "vitest";
import { makeClient, orNull } from "~/effect/client";
import { json, networkError, recordingFetch, respondWith, status, stubLocation } from "~/effect/test-support";

beforeAll(stubLocation);

describe("makeClient", () => {
  it("sends credentials: include so session cookies work on the same-origin BFF", async () => {
    const { fetch, calls } = recordingFetch(json({ table_id: "ABCD" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.createTable({}));
    expect(calls).toHaveLength(1);
    expect(calls[0][1]?.credentials).toBe("include");
  });
  it("prepends the same-origin /api BFF prefix", async () => {
    const { fetch, calls } = recordingFetch(json({ table_id: "ABCD" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.createTable({}));
    expect(calls).toHaveLength(1);
    const url = calls[0][0];
    expect(url.pathname.startsWith("/api/")).toBe(true);
  });
});

describe("orNull", () => {
  it("passes a success value through untouched", async () => {
    expect(await Effect.runPromise(orNull(Effect.succeed({ table_id: "ABCD" })))).toEqual({ table_id: "ABCD" });
  });

  it("folds a typed failure to null", async () => {
    expect(await Effect.runPromise(orNull(Effect.fail(new Error("boom"))))).toBeNull();
  });

  it("folds an unreachable server (network error) to null", async () => {
    const client = makeClient(networkError);
    expect(await Effect.runPromise(orNull(client.createTable({})))).toBeNull();
  });

  it("folds a 500 to null", async () => {
    const client = makeClient(respondWith(status(500)));
    expect(await Effect.runPromise(orNull(client.createTable({})))).toBeNull();
  });

  it("still yields the view on a 200", async () => {
    const client = makeClient(respondWith(json({ table_id: "ABCD" })));
    expect(await Effect.runPromise(orNull(client.createTable({})))).toEqual({ table_id: "ABCD" });
  });

  // A defect is a bug, not a wire outcome. Folding it to `null` would let a crash masquerade as
  // "couldn't reach the table", which is exactly the diagnosis we'd want to keep.
  it("does not swallow defects", async () => {
    const boom = new Error("programmer error");
    await expect(Effect.runPromise(orNull(Effect.die(boom)))).rejects.toThrow();
  });
});
