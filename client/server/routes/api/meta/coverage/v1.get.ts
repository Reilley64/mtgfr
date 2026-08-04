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
      return cached(
        json({
          faithful_count: meta.faithfulCount,
          oracle_total: meta.oracleTotal,
          sets: meta.sets.map((set) => ({
            code: set.code,
            name: set.name,
            released_at: set.releasedAt,
            faithful: set.faithful,
            oracle_total: set.oracleTotal,
          })),
        }),
        CACHE_CONTROL,
      );
    }),
  );
});
