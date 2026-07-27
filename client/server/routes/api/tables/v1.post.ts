import * as Effect from "effect/Effect";
import { defineHandler } from "nitro/h3";
import { createLobby } from "../../../../app/domain/lobby-store";
import { json, withLobbyAuth } from "../../../lobby-http";

export default defineHandler(async (event) =>
  withLobbyAuth(event, "api tables/v1", ({ me }) =>
    Effect.gen(function* () {
      return json({ table_id: yield* createLobby(me.id) });
    }),
  ),
);
