const TTL_MS = 24 * 60 * 60 * 1000;
const UA = "edh.reilley.dev/0.1";
const BULK_URL = "https://api.scryfall.com/bulk-data/oracle-cards";

type Cache = { value: number; fetchedAt: number };
let cache: Cache | null = null;
let inflight: Promise<number | null> | null = null;

export function getCachedOracleTotal(): number | null {
  return cache?.value ?? null;
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

function countNonEmptyLines(text: string): number {
  let value = 0;
  for (const rawLine of text.split("\n")) {
    if (rawLine.trim()) value += 1;
  }
  return value;
}

/** Stream-count gzip JSONL so we never gunzipSync a ~200MB buffer on the Nitro event loop. */
async function countOracleTotalFromGzipStream(body: ReadableStream<BufferSource>): Promise<number> {
  const decoder = new TextDecoder();
  let carry = "";
  let total = 0;

  const reader = body.pipeThrough(new DecompressionStream("gzip")).getReader();
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    if (value == null) continue;

    const text = carry + decoder.decode(value, { stream: true });
    const lines = text.split("\n");
    carry = lines.pop() ?? "";
    for (const line of lines) {
      if (line.trim()) total += 1;
    }
  }

  total += countNonEmptyLines(carry + decoder.decode());
  return total;
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
    if (!fileRes.ok || fileRes.body == null) return cache?.value ?? null;
    const value = await countOracleTotalFromGzipStream(fileRes.body);
    if (value <= 0) return cache?.value ?? null;
    cache = { value, fetchedAt: Date.now() };
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

/** Await a cold fill; when warm, return cache and kick SWR in the background. */
export async function loadOracleTotal(fetchImpl: typeof fetch = globalThis.fetch): Promise<number | null> {
  const warm = getCachedOracleTotal();
  if (warm != null) {
    ensureOracleTotalRefresh(fetchImpl);
    return warm;
  }
  if (inflight) return inflight;
  return refreshOracleTotal(fetchImpl);
}
