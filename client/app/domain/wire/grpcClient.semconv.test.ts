import { GrpcStatusError } from "@effect-grpc/effect-grpc";
import * as Effect from "effect/Effect";
import type * as Tracer from "effect/Tracer";
import { describe, expect, it } from "vitest";
import { rpcAttrs } from "../otel/semconv";
import { grpcSpanName, withRpcSpan } from "./grpcClient";

describe("grpc client semconv", () => {
  it("maps Game/SubmitIntent to rpc semconv", () => {
    expect(rpcAttrs({ service: "mtgfr.v1.Game", method: "SubmitIntent" })).toMatchObject({
      "rpc.system": "grpc",
      "rpc.service": "mtgfr.v1.Game",
      "rpc.method": "SubmitIntent",
    });
  });

  it("builds span name from service method", () => {
    expect(grpcSpanName("mtgfr.v1.Game", "SubmitIntent")).toBe("mtgfr.v1.Game/SubmitIntent");
  });

  it("wraps outbound calls in an rpc semconv span", async () => {
    const span = await Effect.runPromise(withRpcSpan("mtgfr.v1.Game", "SubmitIntent", Effect.currentSpan));

    expect(span.name).toBe("mtgfr.v1.Game/SubmitIntent");
    expect(Object.fromEntries(span.attributes)).toMatchObject({
      "rpc.system": "grpc",
      "rpc.service": "mtgfr.v1.Game",
      "rpc.method": "SubmitIntent",
    });
  });

  it("annotates GrpcStatusError failures without leaking the message", async () => {
    let capturedSpan: Tracer.Span | undefined;
    const error = GrpcStatusError.unavailable("secret hand payload");

    await Effect.runPromise(
      Effect.exit(
        withRpcSpan(
          "mtgfr.v1.Game",
          "SubmitIntent",
          Effect.gen(function* () {
            capturedSpan = yield* Effect.currentSpan;
            return yield* Effect.fail(error);
          }),
        ),
      ),
    );

    if (!capturedSpan) throw new Error("expected rpc span");
    const attrs = Object.fromEntries(capturedSpan.attributes);
    expect(attrs).toMatchObject({
      "rpc.grpc.status_code": "unavailable",
      "exception.type": "GrpcStatusError",
    });
    expect(attrs).not.toHaveProperty("exception.message");
    expect(Object.values(attrs)).not.toContain("secret hand payload");
  });
});
