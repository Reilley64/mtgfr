import { defineHandler } from "nitro/h3";
import { createLobby } from "../../../../app/domain/lobby-store";
import { json, withLobbyAuth } from "../../../lobby-http";

export default defineHandler(async (event) =>
  withLobbyAuth(event, "api tables/v1", async ({ me, db }) => json({ table_id: await createLobby(db, me.id) })),
);
