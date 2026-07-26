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
              set_type: "commander",
              digital: false,
            },
            {
              code: "asos",
              name: "Secrets of Strixhaven Art Series",
              released_at: "2026-04-24",
              card_count: 54,
              set_type: "memorabilia",
              digital: false,
            },
            {
              code: "tsoc",
              name: "Secrets of Strixhaven Commander Tokens",
              released_at: "2026-04-24",
              card_count: 30,
              set_type: "token",
              digital: false,
            },
            {
              code: "amini",
              name: "Example Minigames",
              released_at: "2023-01-01",
              card_count: 5,
              set_type: "minigame",
              digital: false,
            },
            {
              code: "van",
              name: "Vanguard Series",
              released_at: "1997-01-01",
              card_count: 32,
              set_type: "vanguard",
              digital: false,
            },
            {
              code: "empty",
              name: "Empty Set",
              released_at: null,
              card_count: 0,
              set_type: "expansion",
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

  it("serves stale sets when refresh returns an empty array after a warm cache", async () => {
    let empty = false;
    const fetchImpl = vi.fn(async () => {
      if (empty) return new Response(JSON.stringify({ data: [] }), { status: 200 });
      return new Response(
        JSON.stringify({
          data: [{ code: "soc", name: "Secrets of Strixhaven", released_at: "2026-04-01", card_count: 400 }],
        }),
        { status: 200 },
      );
    });

    await refreshScryfallSets(fetchImpl as unknown as typeof fetch);
    vi.setSystemTime(new Date("2026-07-28T00:00:00Z"));
    empty = true;

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
    const fetchImpl = vi.fn(
      async () =>
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
