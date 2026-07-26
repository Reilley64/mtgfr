const TTL_MS = 24 * 60 * 60 * 1000;
const UA = "edh.reilley.dev/0.1";
const SETS_URL = "https://api.scryfall.com/sets";

export type ScryfallSetRow = {
  code: string;
  name: string;
  releasedAt: string | null;
  cardCount: number;
};

type Cache = { rows: ReadonlyArray<ScryfallSetRow>; fetchedAt: number };

let cache: Cache | null = null;
let inflight: Promise<ReadonlyArray<ScryfallSetRow> | null> | null = null;

export function getCachedScryfallSets(): ReadonlyArray<ScryfallSetRow> | null {
  return cache?.rows ?? null;
}

export function __resetScryfallSetsCacheForTests(): void {
  cache = null;
  inflight = null;
}

export function __inflightScryfallSetsForTests(): Promise<ReadonlyArray<ScryfallSetRow> | null> | null {
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

function readNullableString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return value == null ? null : typeof value === "string" ? value : null;
}

function readNumber(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  return typeof value === "number" ? value : null;
}

function parseScryfallSets(body: unknown): ReadonlyArray<ScryfallSetRow> | null {
  if (!isRecord(body) || !Array.isArray(body.data)) return null;

  const rows: ScryfallSetRow[] = [];
  for (const value of body.data) {
    if (!isRecord(value)) continue;

    const code = readString(value, "code");
    const name = readString(value, "name");
    const cardCount = readNumber(value, "card_count");
    const setType = readString(value, "set_type");
    if (code == null || name == null || cardCount == null) continue;
    if (cardCount <= 0) continue;
    // Not deckable pool targets (Art Series, tokens, minigames, vanguard/avatars).
    if (setType === "memorabilia" || setType === "token" || setType === "minigame" || setType === "vanguard") {
      continue;
    }

    rows.push({
      code,
      name,
      releasedAt: readNullableString(value, "released_at"),
      cardCount,
    });
  }

  return rows;
}

export async function refreshScryfallSets(
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<ReadonlyArray<ScryfallSetRow> | null> {
  try {
    const res = await fetchImpl(SETS_URL, {
      headers: { Accept: "application/json", "User-Agent": UA },
    });
    if (!res.ok) return cache?.rows ?? null;

    const rows = parseScryfallSets((await res.json()) as unknown);
    if (rows == null) return cache?.rows ?? null;
    if (rows.length === 0) return cache?.rows ?? null;

    cache = { rows, fetchedAt: Date.now() };
    return rows;
  } catch {
    return cache?.rows ?? null;
  }
}

export function ensureScryfallSetsRefresh(fetchImpl: typeof fetch = globalThis.fetch): void {
  const now = Date.now();
  if (cacheIsFresh(now) || inflight) return;

  inflight = refreshScryfallSets(fetchImpl).finally(() => {
    inflight = null;
  });
}

/** Await a cold fill; when warm, return cache and kick SWR in the background. */
export async function loadScryfallSets(
  fetchImpl: typeof fetch = globalThis.fetch,
): Promise<ReadonlyArray<ScryfallSetRow> | null> {
  const warm = getCachedScryfallSets();
  if (warm != null) {
    ensureScryfallSetsRefresh(fetchImpl);
    return warm;
  }
  if (inflight) return inflight;
  return refreshScryfallSets(fetchImpl);
}
