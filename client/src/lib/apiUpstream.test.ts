import { describe, expect, it } from "vitest";
import {
  cookieValue,
  DEV_UPSTREAM,
  isUnknownTableLobbyBody,
  normalizePublicApiPath,
  parseUpstreamsJson,
  resolveUpstreamBase,
  upstreamBasesInOrder,
} from "~/lib/apiUpstream";

describe("parseUpstreamsJson", () => {
  it("returns empty for missing or invalid JSON", () => {
    expect(parseUpstreamsJson(undefined)).toEqual({});
    expect(parseUpstreamsJson("not-json")).toEqual({});
    expect(parseUpstreamsJson("[]")).toEqual({});
  });

  it("strips trailing slashes", () => {
    expect(parseUpstreamsJson('{"a":"http://x:8080/"}')).toEqual({ a: "http://x:8080" });
  });
});

describe("cookieValue", () => {
  it("reads a named cookie", () => {
    expect(cookieValue("a=1; mtgfr-instance=edh-api-1-2-3; b=2", "mtgfr-instance")).toBe("edh-api-1-2-3");
  });
});

describe("resolveUpstreamBase", () => {
  const upstreams = JSON.stringify({
    "edh-api-1-2-0": "http://edh-api-1-2-0.edh.svc:8080",
    "edh-api-1-1-0": "http://edh-api-1-1-0.edh.svc:8080",
  });

  it("falls back to localhost when the map is unset", () => {
    expect(resolveUpstreamBase({})).toBe(DEV_UPSTREAM);
  });

  it("routes by mtgfr-instance cookie when known", () => {
    expect(
      resolveUpstreamBase({
        upstreamsJson: upstreams,
        activeInstanceId: "edh-api-1-2-0",
        cookieHeader: "mtgfr-instance=edh-api-1-1-0",
      }),
    ).toBe("http://edh-api-1-1-0.edh.svc:8080");
  });

  it("uses the active instance when the cookie is missing or unknown", () => {
    expect(
      resolveUpstreamBase({
        upstreamsJson: upstreams,
        activeInstanceId: "edh-api-1-2-0",
      }),
    ).toBe("http://edh-api-1-2-0.edh.svc:8080");
    expect(
      resolveUpstreamBase({
        upstreamsJson: upstreams,
        activeInstanceId: "edh-api-1-2-0",
        cookieHeader: "mtgfr-instance=gone",
      }),
    ).toBe("http://edh-api-1-2-0.edh.svc:8080");
  });
});

describe("normalizePublicApiPath", () => {
  it("rejects traversal, encoding tricks, and admin/drain", () => {
    expect(normalizePublicApiPath("%2e%2e/admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin%2Fdrain")).toBeNull();
    expect(normalizePublicApiPath("x/../admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin")).toBeNull();
    expect(normalizePublicApiPath("health/drain")).toBeNull();
    expect(normalizePublicApiPath("health/drain/")).toBeNull();
    expect(normalizePublicApiPath("/tables/join/v1")).toBe("tables/join/v1");
    expect(normalizePublicApiPath("tables/x/stream/v1")).toBe("tables/x/stream/v1");
  });
});

describe("upstreamBasesInOrder", () => {
  const upstreams = JSON.stringify({
    "edh-api-1-2-0": "http://edh-api-1-2-0.edh.svc:8080",
    "edh-api-1-1-0": "http://edh-api-1-1-0.edh.svc:8080",
    "edh-api-1-0-0": "http://edh-api-1-0-0.edh.svc:8080",
  });

  it("orders sticky cookie, then active, then remaining peers", () => {
    expect(
      upstreamBasesInOrder({
        upstreamsJson: upstreams,
        activeInstanceId: "edh-api-1-2-0",
        cookieHeader: "mtgfr-instance=edh-api-1-1-0",
      }),
    ).toEqual([
      "http://edh-api-1-1-0.edh.svc:8080",
      "http://edh-api-1-2-0.edh.svc:8080",
      "http://edh-api-1-0-0.edh.svc:8080",
    ]);
  });
});

describe("isUnknownTableLobbyBody", () => {
  it("detects lobby UnknownTable errors only", () => {
    expect(isUnknownTableLobbyBody('{"error":"UnknownTable","table_id":"ABC"}')).toBe(true);
    expect(isUnknownTableLobbyBody('{"error":null}')).toBe(false);
    expect(isUnknownTableLobbyBody("not-json")).toBe(false);
  });
});
