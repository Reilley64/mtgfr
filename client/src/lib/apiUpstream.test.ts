import { describe, expect, it } from "vitest";
import { normalizePublicApiPath, tableIdFromGamePath, upstreamFromPodDns } from "~/lib/apiUpstream";

describe("normalizePublicApiPath", () => {
  it("rejects traversal, encoding tricks, admin/drain, and public seed", () => {
    expect(normalizePublicApiPath("%2e%2e/admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin%2Fdrain")).toBeNull();
    expect(normalizePublicApiPath("x/../admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin/drain")).toBeNull();
    expect(normalizePublicApiPath("admin")).toBeNull();
    expect(normalizePublicApiPath("health/drain")).toBeNull();
    expect(normalizePublicApiPath("health/drain/")).toBeNull();
    expect(normalizePublicApiPath("tables/seed/v1")).toBeNull();
    expect(normalizePublicApiPath("/tables/join/v1")).toBe("tables/join/v1");
    expect(normalizePublicApiPath("tables/x/stream/v1")).toBe("tables/x/stream/v1");
  });
});

describe("tableIdFromGamePath", () => {
  it("extracts table ids from game routes only", () => {
    expect(tableIdFromGamePath("tables/ABC123/stream/v1")).toBe("ABC123");
    expect(tableIdFromGamePath("tables/ABC123/intent/v1")).toBe("ABC123");
    expect(tableIdFromGamePath("tables/ABC123/yield/v1")).toBe("ABC123");
    expect(tableIdFromGamePath("tables/ABC123/turn-yield/v1")).toBe("ABC123");
    expect(tableIdFromGamePath("tables/ABC123/stack-dwell/v1")).toBe("ABC123");
    expect(tableIdFromGamePath("tables/join/v1")).toBeNull();
    expect(tableIdFromGamePath("auth/me/v1")).toBeNull();
  });
});

describe("upstreamFromPodDns", () => {
  it("builds http://{pod}:8080 for bare pod DNS from seed", () => {
    expect(upstreamFromPodDns("edh-api-1-2-3-abc.edh-api-headless.edh.svc.cluster.local")).toBe(
      "http://edh-api-1-2-3-abc.edh-api-headless.edh.svc.cluster.local:8080",
    );
  });

  it("accepts an absolute URL and strips a trailing slash", () => {
    expect(upstreamFromPodDns("http://127.0.0.1:8080/")).toBe("http://127.0.0.1:8080");
    expect(upstreamFromPodDns("https://api.example/")).toBe("https://api.example");
  });
});
