import { describe, expect, it } from "vitest";
import {
  cookieValue,
  DEV_UPSTREAM,
  isBlockedPublicApiPath,
  parseUpstreamsJson,
  resolveUpstreamBase,
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

describe("isBlockedPublicApiPath", () => {
  it("blocks admin and health/drain", () => {
    expect(isBlockedPublicApiPath("admin/drain")).toBe(true);
    expect(isBlockedPublicApiPath("health/drain")).toBe(true);
    expect(isBlockedPublicApiPath("tables/x/stream/v1")).toBe(false);
  });
});
