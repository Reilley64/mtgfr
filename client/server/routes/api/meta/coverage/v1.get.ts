import { defineHandler } from "nitro/h3";
import { fetchCoverageMeta } from "../../../../../app/domain/coverage-meta";
import { json, runMetaGet } from "../../../../lobby-http";

export default defineHandler(async (event) => {
  return runMetaGet(event, "api meta/coverage/v1", async () => {
    const meta = await fetchCoverageMeta();
    return json({
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
  });
});
