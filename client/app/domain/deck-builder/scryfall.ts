import { Schema as S } from "effect";
import * as Duration from "effect/Duration";
import * as Effect from "effect/Effect";

export type ImageSize = "small" | "normal" | "large" | "png" | "art_crop";
export type ImageFace = "front" | "back";

const CDN = String(import.meta.env.VITE_CARD_CDN ?? "").replace(/\/$/, "");
const SCRYFALL_UA = "edh.reilley.dev/0.1";
/** Scryfall locks an IP for ~30s after a 429 when Retry-After is absent. */
const DEFAULT_RETRY_AFTER_MS = 30_000;
const MAX_RETRY_AFTER_MS = 60_000;
/** Initial attempt + this many retries after 429. */
const MAX_429_RETRIES = 2;

export function cardBackUrl(): string {
  return "/card-back.webp";
}

export function buildImageUrl(printId: string, size: ImageSize, face: ImageFace, cdnBase: string): string {
  if (!printId) return "";
  const base = cdnBase.replace(/\/$/, "");
  if (base) {
    const a = printId[0];
    const b = printId[1];
    const folder = size === "art_crop" ? "art_crop" : "large";
    return `${base}/${folder}/${face}/${a}/${b}/${printId}.jpg`;
  }
  const faceParam = face === "back" ? "&face=back" : "";
  return `https://api.scryfall.com/cards/${printId}?format=image&version=${size}${faceParam}`;
}

export function scryfallImageUrl(printId: string, size: ImageSize, face: ImageFace = "front"): string {
  return buildImageUrl(printId, size, face, "");
}

export function imageUrlByPrint(printId: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  return buildImageUrl(printId, size, face, CDN);
}

export const ScryfallPrintSchema = S.Struct({
  collector_number: S.String,
  id: S.String,
  released_at: S.String,
  set: S.String,
  set_name: S.String,
});
export type ScryfallPrint = typeof ScryfallPrintSchema.Type;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" ? value : null;
}

/** Parse Scryfall/HTTP `Retry-After` into a clamped delay in milliseconds. */
export function parseRetryAfterMs(header: string | null, nowMs: number = Date.now()): number {
  if (header == null || header.trim() === "") return DEFAULT_RETRY_AFTER_MS;

  const asSeconds = Number(header);
  if (Number.isFinite(asSeconds) && asSeconds >= 0) {
    return Math.min(Math.ceil(asSeconds * 1000), MAX_RETRY_AFTER_MS);
  }

  const when = Date.parse(header);
  if (Number.isFinite(when)) {
    return Math.min(Math.max(0, when - nowMs), MAX_RETRY_AFTER_MS);
  }

  return DEFAULT_RETRY_AFTER_MS;
}

function fetchPrintSearchPage(url: string): Effect.Effect<Response, Error> {
  return Effect.gen(function* () {
    let retries = 0;
    while (true) {
      const res = yield* Effect.tryPromise({
        try: () =>
          fetch(url, {
            headers: { Accept: "application/json", "User-Agent": SCRYFALL_UA },
          }),
        catch: (cause) => (cause instanceof Error ? cause : new Error(String(cause))),
      });

      if (res.status !== 429) return res;
      if (retries >= MAX_429_RETRIES) {
        return yield* Effect.fail(new Error(`Scryfall print search failed (${res.status})`));
      }

      retries += 1;
      const delayMs = parseRetryAfterMs(res.headers.get("Retry-After"));
      yield* Effect.sleep(Duration.millis(delayMs));
    }
  });
}

/** One page of printings, plus where the next one lives. */
export type PrintPage = { readonly prints: ScryfallPrint[]; readonly nextPage: string | null };

/** Where a card's printings start, oldest release first. */
export function printSearchUrl(oracleId: string): string {
  const q = encodeURIComponent(`oracleid:${oracleId}`);
  return `https://api.scryfall.com/cards/search?q=${q}&unique=prints&order=released`;
}

/** Fetches a single Scryfall search page — 175 printings at most. Callers walk `nextPage`
 *  themselves so each page can be shown as it lands instead of after the last one. */
export function searchPrintPage(url: string): Effect.Effect<PrintPage, Error> {
  return Effect.gen(function* () {
    const res = yield* fetchPrintSearchPage(url);
    if (!res.ok) {
      return yield* Effect.fail(new Error(`Scryfall print search failed (${res.status})`));
    }
    const body: unknown = yield* Effect.tryPromise({
      try: () => res.json(),
      catch: (cause) => (cause instanceof Error ? cause : new Error(String(cause))),
    });
    if (!isRecord(body)) return { prints: [], nextPage: null };

    const prints: ScryfallPrint[] = [];
    const data = Array.isArray(body.data) ? body.data : [];
    for (const value of data) {
      if (!isRecord(value)) continue;
      const id = readString(value, "id");
      const set = readString(value, "set");
      const setName = readString(value, "set_name");
      const collectorNumber = readString(value, "collector_number");
      if (id == null || set == null || setName == null || collectorNumber == null) continue;

      prints.push({
        collector_number: collectorNumber,
        id,
        released_at: readString(value, "released_at") ?? "",
        set,
        set_name: setName,
      });
    }
    return { prints, nextPage: body.has_more === true ? readString(body, "next_page") : null };
  });
}
