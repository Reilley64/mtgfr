// The priority-watch shame clock: how long the current priority holder has been dawdling, and how
// loudly to say so. Split out of Board.tsx so the escalation is testable — the tick sleeps on the
// Effect Clock (a schedule, hence Effect's; ADR 0019), not on `setInterval` + `Date.now()`.

import * as Clock from "effect/Clock";
import * as Effect from "effect/Effect";
import * as Schedule from "effect/Schedule";
import * as Stream from "effect/Stream";

/** Seconds waited before the watch line escalates. Amber for the slowpoke, red for the villain. */
export const EMBER_AFTER_S = 15;
export const FLARE_AFTER_S = 30;

export type Heat = "sage" | "ember" | "flare";

/** How hot the watch line reads, for a given wait. */
export function heatOf(elapsed: number): Heat {
  if (elapsed >= FLARE_AFTER_S) return "flare";
  if (elapsed >= EMBER_AFTER_S) return "ember";
  return "sage";
}

/** Whole seconds since subscription, emitted about once per second. Nothing before the first
 * second — the caller renders 0 until then. Restarting the stream restarts the count, which is
 * exactly what priority changing hands should do.
 *
 * Each tick *reads* the clock rather than counting ticks: a backgrounded tab throttles timers, and
 * a counter would then under-report the wait. Reading `Clock.currentTimeMillis` keeps the old
 * `Date.now()` accuracy while staying virtual under `TestClock`. */
export const elapsedSeconds: Stream.Stream<number> = Stream.unwrap(
  Effect.map(Clock.currentTimeMillis, (start) =>
    Stream.fromSchedule(Schedule.spaced("1 second")).pipe(
      Stream.mapEffect(() => Effect.map(Clock.currentTimeMillis, (now) => Math.floor((now - start) / 1000))),
    ),
  ),
);

/** Drive `onTick` with the elapsed second count until the returned effect's fiber is interrupted. */
export const watchElapsed = (onTick: (seconds: number) => void): Effect.Effect<void> =>
  Stream.runForEach(elapsedSeconds, (n) => Effect.sync(() => onTick(n)));
