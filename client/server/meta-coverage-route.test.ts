import * as Effect from "effect/Effect";
import type { H3Event } from "nitro/h3";
import { beforeEach, describe, expect, it, vi } from "vitest";
import coverageHandler from "./routes/api/meta/coverage/v1.get";

const mocks = vi.hoisted(() => ({
  fetchCoverageMeta: vi.fn(),
  runTracedRequest: vi.fn(),
}));

vi.mock("../app/domain/coverage-meta", () => ({
  fetchCoverageMeta: mocks.fetchCoverageMeta,
}));

vi.mock("../app/domain/otel", () => ({
  runTracedRequest: mocks.runTracedRequest,
}));

const event = { req: new Request("http://test.local") } as unknown as H3Event;

const warmMeta = {
  faithfulCount: 12,
  oracleTotal: 100,
  sets: [{ code: "blb", name: "Bloomburrow", releasedAt: "2024-08-02", faithful: 3, oracleTotal: 40 }],
};

describe("api meta/coverage/v1", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.runTracedRequest.mockImplementation((_traceparent, _spanName, body) => Effect.runPromise(body));
  });

  // Cloudflare's Cache Rule only marks the path eligible — this header is the edge/browser TTL.
  it("sends a cache-control the edge can cache on", async () => {
    mocks.fetchCoverageMeta.mockResolvedValue(warmMeta);

    const res = await coverageHandler(event);

    expect(res.headers.get("cache-control")).toBe("public, max-age=60, s-maxage=3600, stale-while-revalidate=600");
    await expect(res.json()).resolves.toMatchObject({ faithful_count: 12, oracle_total: 100 });
  });

  it.each([
    ["the API is unreachable", { ...warmMeta, faithfulCount: null }],
    ["the oracle total is unknown", { ...warmMeta, oracleTotal: null }],
    ["the Scryfall set cache is cold", { ...warmMeta, sets: [] }],
  ])("does not let the edge cache a degraded read when %s", async (_case, meta) => {
    mocks.fetchCoverageMeta.mockResolvedValue(meta);

    const res = await coverageHandler(event);

    expect(res.headers.get("cache-control")).toBeNull();
  });
});
