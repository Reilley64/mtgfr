// `orNull` is the one place a wire failure becomes a value.

import * as Data from "effect/Data";
import * as Effect from "effect/Effect";
import { beforeAll, describe, expect, it } from "vitest";
import { makeClient, orNull } from "~/effect/client";
import { json, networkError, recordingFetch, respondWith, status, stubLocation } from "~/effect/test-support";

class Boom extends Data.TaggedError("Boom")<{}> {}

beforeAll(stubLocation);

describe("makeClient", () => {
  it("sends credentials: include so session cookies work on the same-origin BFF", async () => {
    const { fetch, calls } = recordingFetch(json({ id: 1, email: "a@b.co", username: "alice" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.me());
    expect(calls).toHaveLength(1);
    expect(calls[0][1]?.credentials).toBe("include");
  });
  it("prepends the same-origin /api/rpc BFF prefix", async () => {
    const { fetch, calls } = recordingFetch(json({ id: 1, email: "a@b.co", username: "alice" }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.me());
    expect(calls).toHaveLength(1);
    const url = calls[0][0];
    expect(url.pathname).toBe("/api/rpc/auth/me");
  });
});

describe("orNull", () => {
  it("passes a success value through untouched", async () => {
    expect(await Effect.runPromise(orNull(Effect.succeed({ id: 1 })))).toEqual({ id: 1 });
  });

  it("folds a typed failure to null", async () => {
    expect(await Effect.runPromise(orNull(Effect.fail(new Boom())))).toBeNull();
  });

  it("folds an unreachable server (network error) to null", async () => {
    const client = makeClient(networkError);
    expect(await Effect.runPromise(orNull(client.me()))).toBeNull();
  });

  it("folds a 500 to null", async () => {
    const client = makeClient(respondWith(status(500)));
    expect(await Effect.runPromise(orNull(client.me()))).toBeNull();
  });

  it("still yields the value on a 200", async () => {
    const client = makeClient(respondWith(json({ id: 1, email: "a@b.co", username: "alice" })));
    expect(await Effect.runPromise(orNull(client.me()))).toEqual({ id: 1, email: "a@b.co", username: "alice" });
  });

  it("does not swallow defects", async () => {
    const boom = new Error("programmer error");
    await expect(Effect.runPromise(orNull(Effect.die(boom)))).rejects.toThrow();
  });
});
