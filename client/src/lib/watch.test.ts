// The priority-watch shame clock. The escalation thresholds were previously unreachable by a
// test: the tick was a `setInterval` reading `Date.now()`, so asserting "amber at 15s, red at 30s"
// meant waiting thirty real seconds. Sleeping on the Effect Clock instead makes the whole
// escalation virtual — `TestClock.adjust` walks it in microseconds.

import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Stream from "effect/Stream";
import * as TestClock from "effect/testing/TestClock";
import { describe, expect, it } from "vitest";
import { elapsedSeconds, heatOf } from "~/lib/watch";

const adjust = (ms: number) => TestClock.adjust(Duration.millis(ms));

function drive(body: Effect.Effect<void, never, TestClock.TestClock>): Promise<void> {
  return Effect.runPromise(body.pipe(Effect.provide(TestClock.layer())));
}

/** Collect what `elapsedSeconds` has emitted so far, advancing virtual time by `ms`. */
function ticks(ms: number): Promise<number[]> {
  const seen: number[] = [];
  return drive(
    Effect.gen(function* () {
      const fiber = yield* Effect.forkChild(Stream.runForEach(elapsedSeconds, (n) => Effect.sync(() => seen.push(n))));
      yield* adjust(ms);
      yield* Fiber.interrupt(fiber);
    }),
  ).then(() => seen);
}

describe("elapsedSeconds", () => {
  it("emits nothing before the first second", async () => {
    expect(await ticks(999)).toEqual([]);
  });

  it("counts one per second, starting at 1", async () => {
    expect(await ticks(3000)).toEqual([1, 2, 3]);
  });

  it("reaches the escalation thresholds on the virtual clock", async () => {
    const seen = await ticks(30_000);
    expect(seen).toHaveLength(30);
    expect(seen.at(-1)).toBe(30);
  });

  // The reason each tick reads the clock instead of incrementing a counter: a backgrounded tab
  // throttles timers, so ticks go missing. Elapsed must stay true to the wall, as it was when this
  // read `Date.now()`. A stalled consumer stands in for the throttled tab — a counter would resume
  // at 2 here.
  it("reports true elapsed after a stall, not the number of ticks", async () => {
    const seen: number[] = [];
    await drive(
      Effect.gen(function* () {
        const fiber = yield* Effect.forkChild(
          Stream.runForEach(elapsedSeconds, (n) =>
            Effect.gen(function* () {
              seen.push(n);
              if (seen.length === 1) yield* Effect.sleep(Duration.millis(5000));
            }),
          ),
        );
        yield* adjust(1000); // first tick at t=1s, then the consumer stalls for 5s
        yield* adjust(5000);
        yield* adjust(1000); // next tick lands at t=7s
        yield* Fiber.interrupt(fiber);
      }),
    );
    expect(seen[0]).toBe(1);
    expect(seen[1]).toBe(7);
  });
});

describe("heatOf", () => {
  it("rests at sage below 15s", () => {
    expect(heatOf(0)).toBe("sage");
    expect(heatOf(14)).toBe("sage");
  });

  it("escalates to ember at exactly 15s", () => {
    expect(heatOf(15)).toBe("ember");
    expect(heatOf(29)).toBe("ember");
  });

  it("escalates to flare at exactly 30s", () => {
    expect(heatOf(30)).toBe("flare");
    expect(heatOf(999)).toBe("flare");
  });
});
