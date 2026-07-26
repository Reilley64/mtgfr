// Priority-watch shame clock: elapsed seconds and heat escalation (Solid `watch.ts` port).

import * as Clock from "effect/Clock";
import * as Effect from "effect/Effect";
import * as Schedule from "effect/Schedule";
import * as Stream from "effect/Stream";

/** Seconds waited before the watch line escalates. */
export const EMBER_AFTER_S = 15;
export const FLARE_AFTER_S = 30;

export type Heat = "sage" | "ember" | "flare";

export function heatOf(elapsed: number): Heat {
  if (elapsed >= FLARE_AFTER_S) return "flare";
  if (elapsed >= EMBER_AFTER_S) return "ember";
  return "sage";
}

export const elapsedSeconds: Stream.Stream<number> = Stream.unwrap(
  Effect.map(Clock.currentTimeMillis, (start) =>
    Stream.fromSchedule(Schedule.spaced("1 second")).pipe(
      Stream.mapEffect(() => Effect.map(Clock.currentTimeMillis, (now) => Math.floor((now - start) / 1000))),
    ),
  ),
);

export const watchElapsed = (onTick: (seconds: number) => void): Effect.Effect<void> =>
  Stream.runForEach(elapsedSeconds, (n) => Effect.sync(() => onTick(n)));
