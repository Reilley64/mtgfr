import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { loadLobby, toLobbyView } from "../../../../../../app/domain/lobby-store";
import { json, tableParam, unknownLobby, withLobbyAuth } from "../../../../../lobby-http";

export default defineHandler(async (event) => {
  const tableId = tableParam(event);
  if (!tableId) return new Response("Not Found", { status: 404 });

  return withLobbyAuth(event, `api tables/${tableId}/lobby/v1`, ({ me }) =>
    Effect.gen(function* () {
      const snap = yield* loadLobby(tableId);
      if (!snap) {
        return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
      }
      return json(toLobbyView(snap, me.id));
    }),
  );
});
