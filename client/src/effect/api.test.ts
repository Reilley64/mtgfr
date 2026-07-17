// Auth calls on `/api/rpc/auth/*`. Assert failures via `statusOf`.

import * as Result from "effect/Result";
import { beforeAll, describe, expect, it } from "vitest";
import { statusOf } from "~/effect/client";
import { json, networkError, run, runEither, status, stubLocation } from "~/effect/test-support";

beforeAll(stubLocation);

const creds = { email: "a@b.co", password: "pw" };

describe("client.login", () => {
  it("succeeds on a 2xx", async () => {
    const r = await runEither(respondJson({ id: 1, email: "a@b.co", username: "alice" }), (c) => c.login(creds));
    expect(Result.isSuccess(r)).toBe(true);
  });

  it("surfaces a 401 as an HttpClientError with the status", async () => {
    const r = await runEither(respondStatus(401), (c) => c.login({ ...creds, password: "bad" }));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(401);
  });

  it("surfaces a 500", async () => {
    const r = await runEither(respondStatus(500), (c) => c.login(creds));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(500);
  });

  it("surfaces a network error with no status", async () => {
    const r = await runEither(networkError, (c) => c.login(creds));
    expect(Result.isFailure(r)).toBe(true);
    expect(Result.isFailure(r) && statusOf(r.failure)).toBeUndefined();
  });
});

describe("client.signup", () => {
  it("surfaces a duplicate-email 409 as an HttpClientError", async () => {
    const r = await runEither(respondStatus(409), (c) => c.signup({ ...creds, email: "taken@b.co", username: "taken" }));
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(409);
  });
});

describe("client.me", () => {
  it("returns the user on 200", async () => {
    const r = await run(respondJson({ id: 1, email: "a@b.co", username: "alice" }), (c) => c.me());
    expect(r).toEqual({ id: 1, email: "a@b.co", username: "alice" });
  });

  it("surfaces a 401 as a failure (the guard treats any failure as not-signed-in)", async () => {
    const r = await runEither(respondStatus(401), (c) => c.me());
    expect(Result.isFailure(r) && statusOf(r.failure)).toBe(401);
  });
});

/** A stub `fetch` answering with the given status and no body. */
function respondStatus(code: number): typeof fetch {
  return () => Promise.resolve(status(code));
}
/** A stub `fetch` answering 200 with the given JSON body. */
function respondJson(body: unknown): typeof fetch {
  return () => Promise.resolve(json(body));
}
