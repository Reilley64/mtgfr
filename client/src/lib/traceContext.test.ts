import { describe, expect, it } from "vitest";
import { formatTraceparent, parseTraceparent } from "~/lib/traceContext";

describe("parseTraceparent", () => {
  it("parses a valid sampled header", () => {
    expect(
      parseTraceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
    ).toEqual({
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      spanId: "00f067aa0ba902b7",
      traceFlags: 0x01,
    });
  });

  it("rejects blank, malformed, and all-zero ids", () => {
    expect(parseTraceparent(null)).toBeNull();
    expect(parseTraceparent("")).toBeNull();
    expect(parseTraceparent("not-a-traceparent")).toBeNull();
    expect(
      parseTraceparent("00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
    ).toBeNull();
    expect(
      parseTraceparent("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01"),
    ).toBeNull();
  });
});

describe("formatTraceparent", () => {
  it("round-trips sampled and unsampled flags", () => {
    const sampled = {
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      spanId: "00f067aa0ba902b7",
      sampled: true as const,
    };
    expect(formatTraceparent(sampled)).toBe(
      "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
    );
    expect(parseTraceparent(formatTraceparent({ ...sampled, sampled: false }))).toEqual({
      traceId: sampled.traceId,
      spanId: sampled.spanId,
      traceFlags: 0x00,
    });
  });
});
