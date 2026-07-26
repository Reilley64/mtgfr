import { defineHandler } from "nitro/h3";
import { deleteTableRoute, loadLobby } from "../../../../../../app/domain/lobby-store";
import { tableParam, withLobbyAuth } from "../../../../../lobby-http";

export default defineHandler(async (event) => {
  const tableId = tableParam(event);
  if (!tableId) return new Response("Not Found", { status: 404 });

  return withLobbyAuth(event, `api tables/${tableId}/route/v1`, async ({ me, db }) => {
    const snap = await loadLobby(db, tableId);
    if (snap && !snap.seats.some((seat) => seat.userId === me.id) && snap.hostUserId !== me.id) {
      return new Response("Forbidden", { status: 403 });
    }
    await deleteTableRoute(db, tableId);
    return new Response(null, { status: 204 });
  });
});
