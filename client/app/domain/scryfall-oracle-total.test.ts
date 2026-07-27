import { gzipSync } from "node:zlib";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  __inflightOracleTotalForTests,
  __resetOracleTotalCacheForTests,
  ensureOracleTotalRefresh,
  getCachedOracleTotal,
  refreshOracleTotal,
} from "./scryfall-oracle-total";

function gzipJsonl(lines: string[]): Uint8Array {
  return Uint8Array.from(gzipSync(Buffer.from(`${lines.join("\n")}\n`, "utf8")));
}

describe("scryfall oracle total cache", () => {
  beforeEach(() => {
    __resetOracleTotalCacheForTests();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-26T00:00:00Z"));
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("returns null before any successful refresh", () => {
    expect(getCachedOracleTotal()).toBeNull();
  });

  it("counts oracle bulk rows for the global total", async () => {
    const gz = gzipJsonl(['{"id":"a","set":"soc"}', '{"id":"b","set":"soc"}', '{"id":"c","set":"cmd"}', ""]);
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/oracle-cards")) {
        return new Response(
          JSON.stringify({
            jsonl_download_uri: "https://data.scryfall.io/oracle-cards/test.jsonl.gz",
          }),
          { status: 200 },
        );
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });
    const total = await refreshOracleTotal(fetchImpl as unknown as typeof fetch);
    expect(total).toBe(3);
    expect(getCachedOracleTotal()).toBe(3);
    expect(fetchImpl.mock.calls[0]?.[0]).toEqual(expect.stringContaining("bulk-data/oracle-cards"));
    const init = fetchImpl.mock.calls[0]?.[1] as RequestInit | undefined;
    const headers = init?.headers as Record<string, string>;
    expect(headers["User-Agent"]).toBe("edh.reilley.dev/0.1");
  });

  it("skips blank lines when counting the global total", async () => {
    const gz = gzipJsonl(['{"id":"a","set":"soc"}', '{"id":"b","set":7}', '{"id":"c"}', ""]);
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/oracle-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });
    const total = await refreshOracleTotal(fetchImpl as unknown as typeof fetch);
    expect(total).toBe(3);
    expect(getCachedOracleTotal()).toBe(3);
  });

  it("serves stale value when refresh fails after a warm cache", async () => {
    const gz = gzipJsonl(['{"id":"a"}']);
    let fail = false;
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      if (fail) return new Response("nope", { status: 503 });
      const url = String(input);
      if (url.includes("/bulk-data/oracle-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });
    await refreshOracleTotal(fetchImpl as unknown as typeof fetch);
    vi.setSystemTime(new Date("2026-07-28T00:00:00Z")); // past TTL
    fail = true;
    const total = await refreshOracleTotal(fetchImpl as unknown as typeof fetch);
    expect(total).toBe(1);
    expect(getCachedOracleTotal()).toBe(1);
  });

  it("ensureOracleTotalRefresh does not block and populates cache", async () => {
    const gz = gzipJsonl(['{"id":"a"}', '{"id":"b"}', '{"id":"c"}']);
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/oracle-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      return new Response(gz as unknown as BodyInit, { status: 200 });
    });
    ensureOracleTotalRefresh(fetchImpl as unknown as typeof fetch);
    expect(getCachedOracleTotal()).toBeNull();
    await __inflightOracleTotalForTests();
    expect(getCachedOracleTotal()).toBe(3);
  });

  it("streams the gzip body instead of buffering via arrayBuffer + gunzipSync", async () => {
    const gz = gzipJsonl(['{"id":"a"}', '{"id":"b"}']);
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(gz);
        controller.close();
      },
    });
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
      const url = String(input);
      if (url.includes("/bulk-data/oracle-cards")) {
        return new Response(JSON.stringify({ jsonl_download_uri: "https://data.scryfall.io/x.jsonl.gz" }), {
          status: 200,
        });
      }
      const res = new Response(stream, { status: 200 });
      res.arrayBuffer = async () => {
        throw new Error("arrayBuffer must not be used for oracle bulk (blocks the Nitro event loop)");
      };
      return res;
    });

    const total = await refreshOracleTotal(fetchImpl as unknown as typeof fetch);
    expect(total).toBe(2);
    expect(getCachedOracleTotal()).toBe(2);
  });
});
