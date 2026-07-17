import { afterEach, describe, expect, it } from "vitest";
import { grpcUpstream, parseMePayload } from "~/lib/apiUpstreamAuth";

describe("parseMePayload", () => {
  it("accepts a full Me body", () => {
    expect(parseMePayload({ id: 7, email: "a@b.c", username: "host" })).toEqual({
      id: 7,
      email: "a@b.c",
      username: "host",
    });
  });

  it("rejects the historical shape that omitted id (would break lobby host_user_id)", () => {
    expect(parseMePayload({ email: "a@b.c", username: "host" })).toBeNull();
  });

  it("rejects non-objects", () => {
    expect(parseMePayload(null)).toBeNull();
    expect(parseMePayload("nope")).toBeNull();
  });
});

describe("grpcUpstream", () => {
  const saved = { API_UPSTREAM: process.env.API_UPSTREAM, GRPC_UPSTREAM: process.env.GRPC_UPSTREAM };
  afterEach(() => {
    process.env.API_UPSTREAM = saved.API_UPSTREAM;
    process.env.GRPC_UPSTREAM = saved.GRPC_UPSTREAM;
  });

  it("defaults to 127.0.0.1:50051 (same host as the default HTTP upstream, gRPC port)", () => {
    delete process.env.API_UPSTREAM;
    delete process.env.GRPC_UPSTREAM;
    expect(grpcUpstream()).toBe("127.0.0.1:50051");
  });

  it("derives the gRPC port from a custom API_UPSTREAM host", () => {
    delete process.env.GRPC_UPSTREAM;
    process.env.API_UPSTREAM = "http://edh-api.internal:8080";
    expect(grpcUpstream()).toBe("edh-api.internal:50051");
  });

  it("honors an explicit GRPC_UPSTREAM override", () => {
    process.env.GRPC_UPSTREAM = "edh-api-grpc.internal:50051";
    expect(grpcUpstream()).toBe("edh-api-grpc.internal:50051");
  });
});
