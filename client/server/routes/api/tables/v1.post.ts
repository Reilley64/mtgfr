import { defineHandler } from "nitro/h3";
import { createLobby } from "../../../../app/domain/lobby-store";
import { runWebDb } from "../../../db/client";
import { json, withLobbyAuth } from "../../../lobby-http";

export default defineHandler(async (event) =>
  withLobbyAuth(event, "api tables/v1", async ({ me }) => json({ table_id: await runWebDb(createLobby(me.id)) })),
);
