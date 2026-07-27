import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { fetchDeckName } from "../../../../../../app/domain/api-upstream-auth";
import { gravatarHash } from "../../../../../../app/domain/gravatar";
import { joinLobby, loadLobby, toLobbyView } from "../../../../../../app/domain/lobby-store";
import { json, readJsonObject, tableParam, unknownLobby, withLobbyAuth } from "../../../../../lobby-http";

export default defineHandler(async (event) => {
  const tableId = tableParam(event);
  if (!tableId) return new Response("Not Found", { status: 404 });

  return withLobbyAuth(event, `api tables/${tableId}/join/v1`, ({ me, env }) =>
    Effect.gen(function* () {
      const body = yield* Effect.promise(() => readJsonObject(event));
      if (!body) return json({ error: "BadJson" }, 400);

      const deckId = Number(body.deck_id);
      const deckName = yield* fetchDeckName(env, deckId);
      if (!deckName) {
        const snap = yield* loadLobby(tableId);
        if (!snap) {
          return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
        }
        return json(toLobbyView(snap, me.id, "UnknownDeck"));
      }

      const result = yield* joinLobby({
        tableId,
        userId: me.id,
        username: me.username,
        gravatarHash: yield* Effect.promise(() => gravatarHash(me.email)),
        deckId,
        deckName,
      });
      if (!result.snap) {
        return json(toLobbyView(unknownLobby(tableId), me.id, result.error), 404);
      }
      return json(toLobbyView(result.snap, me.id, result.error));
    }),
  );
});
