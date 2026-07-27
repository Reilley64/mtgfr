// Typed stream with manual reconnect (`client.streamSse` -> `/api/rpc/game/:table/stream`).

import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import { type Client, client as defaultClient, statusOf } from "../rpc-client";
import type { StreamFrame } from "../wire/types";

/** Backoff bounds: start at 500ms, double per failed reconnect, cap at 10s. */
const RECONNECT_BASE_MS = 500;
const RECONNECT_CAP_MS = 10_000;

/** Kill an established connection after this long with *no* frame of any kind. The server emits a
 * heartbeat every ~5s, so three can be missed before we give up — long enough not to false-trip on
 * a quiet game, short enough to notice a silently-dead upstream. Deterministic under `TestClock`. */
const STALE_TIMEOUT_MS = 15_000;

/** A frame that actually carries game state — every `StreamFrame` except the liveness heartbeat,
 * which the stream filters out before `onFrame` so the store never sees it. */
export type GameFrame = Exclude<StreamFrame, { frame: "heartbeat" }>;

export type GameStreamEvent =
  | { readonly kind: "frame"; readonly frame: GameFrame }
  | { readonly kind: "status"; readonly connected: boolean }
  | { readonly kind: "terminal-error"; readonly status: number };

type RetryEvent = { readonly kind: "retry" };
type StopEvent = { readonly kind: "stop" };
type InternalStreamEvent = GameStreamEvent | RetryEvent | StopEvent;

const retryEvent: RetryEvent = { kind: "retry" };
const stopEvent: StopEvent = { kind: "stop" };

const statusEvent = (connected: boolean): GameStreamEvent => ({ kind: "status", connected });
const terminalErrorEvent = (status: number): GameStreamEvent => ({ kind: "terminal-error", status });
const frameEvent = (frame: GameFrame): GameStreamEvent => ({ kind: "frame", frame });
const retryStream = (): Stream.Stream<InternalStreamEvent> => Stream.make(retryEvent);
const terminalStopStream = (status: number): Stream.Stream<InternalStreamEvent> =>
  Stream.make(terminalErrorEvent(status), stopEvent);

/**
 * Stream generated SSE deltas for `table`. Reconnects with exponential backoff + full jitter
 * (reset after a healthy connection); stops forever on a 4xx (a bad table / expired session won't
 * fix itself), emitting it as a terminal error exactly once. `random` is injectable so the jitter is
 * deterministic under test (CLAUDE.md: inject randomness rather than reading the RNG directly).
 *
 * A silently-dead connection (killed upstream, no FIN) surfaces as neither a stream error nor a
 * close, so the fetch/TCP layer can't be relied on to notice. The server now emits a periodic
 * `Heartbeat` frame; `Stream.timeout` below re-arms on every frame and ends the stream if none
 * (not even a heartbeat) arrives within `STALE_TIMEOUT_MS`, which the reconnect loop treats as a
 * drop. This replaces the old NDJSON byte-level watchdog that was dropped with NDJSON.
 */
export function streamDeltas(
  table: string,
  random: () => number = Math.random,
  client: Client = defaultClient,
): Stream.Stream<GameStreamEvent> {
  return Stream.fromEffect(Effect.annotateCurrentSpan({ table })).pipe(
    Stream.flatMap(() => reconnectingStream(table, RECONNECT_BASE_MS, random, client)),
  );
}

function reconnectingStream(
  table: string,
  backoff: number,
  random: () => number,
  client: Client,
): Stream.Stream<GameStreamEvent> {
  return Stream.suspend(() => {
    let healthy = false;
    let nextBackoff = backoff;

    const connection: Stream.Stream<InternalStreamEvent> = client.streamSse(table).pipe(
      Stream.timeout(Duration.millis(STALE_TIMEOUT_MS)),
      Stream.mapEffect((frame) =>
        Effect.sync(() => {
          const events: Array<InternalStreamEvent> = [];
          if (!healthy) {
            healthy = true;
            nextBackoff = RECONNECT_BASE_MS;
            events.push(statusEvent(true));
          }
          if (frame.frame !== "heartbeat") {
            events.push(frameEvent(frame));
          }
          return events;
        }),
      ),
      Stream.flatMap((events) => Stream.fromIterable(events)),
      Stream.concat(retryStream()),
      Stream.catch((error) => {
        const status = statusOf(error);
        if (status !== undefined && status >= 400 && status < 500) {
          return terminalStopStream(status);
        }
        return retryStream();
      }),
    );

    return connection.pipe(Stream.flatMap((event) => eventToStream(event, table, nextBackoff, random, client)));
  });
}

function eventToStream(
  event: InternalStreamEvent,
  table: string,
  backoff: number,
  random: () => number,
  client: Client,
): Stream.Stream<GameStreamEvent> {
  switch (event.kind) {
    case "frame":
    case "status":
    case "terminal-error":
      return Stream.make(event);
    case "retry": {
      const wait = backoff * (0.5 + random() * 0.5);
      const nextBackoff = Math.min(backoff * 2, RECONNECT_CAP_MS);
      return Stream.make(statusEvent(false)).pipe(
        Stream.concat(Stream.fromEffectDrain(Effect.sleep(Duration.millis(wait)))),
        Stream.concat(reconnectingStream(table, nextBackoff, random, client)),
      );
    }
    case "stop":
      return Stream.empty;
    default: {
      const _exhaustive: never = event;
      return _exhaustive;
    }
  }
}
