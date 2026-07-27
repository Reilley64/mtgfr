import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchCoverageMeta, joinCoverageSetRows } from "./coverage-meta";
import { loadOracleTotal } from "./scryfall-oracle-total";
import { loadSetOracleTotals } from "./scryfall-set-oracle-totals";
import { loadScryfallSets } from "./scryfall-sets";

vi.mock("./scryfall-oracle-total", () => ({
  loadOracleTotal: vi.fn(),
}));

vi.mock("./scryfall-set-oracle-totals", () => ({
  loadSetOracleTotals: vi.fn(),
}));

vi.mock("./scryfall-sets", () => ({
  loadScryfallSets: vi.fn(),
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
    vi.mocked(loadOracleTotal).mockReset();
    vi.mocked(loadSetOracleTotals).mockReset();
    vi.mocked(loadScryfallSets).mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("awaits cold Scryfall loads then joins live faithful_by_set", async () => {
    vi.mocked(loadOracleTotal).mockResolvedValue(28412);
    vi.mocked(loadSetOracleTotals).mockResolvedValue({ soc: 400 });
    vi.mocked(loadScryfallSets).mockResolvedValue([
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
    expect(loadOracleTotal).toHaveBeenCalledOnce();
    expect(loadSetOracleTotals).toHaveBeenCalledOnce();
    expect(loadScryfallSets).toHaveBeenCalledOnce();
  });
});
