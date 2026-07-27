import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { fetchApiMeta } from "../../../../../app/domain/api-upstream-auth";
import { json, runMetaGet } from "../../../../lobby-http";

export default defineHandler(async (event) => {
  return runMetaGet(event, "api meta/version/v1", () =>
    Effect.gen(function* () {
      const meta = yield* Effect.promise(() => fetchApiMeta());
      const body: Record<string, unknown> = {
        version: meta.version ?? "unknown",
      };
      if (meta.faithfulCount != null) body.faithful_count = meta.faithfulCount;
      if (meta.oracleTotal != null) body.oracle_total = meta.oracleTotal;
      return json(body);
    }),
  );
});
