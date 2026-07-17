// The Effect Rpc-ish surface shared by the browser client (`~/effect/client`) and the BFF's
// dispatcher (`~/wire/rpcServer`) — ADR 0032's "thin same-origin Effect HTTP API at `/api/rpc`"
// fallback. A full `RpcGroup`/`RpcServer` pair needs an `HttpServerRequest` bridge SolidStart's
// Vinxi routes don't give us for free; this module is the method map that keeps the two sides
// from drifting instead — one name, one URL, one direction, on both ends.

/** `/api/rpc/<group>/...` — the top-level dispatch key. */
export type RpcGroup = "auth" | "cards" | "decks" | "game";

export function isRpcGroup(value: string | undefined): value is RpcGroup {
  return value === "auth" || value === "cards" || value === "decks" || value === "game";
}

export type AuthMethod = "signup" | "login" | "logout" | "me";
export function isAuthMethod(value: string | undefined): value is AuthMethod {
  return value === "signup" || value === "login" || value === "logout" || value === "me";
}

export type CardsMethod = "catalog" | "search" | "lookup";
export function isCardsMethod(value: string | undefined): value is CardsMethod {
  return value === "catalog" || value === "search" || value === "lookup";
}

/** `/api/rpc/game/:table/<method>` — every game call is scoped to a table. */
export type GameMethod = "intent" | "yield" | "turn-yield" | "stack-dwell" | "stream";
export function isGameMethod(value: string | undefined): value is GameMethod {
  return value === "intent" || value === "yield" || value === "turn-yield" || value === "stack-dwell" || value === "stream";
}
