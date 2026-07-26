import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchCoverageMeta, joinCoverageSetRows } from "./coverage-meta";
import { ensureOracleTotalRefresh, getCachedOracleTotal, getCachedOracleTotalBySet } from "./scryfall-oracle-total";
import { ensureScryfallSetsRefresh, getCachedScryfallSets } from "./scryfall-sets";

vi.mock("./scryfall-oracle-total", () => ({
  ensureOracleTotalRefresh: vi.fn(),
  getCachedOracleTotal: vi.fn(),
  getCachedOracleTotalBySet: vi.fn(),
}));

vi.mock("./scryfall-sets", () => ({
  ensureScryfallSetsRefresh: vi.fn(),
  getCachedScryfallSets: vi.fn(),
}));

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("joinCoverageSetRows", () => {
  it("joins only Scryfall sets and leaves missing oracle totals null", () => {
    expect(
      joinCoverageSetRows(
        [
          {
            code: "soc",
            name: "Secrets of Strixhaven Commander",
            releasedAt: "2026-04-01",
            cardCount: 400,
          },
          {
            code: "scn",
            name: "Set Without Oracle Rows",
            releasedAt: null,
            cardCount: 12,
          },
        ],
        { soc: 400, cmd: 10 },
        { soc: 10 },
      ),
    ).toEqual([
      {
        code: "soc",
        name: "Secrets of Strixhaven Commander",
        releasedAt: "2026-04-01",
        faithful: 10,
        oracleTotal: 400,
      },
      {
        code: "scn",
        name: "Set Without Oracle Rows",
        releasedAt: null,
        faithful: 0,
        oracleTotal: null,
      },
    ]);
  });
});

describe("fetchCoverageMeta", () => {
  beforeEach(() => {
    vi.mocked(ensureOracleTotalRefresh).mockReset();
    vi.mocked(getCachedOracleTotal).mockReset();
    vi.mocked(getCachedOracleTotalBySet).mockReset();
    vi.mocked(ensureScryfallSetsRefresh).mockReset();
    vi.mocked(getCachedScryfallSets).mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns joined live and cached coverage facts", async () => {
    vi.mocked(getCachedOracleTotal).mockReturnValue(28412);
    vi.mocked(getCachedOracleTotalBySet).mockReturnValue({ soc: 400 });
    vi.mocked(getCachedScryfallSets).mockReturnValue([
      {
        code: "soc",
        name: "Secrets of Strixhaven Commander",
        releasedAt: "2026-04-01",
        cardCount: 400,
      },
    ]);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        json({
          version: "1.2.3",
          faithful_count: 662,
          faithful_by_set: { soc: 10 },
        }),
      ),
    );

    await expect(fetchCoverageMeta()).resolves.toEqual({
      faithfulCount: 662,
      oracleTotal: 28412,
      sets: [
        {
          code: "soc",
          name: "Secrets of Strixhaven Commander",
          releasedAt: "2026-04-01",
          faithful: 10,
          oracleTotal: 400,
        },
      ],
    });
    expect(ensureOracleTotalRefresh).toHaveBeenCalledOnce();
    expect(ensureScryfallSetsRefresh).toHaveBeenCalledOnce();
  });
});
