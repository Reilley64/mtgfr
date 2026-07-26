import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import { describe, expect, it } from "vitest";
import { grpcClient } from "./grpcClient";

describe("grpcClient Effect API", () => {
  it("returns Effect values from unary methods", () => {
    const result: unknown = grpcClient("127.0.0.1:1").auth.getMe(null);
    if (result instanceof Promise) result.catch(() => undefined);

    expect(Effect.isEffect(result)).toBe(true);
  });

  it("returns a Stream from game.stream", () => {
    const frames: unknown = grpcClient("127.0.0.1:1").game.stream("TABLE", null);

    expect(Stream.isStream(frames)).toBe(true);
  });
});
