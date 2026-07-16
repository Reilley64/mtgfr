// `orNull` is the one place a wire failure becomes a value.

import * as Effect from "effect/Effect";
import { beforeAll, describe, expect, it } from "vitest";
import { makeClient, orNull } from "~/effect/client";
import { json, networkError, recordingFetch, respondWith, status, stubLocation } from "~/effect/test-support";

beforeAll(stubLocation);

const seedBody = {
  table_id: "ABCD",
  host_user_id: 1,
  seats: [
    { user_id: 1, username: "a", deck_id: 1 },
    { user_id: 2, username: "b", deck_id: 2 },
  ],
};

describe("makeClient", () => {
  it("sends credentials: include so session cookies work on the same-origin BFF", async () => {
    const { fetch, calls } = recordingFetch(json({ table_id: "ABCD", pod_dns: "x", version: "v" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.seedTable({ payload: seedBody }));
    expect(calls).toHaveLength(1);
    expect(calls[0][1]?.credentials).toBe("include");
  });
  it("prepends the same-origin /api BFF prefix", async () => {
    const { fetch, calls } = recordingFetch(json({ table_id: "ABCD", pod_dns: "x", version: "v" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.seedTable({ payload: seedBody }));
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
    expect(await Effect.runPromise(orNull(client.seedTable({ payload: seedBody })))).toBeNull();
  });

  it("folds a 500 to null", async () => {
    const client = makeClient(respondWith(status(500)));
    expect(await Effect.runPromise(orNull(client.seedTable({ payload: seedBody })))).toBeNull();
  });

  it("still yields the view on a 200", async () => {
    const client = makeClient(respondWith(json({ table_id: "ABCD", pod_dns: "x", version: "v" })));
    expect(await Effect.runPromise(orNull(client.seedTable({ payload: seedBody })))).toEqual({
      table_id: "ABCD",
      pod_dns: "x",
      version: "v",
    });
  });

  it("does not swallow defects", async () => {
    const boom = new Error("programmer error");
    await expect(Effect.runPromise(orNull(Effect.die(boom)))).rejects.toThrow();
  });
});
