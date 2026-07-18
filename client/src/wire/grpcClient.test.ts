import { describe, expect, it } from "vitest";
import {
  callOpts,
  GrpcCallError,
  httpStatusOf,
  SESSION_METADATA_KEY,
  TRACEPARENT_METADATA_KEY,
  toCallError,
} from "~/wire/grpcClient";

describe("toCallError", () => {
  it("keeps an existing GrpcCallError so stream mapError+catch does not collapse unavailable to unknown", () => {
    const first = new GrpcCallError("unavailable", "getaddrinfo ENOTFOUND dead-pod.svc");
    const again = toCallError(first);
    expect(again).toBe(first);
    expect(again.code).toBe("unavailable");
    expect(httpStatusOf(again.code)).toBe(503);
  });

  it("wraps a plain Error as unknown", () => {
    const err = toCallError(new Error("boom"));
    expect(err).toBeInstanceOf(GrpcCallError);
    expect(err.code).toBe("unknown");
    expect(err.message).toBe("boom");
    expect(httpStatusOf(err.code)).toBe(500);
  });
});

describe("callOpts", () => {
  it("includes session token metadata", () => {
    const opts = callOpts("tok", null);
    expect(opts?.metadata).toEqual([[SESSION_METADATA_KEY, "tok"]]);
  });

  it("includes an explicit traceparent (required for gRPC ManagedRuntime boundary)", () => {
    const opts = callOpts(null, "00-abc-def-01");
    expect(opts?.metadata).toEqual([[TRACEPARENT_METADATA_KEY, "00-abc-def-01"]]);
  });

  it("includes both session and traceparent", () => {
    const opts = callOpts("tok", "00-from-bff-01");
    expect(opts?.metadata).toEqual([
      [SESSION_METADATA_KEY, "tok"],
      [TRACEPARENT_METADATA_KEY, "00-from-bff-01"],
    ]);
  });

  it("returns undefined when neither session nor traceparent is set", () => {
    expect(callOpts(null, null)).toBeUndefined();
  });
});
