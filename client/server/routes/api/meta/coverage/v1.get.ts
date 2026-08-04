import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { fetchCoverageMeta } from "../../../../../app/domain/coverage-meta";
import { cached, json, runMetaGet } from "../../../../lobby-http";

// `s-maxage` is the edge TTL (purged on deploy); `max-age` is the browser copy, which a purge
// cannot reach, so keep it short.
const CACHE_CONTROL = "public, max-age=60, s-maxage=3600, stale-while-revalidate=600";

export default defineHandler(async (event) => {
  return runMetaGet(event, "api meta/coverage/v1", () =>
    Effect.gen(function* () {
      const meta = yield* Effect.promise(() => fetchCoverageMeta());
      const body = json({
        faithful_count: meta.faithfulCount,
        oracle_total: meta.oracleTotal,
        sets: meta.sets.map((set) => ({
          code: set.code,
          name: set.name,
          released_at: set.releasedAt,
          faithful: set.faithful,
          oracle_total: set.oracleTotal,
        })),
      });

      // `fetchCoverageMeta` degrades to a 200 with null counts (API rolling, Scryfall cache cold),
      // and every set then reports 0 faithful. A deploy purges the good copy moments before the API
      // goes unready, so caching that would pin a zeroed page at the edge until the *next* deploy.
      if (meta.faithfulCount == null || meta.oracleTotal == null || meta.sets.length === 0) {
        return body;
      }

      return cached(body, CACHE_CONTROL);
    }),
  );
});
