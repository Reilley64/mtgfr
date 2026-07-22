// Typed stream with manual reconnect (`client.streamSse` → `/api/rpc/game/:table/stream`).

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

export interface StreamCallbacks {
  onFrame: (frame: GameFrame) => void;
  onError?: (status: number) => void;
  onStatus?: (connected: boolean) => void;
}

/**
 * Drive `cb` off the generated SSE delta stream for `table` until the returned effect is
 * interrupted. Reconnects with exponential backoff + full jitter (reset after a healthy
 * connection); stops forever on a 4xx (a bad table / expired session won't fix itself), reporting
 * it via `onError` exactly once. `random` is injectable so the jitter is deterministic under test
 * (CLAUDE.md: inject randomness rather than reading the RNG directly).
 *
 * A silently-dead connection (killed upstream, no FIN) surfaces as neither a stream error nor a
 * close, so the fetch/TCP layer can't be relied on to notice. The server now emits a periodic
 * `Heartbeat` frame; `Stream.timeout` below re-arms on every frame and ends the stream if none
 * (not even a heartbeat) arrives within `STALE_TIMEOUT_MS`, which the reconnect loop treats as a
 * drop. This replaces the old NDJSON byte-level watchdog that was dropped with NDJSON.
 */
export const streamDeltas = Effect.fn("streamDeltas")(function* (
  table: string,
  cb: StreamCallbacks,
  random: () => number = Math.random,
  client: Client = defaultClient,
) {
  yield* Effect.annotateCurrentSpan({ table });
  let backoff = RECONNECT_BASE_MS;

  const connectOnce = Effect.gen(function* () {
    let healthy = false;
    yield* client.streamSse(table).pipe(
      Stream.timeout(Duration.millis(STALE_TIMEOUT_MS)),
      Stream.tap(() =>
        Effect.sync(() => {
          if (healthy) return;
          healthy = true;
          cb.onStatus?.(true);
          backoff = RECONNECT_BASE_MS;
        }),
      ),
      Stream.filter((frame): frame is GameFrame => frame.frame !== "heartbeat"),
      Stream.runForEach((frame) => Effect.sync(() => cb.onFrame(frame))),
    );
    return "retry" as const;
  });

  const handleFailure = (error: unknown) => {
    const status = statusOf(error);
    if (status !== undefined && status >= 400 && status < 500) {
      cb.onError?.(status);
      return Effect.succeed("stop" as const);
    }
    return Effect.succeed("retry" as const);
  };

  while (true) {
    const outcome = yield* connectOnce.pipe(Effect.catch(handleFailure));
    if (outcome === "stop") return;
    cb.onStatus?.(false);
    const wait = backoff * (0.5 + random() * 0.5);
    yield* Effect.sleep(Duration.millis(wait));
    backoff = Math.min(backoff * 2, RECONNECT_CAP_MS);
  }
});
