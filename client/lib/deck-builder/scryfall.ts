import { Schema as S } from "effect";

export type ImageSize = "small" | "normal" | "large" | "png" | "art_crop";
export type ImageFace = "front" | "back";

const CDN = String(import.meta.env.VITE_CARD_CDN ?? "").replace(/\/$/, "");

export function cardBackUrl(): string {
  return "/card-back.webp";
}

function cdnUrl(printId: string, face: ImageFace): string {
  const a = printId[0];
  const b = printId[1];
  return `${CDN}/large/${face}/${a}/${b}/${printId}.webp`;
}

export function imageUrlByPrint(printId: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  if (!printId) return "";
  if (CDN) return cdnUrl(printId, face);
  const faceParam = face === "back" ? "&face=back" : "";
  return `https://api.scryfall.com/cards/${printId}?format=image&version=${size}${faceParam}`;
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

export async function searchPrints(oracleId: string): Promise<ScryfallPrint[]> {
  const q = encodeURIComponent(`oracleid:${oracleId}`);
  const out: ScryfallPrint[] = [];
  let url: string | null = `https://api.scryfall.com/cards/search?q=${q}&unique=prints&order=released`;
  while (url) {
    const res: Response = await fetch(url, {
      headers: { Accept: "application/json", "User-Agent": "mtgfr/0.1" },
    });
    if (!res.ok) {
      throw new Error(`Scryfall print search failed (${res.status})`);
    }
    const body: unknown = await res.json();
    if (!isRecord(body)) return out;

    const data = Array.isArray(body.data) ? body.data : [];
    for (const value of data) {
      if (!isRecord(value)) continue;
      const id = readString(value, "id");
      const set = readString(value, "set");
      const setName = readString(value, "set_name");
      const collectorNumber = readString(value, "collector_number");
      if (id == null || set == null || setName == null || collectorNumber == null) continue;

      out.push({
        collector_number: collectorNumber,
        id,
        released_at: readString(value, "released_at") ?? "",
        set,
        set_name: setName,
      });
    }
    url = body.has_more === true ? readString(body, "next_page") : null;
  }
  return out;
}
