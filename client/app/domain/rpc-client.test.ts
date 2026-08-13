// `orNull` is the one place a wire failure becomes a value.

import * as Data from "effect/Data";
import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { makeClient, orNull } from "./rpc-client";

class Boom extends Data.TaggedError("Boom")<{ readonly reason: string }> {}

function stubLocation(): void {
  vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });
}

function respondWith(response: Response): typeof fetch {
  // Bun's `typeof fetch` includes `preconnect`; stubs only implement the call signature.
  return (() => Promise.resolve(response)) as unknown as typeof fetch;
}

const status = (code: number) => new Response(null, { status: code });
const json = (body: unknown, code = 200) =>
  new Response(JSON.stringify(body), { status: code, headers: { "content-type": "application/json" } });
const networkError: typeof fetch = (() => Promise.reject(new TypeError("Failed to fetch"))) as unknown as typeof fetch;

function recordingFetch(response: Response): { fetch: typeof fetch; calls: [URL, RequestInit | undefined][] } {
  const calls: [URL, RequestInit | undefined][] = [];
  const fetchImpl = ((url: URL, init?: RequestInit) => {
    calls.push([url, init]);
    return Promise.resolve(response);
  }) as unknown as typeof fetch;
  return { fetch: fetchImpl, calls };
}

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

  it("revives only streamed stack entry ids from lossless decimal strings", async () => {
    const first = "9007199254740992";
    const second = "9007199254740993";
    const response = new Response(
      `data: ${JSON.stringify({
        frame: "snapshot",
        seq: 1,
        state: {
          stack: [{ entry_id: first, source: 0 }, { entry_id: second }],
          lookalike: { entry_id: second },
        },
        lookalike: { entry_id: second },
      })}\n\n`,
      { status: 200, headers: { "content-type": "text/event-stream" } },
    );
    const client = makeClient(respondWith(response));

    const frames = Array.from(await Effect.runPromise(Stream.runCollect(client.streamSse("TABLE"))));
    const frame = frames[0] as unknown as {
      state: { stack: Array<{ entry_id: bigint; source?: number }>; lookalike: { entry_id: string } };
      lookalike: { entry_id: string };
    };

    expect(frame.state.stack.map((entry) => entry.entry_id)).toEqual([BigInt(first), BigInt(second)]);
    expect(frame.state.stack[0]?.entry_id).not.toBe(frame.state.stack[1]?.entry_id);
    expect(frame.state.stack[0]?.source).toBe(0);
    expect(frame.state.lookalike.entry_id).toBe(second);
    expect(frame.lookalike.entry_id).toBe(second);
  });

  it("accepts zero and the canonical uint64 maximum for streamed stack entry ids", async () => {
    const maximum = "18446744073709551615";
    const response = new Response(
      `data: ${JSON.stringify({
        frame: "snapshot",
        seq: 1,
        state: { stack: [{ entry_id: "0" }, { entry_id: maximum }] },
      })}\n\n`,
      { status: 200, headers: { "content-type": "text/event-stream" } },
    );
    const client = makeClient(respondWith(response));

    const frames = Array.from(await Effect.runPromise(Stream.runCollect(client.streamSse("TABLE"))));
    const stack = frames[0]?.frame === "snapshot" ? frames[0].state.stack : [];

    expect(stack.map((entry) => entry.entry_id)).toEqual([0n, BigInt(maximum)]);
  });

  it.each([
    ["above uint64 maximum", "18446744073709551616"],
    ["negative", "-1"],
    ["leading zero", "01"],
    ["empty", ""],
    ["very long", "9".repeat(10_000)],
  ])("rejects a %s streamed stack entry id through the typed error channel", async (_case, entryId) => {
    const response = new Response(
      `data: ${JSON.stringify({ frame: "snapshot", seq: 1, state: { stack: [{ entry_id: entryId }] } })}\n\n`,
      { status: 200, headers: { "content-type": "text/event-stream" } },
    );
    const client = makeClient(respondWith(response));

    const error = await Effect.runPromise(Effect.flip(Stream.runCollect(client.streamSse("TABLE"))));

    expect(error).toMatchObject({ _tag: "StreamFrameParseError", message: "Invalid SSE stream frame" });
    expect(String(error)).toBe("StreamFrameParseError: Invalid SSE stream frame");
  });

  it.each([
    ["an unknown frame variant", { frame: "unknown" }],
    ["a snapshot without a stack", { frame: "snapshot", seq: 1, state: {} }],
    ["a delta without events", { frame: "delta", seq: 1, state: { stack: [] } }],
  ])("rejects %s through the typed error channel", async (_case, value) => {
    const response = new Response(`data: ${JSON.stringify(value)}\n\n`, {
      status: 200,
      headers: { "content-type": "text/event-stream" },
    });
    const client = makeClient(respondWith(response));

    const error = await Effect.runPromise(Effect.flip(Stream.runCollect(client.streamSse("TABLE"))));

    expect(error).toMatchObject({ _tag: "StreamFrameParseError", message: "Invalid SSE stream frame" });
  });

  it("rejects malformed SSE JSON through the typed error channel without echoing the payload", async () => {
    const payload = '{"frame":"snapshot","secret":"DO_NOT_ECHO"';
    const response = new Response(`data: ${payload}\n\n`, {
      status: 200,
      headers: { "content-type": "text/event-stream" },
    });
    const client = makeClient(respondWith(response));

    const error = await Effect.runPromise(Effect.flip(Stream.runCollect(client.streamSse("TABLE"))));

    expect(error).toMatchObject({ _tag: "StreamFrameParseError", message: "Invalid SSE stream frame" });
    expect(String(error)).not.toContain("DO_NOT_ECHO");
  });

  it("posts an empty JSON object on logout — the BFF rejects bodiless POSTs as BadJson", async () => {
    const { fetch, calls } = recordingFetch(new Response(null, { status: 204 }));
    const client = makeClient(fetch);
    await Effect.runPromise(client.logout());
    expect(calls).toHaveLength(1);
    expect(calls[0][0].pathname).toBe("/api/rpc/auth/logout");
    const body = calls[0][1]?.body;
    const text = body instanceof Uint8Array ? new TextDecoder().decode(body) : body;
    expect(text).toBe("{}");
  });

  it("builds ratings/leaderboard with limit and offset query params", async () => {
    const { fetch, calls } = recordingFetch(
      json({
        entries: [{ user_id: 7, username: "alice", rating: 1234, rank: 26 }],
        total: 99,
      }),
    );
    const client = makeClient(fetch);
    const leaderboard = await Effect.runPromise(client.ratings.leaderboard({ limit: 25, offset: 25 }));
    expect(leaderboard).toEqual({
      entries: [{ user_id: 7, username: "alice", rating: 1234, rank: 26 }],
      total: 99,
    });
    expect(calls).toHaveLength(1);
    const url = calls[0][0];
    expect(url.pathname).toBe("/api/rpc/ratings/leaderboard");
    expect(url.searchParams.get("limit")).toBe("25");
    expect(url.searchParams.get("offset")).toBe("25");
  });
});

describe("orNull", () => {
  it("passes a success value through untouched", async () => {
    expect(await Effect.runPromise(orNull(Effect.succeed({ id: 1 })))).toEqual({ id: 1 });
  });

  it("folds a typed failure to null", async () => {
    expect(await Effect.runPromise(orNull(Effect.fail(new Boom({ reason: "boom" }))))).toBeNull();
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
