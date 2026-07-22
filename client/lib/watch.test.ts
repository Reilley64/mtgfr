import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";
import * as Fiber from "effect/Fiber";
import * as Stream from "effect/Stream";
import * as TestClock from "effect/testing/TestClock";
import { describe, expect, it } from "vitest";
import { elapsedSeconds, heatOf } from "./watch";

const adjust = (ms: number) => TestClock.adjust(Duration.millis(ms));

function drive(body: Effect.Effect<void, never, TestClock.TestClock>): Promise<void> {
  return Effect.runPromise(body.pipe(Effect.provide(TestClock.layer())));
}

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
});

describe("heatOf", () => {
  it("rests at sage below 15s", () => {
    expect(heatOf(0)).toBe("sage");
    expect(heatOf(14)).toBe("sage");
  });

  it("escalates to ember at 15s and flare at 30s", () => {
    expect(heatOf(15)).toBe("ember");
    expect(heatOf(29)).toBe("ember");
    expect(heatOf(30)).toBe("flare");
  });
});
