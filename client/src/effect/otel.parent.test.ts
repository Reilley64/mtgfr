import { describe, expect, it } from "vitest";
import { continueIncomingTrace, currentTraceparent } from "~/effect/otel";
import { parseTraceparent } from "~/lib/traceContext";

describe("continueIncomingTrace", () => {
  it("is a no-op for missing or invalid headers", () => {
    const sentinel = { _tag: "effect-stub" } as never;
    expect(continueIncomingTrace(sentinel, null)).toBe(sentinel);
    expect(continueIncomingTrace(sentinel, "bad")).toBe(sentinel);
  });

  it("is a no-op for unsampled traceparents (Faro non-recording inject)", () => {
    const sentinel = { _tag: "effect-stub" } as never;
    const unsampled = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00";
    expect(parseTraceparent(unsampled)?.traceFlags).toBe(0);
    expect(continueIncomingTrace(sentinel, unsampled)).toBe(sentinel);
  });

  it("accepts a parseable sampled Faro-style traceparent", () => {
    const header = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    expect(parseTraceparent(header)).not.toBeNull();
    // Wiring smoke: with a valid sampled header the helper must return a different Effect
    // (parent span attached) rather than the identical reference.
    const sentinel = { _tag: "effect-stub" } as never;
    expect(continueIncomingTrace(sentinel, header)).not.toBe(sentinel);
  });
});

describe("currentTraceparent", () => {
  it("is an Effect.fn (callable that returns an Effect)", () => {
    expect(typeof currentTraceparent).toBe("function");
    const effect = currentTraceparent();
    expect(effect).toBeDefined();
    expect(typeof (effect as { pipe?: unknown }).pipe).toBe("function");
  });
});
