// The reconnect/backoff behaviour of `streamDeltas`, driven with a `TestClock` so the virtual
// timing (backoff sleeps) is exact, and a mock `fetch` whose body is a `ReadableStream` the test
// pushes SSE `data:` frames into / closes to simulate deltas and server drops.
//
// The tricky bit: byte delivery through the mock `ReadableStream` is *real* async (undici), while
// the backoff timers are *virtual* (TestClock). `settle` yields the real event loop so a pushed
// chunk propagates; `TestClock.adjust` advances the virtual timers. Interleave the two.
//
// Frame parsing + SSE decoding now live in the generated client, so this file no longer covers
// per-frame parse resilience or the old byte-level stale watchdog (both dropped with NDJSON —
// see stream.ts).

import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as TestClock from "effect/testing/TestClock";
import { beforeAll, describe, expect, it } from "vitest";
import { makeClient } from "~/effect/client";
import { type StreamCallbacks, streamDeltas } from "~/effect/stream";
import { stubLocation } from "~/effect/test-support";

beforeAll(stubLocation);

/** A mock `fetch` recording every connection. Each attempt gets a status from `statusFor(n)`; a
 * 200 hands back a `ReadableStream` body the test drives via `push`/`close` by connection index. */
function harness(statusFor: (attempt: number) => number = () => 200) {
  const enc = new TextEncoder();
  const conns: { url: string; controller?: ReadableStreamDefaultController<Uint8Array> }[] = [];
  const fetchImpl = ((input: URL | RequestInfo) => {
    const code = statusFor(conns.length);
    if (code !== 200) {
      conns.push({ url: String(input) });
      return Promise.resolve(new Response(null, { status: code }));
    }
    let controller!: ReadableStreamDefaultController<Uint8Array>;
    const body = new ReadableStream<Uint8Array>({ start: (c) => (controller = c) });
    conns.push({ url: String(input), controller });
    return Promise.resolve(new Response(body, { status: 200 }));
  }) as typeof fetch;
  const controller = (i: number) => {
    const conn = conns[i];
    if (!conn) throw new Error(`connection ${i} does not exist yet (${conns.length} open)`);
    const c = conn.controller;
    if (!c) throw new Error(`connection ${i} has no stream controller`);
    return c;
  };
  return {
    fetchImpl,
    conns,
    /** Push one SSE `data:` frame (JSON value) on connection `i`. */
    frame: (i: number, value: unknown) => controller(i).enqueue(enc.encode(`data: ${JSON.stringify(value)}\n`)),
    /** Push raw bytes (for keepalive/blank-line cases). */
    raw: (i: number, text: string) => controller(i).enqueue(enc.encode(text)),
    close: (i: number) => controller(i).close(),
  };
}

function build(h: ReturnType<typeof harness>, cb: StreamCallbacks, random: () => number = () => 1) {
  return streamDeltas("t", cb, random, makeClient(h.fetchImpl));
}

/** Yield the *real* event loop so a pushed `ReadableStream` chunk (or a fresh fetch) propagates. */
const settle = Effect.promise(() => new Promise((r) => setTimeout(r, 5)));
const adjust = (ms: number) => TestClock.adjust(Duration.millis(ms));

/** Wait until mock fetch has opened connection `i` (CI event-loop timing varies). */
const waitConn = (h: ReturnType<typeof harness>, i: number) =>
  Effect.promise(async () => {
    for (let n = 0; n < 400; n++) {
      if (h.conns[i]) return;
      await new Promise((r) => setTimeout(r, 5));
    }
    throw new Error(`timed out waiting for connection ${i} (have ${h.conns.length})`);
  });

/** Run a driver against a `TestClock`. Forked children are interrupted when the driver returns. */
function drive(body: Effect.Effect<void, never, TestClock.TestClock>): Promise<void> {
  return Effect.runPromise(body.pipe(Effect.provide(TestClock.layer())));
}

describe("streamDeltas frame delivery", () => {
  it("delivers SSE data frames in order and skips blank keepalive lines", async () => {
    const h = harness();
    const frames: unknown[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame: (f) => frames.push(f) }));
        yield* waitConn(h, 0);
        h.frame(0, 1);
        h.raw(0, "\n"); // a blank keepalive line between frames
        h.frame(0, 2);
        yield* settle;
        yield* Fiber.interrupt(fiber);
      }),
    );
    expect(frames).toEqual([1, 2]);
  });
});

