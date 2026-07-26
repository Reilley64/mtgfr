import { gzipSync } from "node:zlib";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  __inflightSetOracleTotalsForTests,
  __resetSetOracleTotalsCacheForTests,
  ensureSetOracleTotalsRefresh,
  getCachedSetOracleTotals,
  parseSetOracleTotals,
  refreshSetOracleTotals,
} from "./scryfall-set-oracle-totals";

function gzipJsonl(lines: string[]): Uint8Array {
  return Uint8Array.from(gzipSync(Buffer.from(`${lines.join("\n")}\n`, "utf8")));
}

describe("parseSetOracleTotals", () => {
  it("counts unique oracle_ids per set across printings", () => {
    const text = [
      '{"oracle_id":"o1","set":"cmd"}',
      '{"oracle_id":"o1","set":"c16"}',
      '{"oracle_id":"o2","set":"cmd"}',
      '{"oracle_id":"o1","set":"CMD"}',
    ].join("\n");

    expect(parseSetOracleTotals(text)).toEqual({ c16: 1, cmd: 2 });
  });

  it("ignores blank and malformed rows", () => {
    const text = ['{"oracle_id":"o1","set":"soc"}', '{"oracle_id":7,"set":"soc"}', '{"oracle_id":"o2"}', ""].join("\n");

    expect(parseSetOracleTotals(text)).toEqual({ soc: 1 });
  });
});

describe("scryfall set oracle totals cache", () => {
  beforeEach(() => {
    __resetSetOracleTotalsCacheForTests();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-26T00:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("downloads default_cards and caches unique oracle totals per set", async () => {
    const gz = gzipJsonl([
      '{"oracle_id":"o1","set":"soc"}',
      '{"oracle_id":"o1","set":"SOC"}',
      '{"oracle_id":"o2","set":"soc"}',
      '{"oracle_id":"o1","set":"cmd"}',
    ]);
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/default-cards")) {
        return new Response(
          JSON.stringify({
            jsonl_download_uri: "https://data.scryfall.io/default-cards/test.jsonl.gz",
          }),
          { status: 200 },
        );
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });

    const totals = await refreshSetOracleTotals(fetchImpl as unknown as typeof fetch);

    expect(totals).toEqual({ cmd: 1, soc: 2 });
    expect(getCachedSetOracleTotals()).toEqual({ cmd: 1, soc: 2 });
    expect(fetchImpl.mock.calls[0]?.[0]).toEqual(expect.stringContaining("bulk-data/default-cards"));
    const init = fetchImpl.mock.calls[0]?.[1] as RequestInit | undefined;
    const headers = init?.headers as Record<string, string>;
    expect(headers["User-Agent"]).toBe("edh.reilley.dev/0.1");
  });

  it("serves stale totals when refresh fails after a warm cache", async () => {
    const gz = gzipJsonl(['{"oracle_id":"o1","set":"soc"}']);
    let fail = false;
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      if (fail) return new Response("nope", { status: 503 });
      const url = String(input);
      if (url.includes("/bulk-data/default-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });
    await refreshSetOracleTotals(fetchImpl as unknown as typeof fetch);
    vi.setSystemTime(new Date("2026-07-28T00:00:00Z"));
    fail = true;

    await expect(refreshSetOracleTotals(fetchImpl as unknown as typeof fetch)).resolves.toEqual({ soc: 1 });
  });

  it("ensureSetOracleTotalsRefresh does not block and populates cache", async () => {
    const gz = gzipJsonl(['{"oracle_id":"o1","set":"soc"}', '{"oracle_id":"o2","set":"soc"}']);
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/default-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });

    ensureSetOracleTotalsRefresh(fetchImpl as unknown as typeof fetch);

    expect(getCachedSetOracleTotals()).toBeNull();
    await __inflightSetOracleTotalsForTests();
    expect(getCachedSetOracleTotals()).toEqual({ soc: 2 });
  });
});
