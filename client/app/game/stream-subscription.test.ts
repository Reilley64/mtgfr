// Stream reconnect coverage for the Foldkit game subscription module.

import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Stream from "effect/Stream";
import * as TestClock from "effect/testing/TestClock";
import { beforeAll, describe, expect, it } from "vitest";
import { makeClient } from "~/effect/client";
import { stubLocation } from "~/effect/test-support";
import { StreamTerminalError } from "./messages";
import { type GameStreamEvent, streamDeltas, streamMessages } from "./stream-subscription";

beforeAll(stubLocation);

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
    frame: (i: number, value: unknown) => controller(i).enqueue(enc.encode(`data: ${JSON.stringify(value)}\n`)),
    raw: (i: number, text: string) => controller(i).enqueue(enc.encode(text)),
    close: (i: number) => controller(i).close(),
  };
}

type StreamHandlers = {
  onFrame?: (frame: GameStreamEvent & { kind: "frame" }) => void;
  onStatus?: (connected: boolean) => void;
  onError?: (status: number) => void;
};

function build(h: ReturnType<typeof harness>, random: () => number = () => 1) {
  return streamDeltas("t", random, makeClient(h.fetchImpl));
}

function observe(h: ReturnType<typeof harness>, handlers: StreamHandlers, random: () => number = () => 1) {
  return build(h, random).pipe(
    Stream.runForEach((event) =>
      Effect.sync(() => {
        switch (event.kind) {
          case "frame":
            handlers.onFrame?.(event);
            return;
          case "status":
            handlers.onStatus?.(event.connected);
            return;
          case "terminal-error":
            handlers.onError?.(event.status);
            return;
          default: {
            const _exhaustive: never = event;
            return _exhaustive;
          }
        }
      }),
    ),
  );
}

const settle = Effect.promise(() => new Promise((r) => setTimeout(r, 5)));
const adjust = (ms: number) => TestClock.adjust(Duration.millis(ms));

const waitConn = (h: ReturnType<typeof harness>, i: number) =>
  Effect.promise(async () => {
    for (let n = 0; n < 400; n++) {
      if (h.conns[i]) return;
      await new Promise((r) => setTimeout(r, 5));
    }
    throw new Error(`timed out waiting for connection ${i} (have ${h.conns.length})`);
  });

function drive(body: Effect.Effect<void, never, TestClock.TestClock>): Promise<void> {
  return Effect.runPromise(body.pipe(Effect.provide(TestClock.layer())));
}

describe("game stream subscription streamDeltas", () => {
  it("delivers SSE data frames in order and skips blank keepalive lines", async () => {
    const h = harness();
    const frames: unknown[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onFrame: (event) => frames.push(event.frame) }));
        yield* waitConn(h, 0);
        h.frame(0, 1);
        h.raw(0, "\n");
        h.frame(0, 2);
        yield* settle;
        yield* Fiber.interrupt(fiber);
      }),
    );
    expect(frames).toEqual([1, 2]);
  });

  it("reconnects with exponential backoff, doubling per failure and capping at 10s", async () => {
    const h = harness(() => 503);
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1);

        yield* adjust(499);
        yield* settle;
        expect(h.conns.length).toBe(1);
        yield* adjust(1);
        yield* settle;
        expect(h.conns.length).toBe(2);

        yield* adjust(1000);
        yield* settle;
        expect(h.conns.length).toBe(3);

        yield* adjust(2000);
        yield* settle;
        yield* adjust(4000);
        yield* settle;
        yield* adjust(8000);
        yield* settle;
        expect(h.conns.length).toBe(6);

        yield* adjust(10_000);
        yield* settle;
        expect(h.conns.length).toBe(7);
        yield* adjust(10_000);
        yield* settle;
        expect(h.conns.length).toBe(8);

        yield* Fiber.interrupt(fiber);
      }),
    );
    expect(status.every((c) => c === false)).toBe(true);
  });

  it("resets the backoff after a healthy connection", async () => {
    const h = harness((n) => (n === 0 ? 503 : 200));
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);

        yield* adjust(500);
        yield* settle;
        yield* waitConn(h, 1);
        h.frame(1, 1);
        yield* settle;
        expect(status.at(-1)).toBe(true);

        h.close(1);
        yield* settle;
        expect(status.at(-1)).toBe(false);

        yield* adjust(499);
        yield* settle;
        expect(h.conns.length).toBe(2);
        yield* adjust(1);
        yield* settle;
        expect(h.conns.length).toBe(3);

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("reports a 4xx once via onError and stops retrying forever", async () => {
    const h = harness(() => 404);
    const errors: number[] = [];
    await drive(
      Effect.gen(function* () {
        yield* Effect.forkChild(observe(h, { onError: (s) => errors.push(s) }));
        yield* waitConn(h, 0);
        expect(h.conns.length).toBe(1);
        expect(errors).toEqual([404]);

        yield* adjust(600_000);
        yield* settle;
        expect(h.conns.length).toBe(1);
        expect(errors).toEqual([404]);
      }),
    );
  });

  it("emits terminal statuses so the board can explain 401 and 404 failures", async () => {
    for (const status of [401, 404]) {
      const h = harness(() => status);
      const messages = await Effect.runPromise(
        streamMessages("t", () => 1, makeClient(h.fetchImpl)).pipe(Stream.take(1), Stream.runCollect),
      );

      expect(Array.from(messages)).toEqual([StreamTerminalError({ status })]);
    }
  });

  it("treats a silent established connection as dead after the stale timeout and reconnects", async () => {
    const h = harness();
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onStatus: (c) => status.push(c) }));
        yield* waitConn(h, 0);
        h.frame(0, { frame: "snapshot", seq: 0, state: {} });
        yield* settle;
        expect(status.at(-1)).toBe(true);

        yield* adjust(14_999);
        yield* settle;
        expect(h.conns.length).toBe(1);
        expect(status.at(-1)).toBe(true);

        yield* adjust(1);
        yield* settle;
        expect(status.at(-1)).toBe(false);
        yield* adjust(500);
        yield* settle;
        expect(h.conns.length).toBe(2);

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("heartbeat frames refresh the stale timer but never reach onFrame", async () => {
    const h = harness();
    const frames: unknown[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onFrame: (event) => frames.push(event.frame) }));
        yield* waitConn(h, 0);
        h.frame(0, { frame: "snapshot", seq: 0, state: {} });
        yield* settle;

        for (let t = 0; t < 4; t++) {
          yield* adjust(10_000);
          h.frame(0, { frame: "heartbeat" });
          yield* settle;
        }
        expect(h.conns.length).toBe(1);
        expect(frames).toEqual([{ frame: "snapshot", seq: 0, state: {} }]);

        yield* Fiber.interrupt(fiber);
      }),
    );
  });

  it("a heartbeat alone proves a reconnect is healthy", async () => {
    const h = harness();
    const status: boolean[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(observe(h, { onStatus: (c) => status.push(c) }));
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
        const fiber = yield* Effect.forkChild(observe(h, { onFrame: (event) => frames.push(event.frame) }));
        yield* waitConn(h, 0);
        h.frame(0, 1);
        yield* settle;
        expect(frames).toEqual([1]);

        yield* Fiber.interrupt(fiber);
        yield* adjust(60_000);
        yield* settle;
        expect(h.conns.length).toBe(1);
        expect(frames).toEqual([1]);
      }),
    );
  });
});
