import { describe, expect, it } from "vitest";
import { GrpcCallError, httpStatusOf, toCallError } from "~/wire/grpcClient";

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
