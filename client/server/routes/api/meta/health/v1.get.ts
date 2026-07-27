import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { json, runMetaGet } from "../../../../lobby-http";

export default defineHandler(async (event) =>
  runMetaGet(event, "api meta/health/v1", () => Effect.succeed(json({ ok: true }))),
);
