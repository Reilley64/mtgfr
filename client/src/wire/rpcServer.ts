// `/api/rpc` dispatcher: request shape in, outcome out — unit-testable without a Vinxi route.

import * as Match from "effect/Match";
import { GrpcCallError, type GrpcRequestEnv, grpcClientFor, httpStatusOf } from "~/wire/grpcClient";
import { isAuthMethod, isCardsMethod, isGameMethod, isRpcGroup } from "~/wire/rpcs";
import type { DeckError, IntentEnvelope, SaveDeckRequest, StreamFrame } from "~/wire/types";

export interface RpcEnv extends GrpcRequestEnv {
  readonly defaultAddress: string;
  /** Resolve a table id to its owning pod's gRPC address, or `null` for an unknown table. */
  readonly resolveTableAddress: (tableId: string) => Promise<string | null>;
}

export type RpcOutcome =
  | { kind: "json"; status: number; body: unknown; setSessionToken?: string; clearSession?: boolean }
  | { kind: "empty"; status: number; clearSession?: boolean }
  | { kind: "stream"; frames: AsyncIterable<StreamFrame> };

function jsonOk(body: unknown): RpcOutcome {
  return { kind: "json", status: 200, body };
}

/** `Status::invalid_argument("illegal deck: a; b; c")` → `{ problems: ["a","b","c"] }` — the gRPC
 * `Decks` service folds `DeckOpError::Illegal`'s structured problems into one status message
 * (`crates/server/src/grpc/decks_svc.rs`); this is the inverse, restoring the shape the deck
 * builder's 422 handling already expects. */
function deckErrorOf(message: string): DeckError {
  const prefix = "illegal deck: ";
  if (!message.startsWith(prefix)) return { problems: [message] };
  return { problems: message.slice(prefix.length).split("; ") };
}

function fromGrpcError(err: unknown): RpcOutcome {
  if (!(err instanceof GrpcCallError)) throw err;
  const status = httpStatusOf(err.code);
  const body = err.code === "invalid_argument" ? deckErrorOf(err.message) : { error: err.message };
  return { kind: "json", status, body };
}

async function dispatchAuth(method: string | undefined, body: unknown, env: RpcEnv): Promise<RpcOutcome> {
  if (!isAuthMethod(method)) return { kind: "empty", status: 404 };
  const client = grpcClientFor(env.defaultAddress, env);
  try {
    return await Match.value(method).pipe(
      Match.when("signup", async () => {
        const req = body as { email: string; password: string; username: string };
        const res = await client.auth.signup(req, env.sessionToken);
        return { kind: "json" as const, status: 200, body: res.me, setSessionToken: res.sessionToken };
      }),
      Match.when("login", async () => {
        const req = body as { email: string; password: string };
        const res = await client.auth.login(req, env.sessionToken);
        return { kind: "json" as const, status: 200, body: res.me, setSessionToken: res.sessionToken };
      }),
      Match.when("logout", async () => {
        await client.auth.logout(env.sessionToken);
        return { kind: "empty" as const, status: 204, clearSession: true };
      }),
      Match.when("me", async () => jsonOk(await client.auth.getMe(env.sessionToken))),
      Match.exhaustive,
    );
  } catch (err) {
    return fromGrpcError(err);
  }
}

async function dispatchCards(method: string | undefined, query: URLSearchParams, env: RpcEnv): Promise<RpcOutcome> {
  if (!isCardsMethod(method)) return { kind: "empty", status: 404 };
  const client = grpcClientFor(env.defaultAddress, env);
  try {
    return await Match.value(method).pipe(
      Match.when("catalog", async () => jsonOk(await client.cards.catalog())),
      Match.when("search", async () => {
        const q = query.get("q") ?? "";
        const limit = Number(query.get("limit") ?? "50");
        const offset = Number(query.get("offset") ?? "0");
        return jsonOk(await client.cards.search(q, limit, offset));
      }),
      Match.when("lookup", async () => jsonOk(await client.cards.lookup(query.getAll("ids")))),
      Match.exhaustive,
    );
  } catch (err) {
    return fromGrpcError(err);
  }
}

async function dispatchDecks(
  id: string | undefined,
  httpMethod: string,
  body: unknown,
  env: RpcEnv,
): Promise<RpcOutcome> {
  const client = grpcClientFor(env.defaultAddress, env);
  try {
    if (id === undefined) {
      if (httpMethod === "GET") return jsonOk(await client.decks.list(env.sessionToken));
      if (httpMethod === "POST") return jsonOk(await client.decks.create(body as SaveDeckRequest, env.sessionToken));
      return { kind: "empty", status: 405 };
    }
    const deckId = Number(id);
    if (httpMethod === "GET") return jsonOk(await client.decks.get(deckId, env.sessionToken));
    if (httpMethod === "PUT")
      return jsonOk(await client.decks.update(deckId, body as SaveDeckRequest, env.sessionToken));
    if (httpMethod === "DELETE") {
      await client.decks.delete(deckId, env.sessionToken);
      return { kind: "empty", status: 204 };
    }
    return { kind: "empty", status: 405 };
  } catch (err) {
    return fromGrpcError(err);
  }
}

async function dispatchGame(
  tableId: string | undefined,
  method: string | undefined,
  body: unknown,
  env: RpcEnv,
): Promise<RpcOutcome> {
  if (!tableId || !isGameMethod(method)) return { kind: "empty", status: 404 };
  const address = await env.resolveTableAddress(tableId);
  if (!address) return { kind: "empty", status: 404 };
  const client = grpcClientFor(address, env);
  try {
    return await Match.value(method).pipe(
      Match.when("intent", async () =>
        jsonOk(await client.game.submitIntent(tableId, body as IntentEnvelope, env.sessionToken)),
      ),
      Match.when("yield", async () =>
        jsonOk(await client.game.setYield(tableId, (body as { enabled: boolean }).enabled, env.sessionToken)),
      ),
      Match.when("turn-yield", async () =>
        jsonOk(await client.game.setTurnYield(tableId, (body as { enabled: boolean }).enabled, env.sessionToken)),
      ),
      Match.when("stack-dwell", async () =>
        jsonOk(await client.game.setStackDwell(tableId, (body as { dwelling: boolean }).dwelling, env.sessionToken)),
      ),
      Match.when("stream", async () => ({
        kind: "stream" as const,
        frames: client.game.stream(tableId, env.sessionToken),
      })),
      Match.exhaustive,
    );
  } catch (err) {
    return fromGrpcError(err);
  }
}

/** Route `/api/rpc/<segments…>` to a gRPC call. `segments` already omit the leading `rpc`. */
export function dispatchRpc(
  segments: ReadonlyArray<string>,
  httpMethod: string,
  body: unknown,
  query: URLSearchParams,
  env: RpcEnv,
): Promise<RpcOutcome> {
  const [group, ...rest] = segments;
  if (!isRpcGroup(group)) return Promise.resolve({ kind: "empty", status: 404 });
  return Match.value(group).pipe(
    Match.when("auth", () => dispatchAuth(rest[0], body, env)),
    Match.when("cards", () => dispatchCards(rest[0], query, env)),
    Match.when("decks", () => dispatchDecks(rest[0], httpMethod, body, env)),
    Match.when("game", () => dispatchGame(rest[0], rest[1], body, env)),
    Match.exhaustive,
  );
}
