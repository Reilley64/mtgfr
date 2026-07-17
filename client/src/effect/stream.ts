// Typed stream with manual reconnect (`client.streamSse` → `/api/rpc/game/:table/stream`).

import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import { type Client, client as defaultClient, statusOf } from "~/effect/client";
import type { StreamFrame } from "~/wire/types";

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
export function streamDeltas(
  table: string,
  cb: StreamCallbacks,
  random: () => number = Math.random,
  client: Client = defaultClient,
): Effect.Effect<void> {
  return Effect.gen(function* () {
    let backoff = RECONNECT_BASE_MS;

    // One connection attempt. "stop" ends the loop for good (a terminal 4xx); "retry" means the
    // caller should back off and reconnect (5xx, transport error, SSE `retry`, or a clean drop).
    const connectOnce = Effect.gen(function* () {
      let healthy = false;
      yield* client.streamSse(table).pipe(
        // `Stream.timeout` re-arms on every pull, so *any* frame — heartbeat included — refreshes
        // the window; the stream just ends if none arrives within it, which the loop below treats
        // like any other drop (onStatus(false) + backoff reconnect). This is what catches a
        // silently-dead upstream that never sends a FIN. Heartbeats are the liveness signal, so
        // they must pass through the timeout *before* being filtered out of the store below.
        Stream.timeout(Duration.millis(STALE_TIMEOUT_MS)),
        // Liveness is proven by *any* frame, so this runs before the heartbeat filter below: a
        // reconnect onto a quiet game sends heartbeats and nothing else, and must still clear the
        // reconnect banner.
        Stream.tap(() =>
          Effect.sync(() => {
            if (healthy) return;
            healthy = true;
            cb.onStatus?.(true);
            backoff = RECONNECT_BASE_MS; // first frame proves the connection — reset the backoff
          }),
        ),
        Stream.filter((frame): frame is GameFrame => frame.frame !== "heartbeat"),
        Stream.runForEach((frame) => Effect.sync(() => cb.onFrame(frame))),
      );
      return "retry" as const; // the stream ended — the server dropped us
    });

    // 4xx won't self-resolve by hammering — report and quit; everything else backs off and retries.
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
      // Full jitter (wait * 0.5–1.0) so a fleet of clients doesn't reconnect in lockstep, then
      // double the base up to the cap.
      const wait = backoff * (0.5 + random() * 0.5);
      yield* Effect.sleep(Duration.millis(wait));
      backoff = Math.min(backoff * 2, RECONNECT_CAP_MS);
    }
  });
}
