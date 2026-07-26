import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  __inflightScryfallSetsForTests,
  __resetScryfallSetsCacheForTests,
  ensureScryfallSetsRefresh,
  getCachedScryfallSets,
  refreshScryfallSets,
} from "./scryfall-sets";

describe("scryfall sets cache", () => {
  beforeEach(() => {
    __resetScryfallSetsCacheForTests();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-26T00:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("returns null before any successful refresh", () => {
    expect(getCachedScryfallSets()).toBeNull();
  });

  it("caches non-empty Scryfall sets and sends the project user agent", async () => {
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      expect(String(input)).toBe("https://api.scryfall.com/sets");
      const headers = init?.headers as Record<string, string> | undefined;
      expect(headers?.["User-Agent"]).toBe("edh.reilley.dev/0.1");
      return new Response(
        JSON.stringify({
          data: [
            {
              code: "soc",
              name: "Secrets of Strixhaven",
              released_at: "2026-04-01",
              card_count: 400,
              digital: false,
            },
            {
              code: "token",
              name: "Tokens",
              released_at: null,
              card_count: 0,
              digital: false,
            },
          ],
        }),
        { status: 200 },
      );
    });

    const rows = await refreshScryfallSets(fetchImpl as unknown as typeof fetch);

    expect(rows).toEqual([
      {
        code: "soc",
        name: "Secrets of Strixhaven",
        releasedAt: "2026-04-01",
        cardCount: 400,
      },
    ]);
    expect(getCachedScryfallSets()).toEqual(rows);
  });

  it("serves stale sets when refresh fails after a warm cache", async () => {
    let fail = false;
    const fetchImpl = vi.fn(async () => {
      if (fail) return new Response("nope", { status: 503 });
      return new Response(
        JSON.stringify({
          data: [{ code: "soc", name: "Secrets of Strixhaven", released_at: "2026-04-01", card_count: 400 }],
        }),
        { status: 200 },
      );
    });

    await refreshScryfallSets(fetchImpl as unknown as typeof fetch);
    vi.setSystemTime(new Date("2026-07-28T00:00:00Z"));
    fail = true;

    const rows = await refreshScryfallSets(fetchImpl as unknown as typeof fetch);

    expect(rows).toEqual([
      {
        code: "soc",
        name: "Secrets of Strixhaven",
        releasedAt: "2026-04-01",
        cardCount: 400,
      },
    ]);
  });

  it("ensureScryfallSetsRefresh does not block and populates the cache", async () => {
    const fetchImpl = vi.fn(async () =>
      new Response(
        JSON.stringify({
          data: [{ code: "soc", name: "Secrets of Strixhaven", released_at: "2026-04-01", card_count: 400 }],
        }),
        { status: 200 },
      ),
    );

    ensureScryfallSetsRefresh(fetchImpl as unknown as typeof fetch);

    expect(getCachedScryfallSets()).toBeNull();
    await __inflightScryfallSetsForTests();
    expect(getCachedScryfallSets()).toEqual([
      {
        code: "soc",
        name: "Secrets of Strixhaven",
        releasedAt: "2026-04-01",
        cardCount: 400,
      },
    ]);
  });
});