describe("streamDeltas reconnection", () => {
  it("reconnects with exponential backoff, doubling per failure and capping at 10s", async () => {
    // Every attempt 5xxs, so the backoff never resets — this isolates the doubling/cap curve.
    // With random()=1 the full-jitter factor is 1.0, so each wait equals the current backoff.
    const h = harness(() => 503);
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame() {}, onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1);

        yield* adjust(499);
        yield* settle;
        expect(h.conns.length).toBe(1);
        yield* adjust(1);
        yield* settle;
        expect(h.conns.length).toBe(2); // reconnect at 500ms

        yield* adjust(1000);
        yield* settle;
        expect(h.conns.length).toBe(3); // wait doubled to 1000ms

        yield* adjust(2000);
        yield* settle;
        yield* adjust(4000);
        yield* settle;
        yield* adjust(8000);
        yield* settle;
        expect(h.conns.length).toBe(6);

        yield* adjust(10_000);
        yield* settle; // capped at 10s (would be 16s uncapped)
        expect(h.conns.length).toBe(7);
        yield* adjust(10_000);
        yield* settle;
        expect(h.conns.length).toBe(8);

        yield* Fiber.interrupt(fiber);
      }),
    );
    expect(status.every((c) => c === false)).toBe(true); // never healthy, only drop notifications
  });

  it("resets the backoff after a healthy connection (first frame received)", async () => {
    // Attempt 0 fails (backoff climbs to 1000ms); attempt 1 connects and delivers a frame (healthy)
    // then drops. If the reset works the post-drop wait is back to 500ms, not 1000ms.
    const h = harness((n) => (n === 0 ? 503 : 200));
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame() {}, onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1); // attempt 0 (503) → 500ms wait, backoff now 1000

        yield* adjust(500);
        yield* settle;
        yield* waitConn(h, 1);
        expect(h.conns.length).toBe(2); // attempt 1 connects
        h.frame(1, 1); // a frame proves health → onStatus(true), backoff reset
        yield* settle;
        expect(status.at(-1)).toBe(true);

        h.close(1); // server drops the healthy connection
        yield* settle;
        expect(status.at(-1)).toBe(false);

        yield* adjust(499);
        yield* settle;
        expect(h.conns.length).toBe(2); // still waiting
        yield* adjust(1);
        yield* settle;
        expect(h.conns.length).toBe(3); // reconnect at exactly 500ms → backoff was reset

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("reports a 4xx once via onError and stops retrying forever", async () => {
    const h = harness(() => 404);
    const errors: number[] = [];
    await drive(
      Effect.gen(function* () {
        yield* Effect.forkChild(build(h, { onFrame() {}, onError: (s) => errors.push(s) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1);
        expect(errors).toEqual([404]);

        yield* adjust(600_000); // ten minutes
        yield* settle;
        expect(h.conns.length).toBe(1); // no further attempts
        expect(errors).toEqual([404]); // reported exactly once
      }),
    );
  });

  it("treats a silent established connection as dead after the stale timeout and reconnects", async () => {
    // Connection 0 goes healthy (a snapshot proves it), then falls silent — the upstream died with
    // no FIN, so no error surfaces. After STALE_TIMEOUT_MS with no frame the watchdog must drop it:
    // onStatus(false), then a backoff reconnect (connection 1). Heartbeats, had they arrived, would
    // have kept refreshing the timer — their absence is the whole signal.
    const h = harness();
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame() {}, onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1);
        h.frame(0, { frame: "snapshot", seq: 0, state: {} }); // proves health
        yield* settle;
        expect(status.at(-1)).toBe(true);

        yield* adjust(14_999); // just under the 15s stale window — still considered alive
        yield* settle;
        expect(h.conns.length).toBe(1);
        expect(status.at(-1)).toBe(true);

        yield* adjust(1); // 15s of silence → dead
        yield* settle;
        expect(status.at(-1)).toBe(false); // dropped
        yield* adjust(500); // the (reset) backoff wait
        yield* settle;
        expect(h.conns.length).toBe(2); // reconnected

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("heartbeat frames refresh the stale timer but never reach onFrame", async () => {
    const h = harness();
    const frames: unknown[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame: (f) => frames.push(f) }));
        yield* waitConn(h, 0);
        h.frame(0, { frame: "snapshot", seq: 0, state: {} });
        yield* settle;

        // A heartbeat every 10s keeps the 15s watchdog from ever tripping across 40s of otherwise
        // quiet time, and none of them are delivered to the store.
        for (let t = 0; t < 4; t++) {
          yield* adjust(10_000);
          h.frame(0, { frame: "heartbeat" });
          yield* settle;
        }
        expect(h.conns.length).toBe(1); // never dropped — heartbeats kept it alive
        expect(frames).toEqual([{ frame: "snapshot", seq: 0, state: {} }]); // heartbeats filtered out

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("a heartbeat alone proves a reconnect is healthy", async () => {
    // Reconnecting onto a quiet game (nobody has taken an action, so no delta and no snapshot yet)
    // leaves the heartbeat as the only frame on the wire. It must still clear the reconnect banner,
    // or the player stares at "Reconnecting…" over a perfectly live table.
    const h = harness();
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame() {}, onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        h.frame(0, { frame: "heartbeat" });
        yield* settle;
        expect(status).toEqual([true]);

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("stops on interrupt: no further frames, no further reconnects", async () => {
    const h = harness();
    const frames: unknown[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(build(h, { onFrame: (f) => frames.push(f) }));
        yield* waitConn(h, 0);
        h.frame(0, 1);
        yield* settle;
        expect(frames).toEqual([1]);

        yield* Fiber.interrupt(fiber);
        yield* adjust(60_000);
        yield* settle;
        expect(h.conns.length).toBe(1); // no reconnect after stop
        expect(frames).toEqual([1]); // no more deliveries
      }),
    );
  });
});
