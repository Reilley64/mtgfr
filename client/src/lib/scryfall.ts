// Card imagery from an optional self-hosted CDN (bake `VITE_CARD_CDN`), which mirrors Scryfall's
// image folder path (`<size>/front/<a>/<b>/<id>.ext`) but serves only `large` and converts
// everything to webp. Keyed by Scryfall *Printing* UUID (ADR 0031). Missing art is a broken
// image — no Scryfall image-host fallback. Without a CDN, resolve the same UUID via Scryfall's
// card-id image endpoint so localdev still works.

export type ImageSize = "small" | "normal" | "large" | "png" | "art_crop";
export type ImageFace = "front" | "back";

const CDN = String(import.meta.env.VITE_CARD_CDN ?? "").replace(/\/$/, "");

/** Classic Magic card back — library piles and any face-down card on the board. */
export function cardBackUrl(): string {
  return "/card-back.webp";
}

// The CDN path for a Scryfall Printing UUID: first two hex chars fan out the folder tree. Only
// `large` webp exists, so the requested size is ignored when CDN is set.
function cdnUrl(printId: string, face: ImageFace): string {
  const a = printId[0];
  const b = printId[1];
  return `${CDN}/large/${face}/${a}/${b}/${printId}.webp`;
}

/** Art URL for a Printing UUID. Empty print → empty URL (broken `<img>`). `face` selects DFC side. */
export function imageUrlByPrint(printId: string, size: ImageSize = "large", face: ImageFace = "front"): string {
  if (!printId) return "";
  if (CDN) return cdnUrl(printId, face);
  const faceParam = face === "back" ? "&face=back" : "";
  return `https://api.scryfall.com/cards/${printId}?format=image&version=${size}${faceParam}`;
}

export type ScryfallPrint = {
  id: string;
  set: string;
  set_name: string;
  collector_number: string;
  released_at: string;
};

/** Scryfall prints for a Card (oracle) id — picker metadata only; images still use CDN. */
export async function searchPrints(oracleId: string): Promise<ScryfallPrint[]> {
  const q = encodeURIComponent(`oracleid:${oracleId}`);
  const out: ScryfallPrint[] = [];
  let url: string | null = `https://api.scryfall.com/cards/search?q=${q}&unique=prints&order=released`;
  while (url) {
    const res = await fetch(url, {
      headers: { Accept: "application/json", "User-Agent": "mtgfr/0.1" },
    });
    if (!res.ok) {
      throw new Error(`Scryfall print search failed (${res.status})`);
    }
    const body = (await res.json()) as {
      data?: Array<{
        id: string;
        set: string;
        set_name: string;
        collector_number: string;
        released_at?: string;
      }>;
      next_page?: string | null;
      has_more?: boolean;
    };
    for (const c of body.data ?? []) {
      out.push({
        id: c.id,
        set: c.set,
        set_name: c.set_name,
        collector_number: c.collector_number,
        released_at: c.released_at ?? "",
      });
    }
    url = body.has_more && body.next_page ? body.next_page : null;
  }
  return out;
}
