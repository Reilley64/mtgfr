// Card imagery from an optional self-hosted CDN (bake `VITE_CARD_CDN`), which mirrors Scryfall's
// image folder path (`<size>/front/<a>/<b>/<id>.ext`) but serves only `large` and converts
// everything to webp. Keyed by Scryfall id, so `imageUrlByName` resolves the pool's names through
// the build-time `card-ids.json` map; a name not in the map (or when no CDN is configured) falls
// back to Scryfall's `named` endpoint. See ADR 0015.
//
// The classic Magic card back (library piles, face-down cards) is a static asset under
// `client/public/card-back.webp` — the CDN only mirrors front faces today.

import cardIds from "~/lib/card-ids.json";

export type ImageSize = "small" | "normal" | "large" | "png" | "art_crop";

const CDN = String(import.meta.env.VITE_CARD_CDN ?? "").replace(/\/$/, "");
const NAME_TO_ID = cardIds as Record<string, string>;

/** Classic Magic card back — library piles and any face-down card on the board. */
export function cardBackUrl(): string {
  return "/card-back.webp";
}

// The CDN path for a Scryfall id: first two hex chars of the id fan out the folder tree. Only
// `large` webp exists, so the requested size is ignored.
function cdnUrl(scryfallId: string): string {
  const a = scryfallId[0];
  const b = scryfallId[1];
  return `${CDN}/large/front/${a}/${b}/${scryfallId}.webp`;
}

// Server snapshots carry a card's name, not its Scryfall id, so we resolve art by name through
// the build-time `card-ids.json` map and serve it from the CDN when configured. Otherwise (and
// for unmapped names) fall back to Scryfall's `named` image endpoint (a 302 to their CDN); `fuzzy`
// so simplified/placeholder names still match (e.g. "Grizzly Bear" → "Grizzly Bears").
export function imageUrlByName(name: string, size: ImageSize): string {
  const id = NAME_TO_ID[name];
  if (id && CDN) return cdnUrl(id);
  return `https://api.scryfall.com/cards/named?fuzzy=${encodeURIComponent(name)}&format=image&version=${size}`;
}
