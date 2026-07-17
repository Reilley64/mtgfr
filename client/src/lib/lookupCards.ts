// Chunked `/cards/lookup/v1` — repeated `ids` query params blow past URL limits on long decklists.

import * as Effect from "effect/Effect";
import { client } from "~/effect/client";
import type { CatalogCard } from "~/wire/types";

export const LOOKUP_CHUNK = 40;

export function lookupCardsByIds(ids: readonly string[]): Effect.Effect<CatalogCard[], unknown> {
  const unique = [...new Set(ids.filter(Boolean))];
  if (unique.length === 0) return Effect.succeed([]);
  return Effect.gen(function* () {
    const out: CatalogCard[] = [];
    for (let i = 0; i < unique.length; i += LOOKUP_CHUNK) {
      const page = yield* client.lookupCards(unique.slice(i, i + LOOKUP_CHUNK));
      out.push(...page);
    }
    return out;
  });
}
