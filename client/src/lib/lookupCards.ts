// Batched `/cards/lookup/v1` — Card ids ride repeated query params; chunk so long decklists
// don't blow past URL length limits.

import * as Effect from "effect/Effect";
import type { CatalogCard } from "~/api/generated";
import { client } from "~/effect/client";

/** Max Card ids per lookup request (UUID × N stays comfortably under typical URL limits). */
export const LOOKUP_CHUNK = 40;

/** Look up catalog cards by Card id, chunking the query when needed. */
export function lookupCardsByIds(ids: readonly string[]): Effect.Effect<CatalogCard[], unknown> {
  const unique = [...new Set(ids.filter(Boolean))];
  if (unique.length === 0) return Effect.succeed([]);
  const chunks: string[][] = [];
  for (let i = 0; i < unique.length; i += LOOKUP_CHUNK) {
    chunks.push(unique.slice(i, i + LOOKUP_CHUNK));
  }
  return Effect.gen(function* () {
    const out: CatalogCard[] = [];
    for (const chunk of chunks) {
      const page = yield* client.lookupCards({ params: { ids: chunk } });
      out.push(...page);
    }
    return out;
  });
}
