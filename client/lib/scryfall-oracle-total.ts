import { gunzipSync } from "node:zlib";

const TTL_MS = 24 * 60 * 60 * 1000;
const UA = "edh.reilley.dev/0.1";
const BULK_URL = "https://api.scryfall.com/bulk-data/oracle-cards";

type Cache = { value: number; bySet: Readonly<Record<string, number>>; fetchedAt: number };
let cache: Cache | null = null;
let inflight: Promise<number | null> | null = null;

export function getCachedOracleTotal(): number | null {
  return cache?.value ?? null;
}

export function getCachedOracleTotalBySet(): Readonly<Record<string, number>> | null {
  return cache?.bySet ?? null;
}

export function __resetOracleTotalCacheForTests(): void {
  cache = null;
  inflight = null;
}

export function __inflightOracleTotalForTests(): Promise<number | null> | null {
  return inflight;
}

function cacheIsFresh(now: number): boolean {
  return cache != null && now - cache.fetchedAt < TTL_MS;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseOracleCounts(text: string): { value: number; bySet: Record<string, number> } {
  const bySet: Record<string, number> = {};
  let value = 0;

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    if (!line) continue;

    const parsed = JSON.parse(line) as unknown;
    value += 1;

    if (!isRecord(parsed) || typeof parsed.set !== "string") continue;
    bySet[parsed.set] = (bySet[parsed.set] ?? 0) + 1;
  }

  return { value, bySet };
}

export async function refreshOracleTotal(fetchImpl: typeof fetch = globalThis.fetch): Promise<number | null> {
  try {
    const metaRes = await fetchImpl(BULK_URL, {
      headers: { Accept: "application/json", "User-Agent": UA },
    });
    if (!metaRes.ok) return cache?.value ?? null;
    const meta = (await metaRes.json()) as { jsonl_download_uri?: string };
    if (!meta.jsonl_download_uri) return cache?.value ?? null;
    const fileRes = await fetchImpl(meta.jsonl_download_uri, {
      headers: { "User-Agent": UA },
    });
    if (!fileRes.ok) return cache?.value ?? null;
    const buf = Buffer.from(await fileRes.arrayBuffer());
    const text = gunzipSync(buf).toString("utf8");
    const { value, bySet } = parseOracleCounts(text);
    if (value <= 0) return cache?.value ?? null;
    cache = { value, bySet, fetchedAt: Date.now() };
    return value;
  } catch {
    return cache?.value ?? null;
  }
}

export function ensureOracleTotalRefresh(fetchImpl: typeof fetch = globalThis.fetch): void {
  const now = Date.now();
  if (cacheIsFresh(now) || inflight) return;
  inflight = refreshOracleTotal(fetchImpl).finally(() => {
    inflight = null;
  });
}
