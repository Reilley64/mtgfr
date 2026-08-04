import * as Effect from "effect/Effect";
import type { H3Event } from "nitro/h3";
import { beforeEach, describe, expect, it, vi } from "vitest";
import coverageHandler from "./routes/api/meta/coverage/v1.get";

const mocks = vi.hoisted(() => ({
  fetchCoverageMeta: vi.fn(),
}));

vi.mock("../app/domain/coverage-meta", () => ({
  fetchCoverageMeta: mocks.fetchCoverageMeta,
}));

vi.mock("./lobby-http", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lobby-http")>()),
  runMetaGet: (_event: H3Event, _span: string, body: () => Effect.Effect<Response, never>) => Effect.runPromise(body()),
}));

const event = { req: { method: "GET", headers: new Headers() } } as unknown as H3Event;

describe("api meta/coverage/v1", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.fetchCoverageMeta.mockResolvedValue({ faithfulCount: 12, oracleTotal: 100, sets: [] });
  });

  // Cloudflare's Cache Rule only marks the path eligible — this header is the edge/browser TTL.
  it("sends a cache-control the edge can cache on", async () => {
    const res = await coverageHandler(event);
    expect(res.headers.get("cache-control")).toBe("public, max-age=60, s-maxage=3600, stale-while-revalidate=600");
    await expect(res.json()).resolves.toMatchObject({ faithful_count: 12, oracle_total: 100 });
  });
});
