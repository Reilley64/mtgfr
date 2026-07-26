import { defineHandler } from "nitro/h3";
import { setReady, toLobbyView } from "../../../../../../app/domain/lobby-store";
import { runWebDb } from "../../../../../db/client";
import { json, readJsonObject, tableParam, unknownLobby, withLobbyAuth } from "../../../../../lobby-http";

export default defineHandler(async (event) => {
  const tableId = tableParam(event);
  if (!tableId) return new Response("Not Found", { status: 404 });

  return withLobbyAuth(event, `api tables/${tableId}/ready/v1`, async ({ me }) => {
    const body = await readJsonObject(event);
    if (!body) return json({ error: "BadJson" }, 400);

    const result = await runWebDb(setReady(tableId, me.id, Boolean(body.ready)));
    if (!result.snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, result.error), 404);
    }
    return json(toLobbyView(result.snap, me.id, result.error));
  });
});
