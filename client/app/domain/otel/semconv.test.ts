import { describe, expect, it } from "vitest";
import {
  assertNoForbiddenKeys,
  dbAttrs,
  FORBIDDEN_ATTR_KEYS,
  httpServerAttrs,
  MTGFR_INTENT_ACCEPTED,
  MTGFR_INTENT_KIND,
  MTGFR_TABLE_ID,
  rpcAttrs,
} from "./semconv";

describe("otel semconv dictionary", () => {
  it("builds HTTP server attrs with 1.37 keys", () => {
    expect(httpServerAttrs({ method: "GET", route: "GET /api/meta", statusCode: 200 })).toEqual({
      "http.request.method": "GET",
      "http.route": "GET /api/meta",
      "http.response.status_code": 200,
    });
  });

  it("does not emit legacy HTTP or RPC path keys from HTTP server attrs", () => {
    const attrs = httpServerAttrs({ method: "POST", route: "rpc game/submit", statusCode: 200 });

    expect(attrs).not.toHaveProperty("http.method");
    expect(attrs).not.toHaveProperty("rpc.path");
  });

  it("builds rpc attrs without full path as rpc.method", () => {
    expect(rpcAttrs({ service: "mtgfr.v1.Game", method: "SubmitIntent", statusCode: 0 })).toEqual({
      "rpc.system": "grpc",
      "rpc.service": "mtgfr.v1.Game",
      "rpc.method": "SubmitIntent",
      "rpc.grpc.status_code": 0,
    });
  });

  it("builds safe db attrs without query text", () => {
    const attrs = dbAttrs({ operation: "SELECT", namespace: "mtgfr_web" });
    expect(attrs).toEqual({
      "db.system": "postgresql",
      "db.operation.name": "SELECT",
      "db.namespace": "mtgfr_web",
    });
    assertNoForbiddenKeys(attrs);
  });

  it("rejects forbidden keys including intent payload and query text", () => {
    expect(FORBIDDEN_ATTR_KEYS.has("db.query.text")).toBe(true);
    expect(FORBIDDEN_ATTR_KEYS.has("mtgfr.intent.payload")).toBe(true);
    expect(() =>
      assertNoForbiddenKeys({
        [MTGFR_INTENT_KIND]: "pass",
        "mtgfr.intent.payload": "{}",
      }),
    ).toThrow(/forbidden/);
  });

  it("SubmitIntent-shaped sample has kind/accepted only — no payload/hand", () => {
    const sample = {
      [MTGFR_TABLE_ID]: "tbl-1",
      [MTGFR_INTENT_KIND]: "pass",
      [MTGFR_INTENT_ACCEPTED]: true,
    };
    assertNoForbiddenKeys(sample);
    expect(sample).not.toHaveProperty("mtgfr.intent.payload");
    expect(Object.keys(sample).some((k) => k.includes("hand"))).toBe(false);
  });
});
