import { defineHandler } from "nitro/h3";
import { json, runMetaGet } from "../../../../lobby-http";

export default defineHandler(async (event) =>
  runMetaGet(event, "api meta/health/v1", async () => json({ ok: true })),
);
