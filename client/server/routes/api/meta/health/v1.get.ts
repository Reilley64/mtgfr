import { defineHandler } from "nitro/h3";
import { json } from "../../../../lobby-http";

export default defineHandler(async () => json({ ok: true }));
