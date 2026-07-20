// Warm the shared image cache from deck lists (prints are not secret — only zone membership is).

import * as Effect from "effect/Effect";
import { client, orNull } from "~/effect/client";
import type { ImageCache } from "~/lib/imageCache";
import { type ImageSize, imageUrlByPrint } from "~/lib/scryfall";
import type { DeckDetail } from "~/wire/types";

/** Unique Scryfall print UUIDs in a deck (commander + mainboard). */
export function printIdsFromDeck(deck: DeckDetail): string[] {
  const out = new Set<string>();
  if (deck.commander_print) out.add(deck.commander_print);
  for (const card of deck.cards) {
    if (card.print) out.add(card.print);
  }
  return [...out];
}

export function imageUrlsForPrints(prints: Iterable<string>, size: ImageSize = "large"): string[] {
  const urls: string[] = [];
  for (const print of prints) {
    const url = imageUrlByPrint(print, size);
    if (url) urls.push(url);
  }
  return urls;
}

/**
 * Fetch each deck (owned decks + public precons) and preload their art into `cache`.
 * Failures for individual decks are ignored — opponents' custom decks are ownership-gated.
 */
export function preloadDecksIntoCache(deckIds: Iterable<number>, cache: ImageCache): Effect.Effect<void> {
  const unique = [...new Set(deckIds)];
  if (unique.length === 0) return Effect.void;

  return Effect.gen(function* () {
    const prints = new Set<string>();
    for (const id of unique) {
      const deck = yield* orNull(client.getDeck(String(id)));
      if (!deck) continue;
      for (const print of printIdsFromDeck(deck)) prints.add(print);
    }
    cache.preload(imageUrlsForPrints(prints));
  });
}
