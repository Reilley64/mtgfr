import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchApiMeta, parseLiveStatus } from "./api-upstream-auth";
import { ensureOracleTotalRefresh, getCachedOracleTotal } from "./scryfall-oracle-total";

vi.mock("./scryfall-oracle-total", () => ({
  ensureOracleTotalRefresh: vi.fn(),
  getCachedOracleTotal: vi.fn(),
}));

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("parseLiveStatus", () => {
  it("reads version and faithful_count", () => {
    expect(parseLiveStatus({ version: "1.2.3", faithful_count: 662 })).toEqual({
      version: "1.2.3",
      faithfulCount: 662,
    });
  });

  it("keeps version when faithful_count is missing", () => {
    expect(parseLiveStatus({ version: "1.2.3" })).toEqual({
      version: "1.2.3",
      faithfulCount: null,
    });
  });

  it("treats non-finite faithful_count as null", () => {
    expect(parseLiveStatus({ version: "1.2.3", faithful_count: Number.POSITIVE_INFINITY })).toEqual({
      version: "1.2.3",
      faithfulCount: null,
    });
  });
});

describe("fetchApiMeta", () => {
  beforeEach(() => {
    vi.mocked(ensureOracleTotalRefresh).mockReset();
    vi.mocked(getCachedOracleTotal).mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns live status fields and cached oracle total", async () => {
    vi.mocked(getCachedOracleTotal).mockReturnValue(28412);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ version: "1.2.3", faithful_count: 662 })),
    );

    await expect(fetchApiMeta()).resolves.toEqual({
      version: "1.2.3",
      faithfulCount: 662,
      oracleTotal: 28412,
    });
    expect(ensureOracleTotalRefresh).toHaveBeenCalledOnce();
  });

  it("keeps version when live coverage is absent", async () => {
    vi.mocked(getCachedOracleTotal).mockReturnValue(28412);
    vi.stubGlobal("fetch", vi.fn(async () => json({ version: "1.2.3" })));

    await expect(fetchApiMeta()).resolves.toEqual({
      version: "1.2.3",
      faithfulCount: null,
      oracleTotal: 28412,
    });
    expect(ensureOracleTotalRefresh).toHaveBeenCalledOnce();
  });

  it("falls back to cached oracle total when live status is unavailable", async () => {
    vi.mocked(getCachedOracleTotal).mockReturnValue(28412);
    vi.stubGlobal("fetch", vi.fn(async () => json({ error: true }, 503)));

    await expect(fetchApiMeta()).resolves.toEqual({
      version: null,
      faithfulCount: null,
      oracleTotal: 28412,
    });
  });

  it("returns null meta fields when the oracle cache is cold", async () => {
    vi.mocked(getCachedOracleTotal).mockReturnValue(null);
    vi.stubGlobal("fetch", vi.fn(async () => {
      throw new Error("upstream down");
    }));

    await expect(fetchApiMeta()).resolves.toEqual({
      version: null,
      faithfulCount: null,
      oracleTotal: null,
    });
  });
});
