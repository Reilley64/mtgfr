const TTL_MS = 24 * 60 * 60 * 1000;
const UA = "edh.reilley.dev/0.1";
const BULK_URL = "https://api.scryfall.com/bulk-data/default-cards";

type SetOracleTotals = Readonly<Record<string, number>>;
type Cache = { value: SetOracleTotals; fetchedAt: number };

let cache: Cache | null = null;
let inflight: Promise<SetOracleTotals | null> | null = null;

export function getCachedSetOracleTotals(): SetOracleTotals | null {
  return cache?.value ?? null;
}

export function __resetSetOracleTotalsCacheForTests(): void {
  cache = null;
  inflight = null;
}

export function __inflightSetOracleTotalsForTests(): Promise<SetOracleTotals | null> | null {
  return inflight;
}

function cacheIsFresh(now: number): boolean {
  return cache != null && now - cache.fetchedAt < TTL_MS;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" ? value : null;
}

function indexSetOracleLine(bySet: Map<string, Set<string>>, rawLine: string): void {
  const line = rawLine.trim();
  if (!line) return;

  let parsed: unknown;
  try {
    parsed = JSON.parse(line);
  } catch {
    return;
  }

  if (!isRecord(parsed)) return;

  const oracleId = readString(parsed, "oracle_id");
  const rawSet = readString(parsed, "set");
  if (oracleId == null || oracleId.length === 0) return;
  if (rawSet == null || rawSet.length === 0) return;

  const setCode = rawSet.toLowerCase();
  const oracleIds = bySet.get(setCode) ?? new Set<string>();
  oracleIds.add(oracleId);
  bySet.set(setCode, oracleIds);
}

function materializeSetOracleTotals(bySet: Map<string, Set<string>>): Record<string, number> {
  const totals: Record<string, number> = {};
  for (const setCode of [...bySet.keys()].sort()) {
    totals[setCode] = bySet.get(setCode)?.size ?? 0;
  }
  return totals;
}

export function parseSetOracleTotals(text: string): Record<string, number> {
  const bySet = new Map<string, Set<string>>();
  for (const rawLine of text.split("\n")) {
    indexSetOracleLine(bySet, rawLine);
  }
  return materializeSetOracleTotals(bySet);
}

async function parseSetOracleTotalsFromGzipStream(body: ReadableStream<BufferSource>): Promise<Record<string, number>> {
  const bySet = new Map<string, Set<string>>();
  const decoder = new TextDecoder();
  let carry = "";

  const reader = body.pipeThrough(new DecompressionStream("gzip")).getReader();
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    const text = carry + decoder.decode(value, { stream: true });
    const lines = text.split("\n");
    carry = lines.pop() ?? "";
    for (const line of lines) {
      indexSetOracleLine(bySet, line);
    }
  }

  indexSetOracleLine(bySet, carry + decoder.decode());
  return materializeSetOracleTotals(bySet);
}

export async function refreshSetOracleTotals(
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<SetOracleTotals | null> {
  try {
    const metaRes = await fetchImpl(BULK_URL, {
      headers: { Accept: "application/json", "User-Agent": UA },
    });
    if (!metaRes.ok) return cache?.value ?? null;

    const meta: unknown = await metaRes.json();
    if (!isRecord(meta)) return cache?.value ?? null;

    const downloadUri = readString(meta, "jsonl_download_uri");
    if (downloadUri == null || downloadUri.length === 0) return cache?.value ?? null;

    const fileRes = await fetchImpl(downloadUri, {
      headers: { "User-Agent": UA },
    });
    if (!fileRes.ok || fileRes.body == null) return cache?.value ?? null;

    const value = await parseSetOracleTotalsFromGzipStream(fileRes.body);
    if (Object.keys(value).length === 0) return cache?.value ?? null;

    cache = { value, fetchedAt: Date.now() };
    return value;
  } catch {
    return cache?.value ?? null;
  }
}

export function ensureSetOracleTotalsRefresh(fetchImpl: typeof fetch = globalThis.fetch): void {
  const now = Date.now();
  if (cacheIsFresh(now) || inflight) return;

  inflight = refreshSetOracleTotals(fetchImpl).finally(() => {
    inflight = null;
  });
}

/** Await a cold fill; when warm, return cache and kick SWR in the background. */
export async function loadSetOracleTotals(fetchImpl: typeof fetch = globalThis.fetch): Promise<SetOracleTotals | null> {
  const warm = getCachedSetOracleTotals();
  if (warm != null) {
    ensureSetOracleTotalsRefresh(fetchImpl);
    return warm;
  }
  if (inflight) return inflight;
  return refreshSetOracleTotals(fetchImpl);
}
