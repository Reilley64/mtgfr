import { defineHandler } from "nitro/h3";
import { seedGame } from "../../../../../../app/domain/api-upstream-auth";
import { commitStart, loadLobby, startError, toLobbyView } from "../../../../../../app/domain/lobby-store";
import { json, readJsonObject, tableParam, unknownLobby, withLobbyAuth } from "../../../../../lobby-http";

export default defineHandler(async (event) => {
  const tableId = tableParam(event);
  if (!tableId) return new Response("Not Found", { status: 404 });

  return withLobbyAuth(event, `api tables/${tableId}/start/v1`, async ({ me, env, db }) => {
    const body = await readJsonObject(event);
    if (!body) return json({ error: "BadJson" }, 400);

    const snap = await loadLobby(db, tableId);
    if (!snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    if (snap.startedAt) {
      return json(toLobbyView(snap, me.id));
    }

    const err = startError(snap, me.id);
    if (err) return json(toLobbyView(snap, me.id, err));

    const seeded = await seedGame(env, {
      table_id: tableId,
      host_user_id: snap.hostUserId,
      seats: snap.seats
        .slice()
        .sort((a, b) => a.seat - b.seat)
        .map((seat) => ({
          user_id: seat.userId,
          username: seat.username,
          gravatar_hash: seat.gravatarHash ?? "",
          deck_id: seat.deckId,
        })),
    });
    if (!seeded.ok) {
      return json(toLobbyView(snap, me.id, seeded.status === 503 ? "Draining" : "SeedFailed"));
    }

    try {
      await commitStart(db, tableId, seeded.data.pod_dns);
    } catch {
      return json(toLobbyView(snap, me.id, "SeedFailed"));
    }

    const started = await loadLobby(db, tableId);
    if (!started) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    return json(toLobbyView(started, me.id));
  });
});
