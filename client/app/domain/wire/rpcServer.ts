// `/api/rpc` dispatcher — unit-testable without a Nitro route.

import * as Effect from "effect/Effect";
import * as Match from "effect/Match";
import * as Stream from "effect/Stream";
import { GrpcCallError, type GrpcRequestEnv, grpcClientFor, httpStatusOf } from "./grpcClient";
import { isAuthMethod, isCardsMethod, isGameMethod, isRatingsMethod, isRpcGroup } from "./rpcs";
import type { DeckError, IntentEnvelope, SaveDeckRequest, StreamFrame } from "./types";

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

function empty(status: number, clearSession = false): RpcOutcome {
  if (clearSession) return { kind: "empty", status, clearSession };
  return { kind: "empty", status };
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
  if (!(err instanceof GrpcCallError)) return { kind: "json", status: 500, body: { error: "InternalServerError" } };
  const status = httpStatusOf(err.code);
  const body = err.code === "invalid_argument" ? deckErrorOf(err.message) : { error: err.message };
  return { kind: "json", status, body };
}

function badQuery(): RpcOutcome {
  return { kind: "json", status: 400, body: { error: "BadQuery" } };
}

function streamFrames(frames: Stream.Stream<StreamFrame, GrpcCallError>): AsyncIterable<StreamFrame> {
  return Stream.toAsyncIterable(frames);
}

type Uint32Query = { ok: true; value: number } | { ok: false };

const UINT32_MAX = 4_294_967_295;

/** Parse a uint32 query param; missing/empty uses `defaultValue`, invalid values reject. */
function parseUint32Query(raw: string | null, defaultValue: number): Uint32Query {
  if (raw === null || raw === "") return { ok: true, value: defaultValue };
  const n = Number(raw);
  if (!Number.isInteger(n) || n < 0 || n > UINT32_MAX) return { ok: false };
  return { ok: true, value: n };
}

type LeaderboardPaging = { ok: true; limit: number; offset: number } | { ok: false };

function parseLeaderboardPaging(query: URLSearchParams): LeaderboardPaging {
  const limit = parseUint32Query(query.get("limit"), 0);
  if (!limit.ok) return { ok: false };
  const offset = parseUint32Query(query.get("offset"), 0);
  if (!offset.ok) return { ok: false };
  return { ok: true, limit: limit.value, offset: offset.value };
}

const dispatchAuth = Effect.fn(function* (method: string | undefined, body: unknown, env: RpcEnv) {
  if (!isAuthMethod(method)) return empty(404);
  const client = grpcClientFor(env.defaultAddress, env);
  return yield* Match.value(method).pipe(
    Match.when("signup", () =>
      Effect.gen(function* () {
        const req = body as { email: string; password: string; username: string };
        const res = yield* client.auth.signup(req, env.sessionToken);
        return { kind: "json" as const, status: 200, body: res.me, setSessionToken: res.sessionToken };
      }),
    ),
    Match.when("login", () =>
      Effect.gen(function* () {
        const req = body as { email: string; password: string };
        const res = yield* client.auth.login(req, env.sessionToken);
        return { kind: "json" as const, status: 200, body: res.me, setSessionToken: res.sessionToken };
      }),
    ),
    Match.when("logout", () =>
      Effect.gen(function* () {
        yield* client.auth.logout(env.sessionToken);
        return empty(204, true);
      }),
    ),
    Match.when("me", () => client.auth.getMe(env.sessionToken).pipe(Effect.map(jsonOk))),
    Match.exhaustive,
  );
});

const dispatchCards = Effect.fn(function* (method: string | undefined, query: URLSearchParams, env: RpcEnv) {
  if (!isCardsMethod(method)) return empty(404);
  const client = grpcClientFor(env.defaultAddress, env);
  return yield* Match.value(method).pipe(
    Match.when("catalog", () => client.cards.catalog().pipe(Effect.map(jsonOk))),
    Match.when("search", () =>
      Effect.gen(function* () {
        const q = query.get("q") ?? "";
        const limit = Number(query.get("limit") ?? "50");
        const offset = Number(query.get("offset") ?? "0");
        return jsonOk(yield* client.cards.search(q, limit, offset));
      }),
    ),
    Match.when("lookup", () => client.cards.lookup(query.getAll("ids")).pipe(Effect.map(jsonOk))),
    Match.exhaustive,
  );
});

const dispatchRatings = Effect.fn(function* (
  method: string | undefined,
  httpMethod: string,
  query: URLSearchParams,
  env: RpcEnv,
) {
  if (!isRatingsMethod(method)) return empty(404);
  if (httpMethod !== "GET") return empty(405);
  const client = grpcClientFor(env.defaultAddress, env);
  return yield* Match.value(method).pipe(
    Match.when("leaderboard", () =>
      Effect.gen(function* () {
        const paging = parseLeaderboardPaging(query);
        if (!paging.ok) return badQuery();
        return jsonOk(
          yield* client.ratings.getLeaderboard({ limit: paging.limit, offset: paging.offset }, env.sessionToken),
        );
      }),
    ),
    Match.exhaustive,
  );
});

const dispatchDecks = Effect.fn(function* (id: string | undefined, httpMethod: string, body: unknown, env: RpcEnv) {
  const client = grpcClientFor(env.defaultAddress, env);
  if (id === undefined) {
    if (httpMethod === "GET") return jsonOk(yield* client.decks.list(env.sessionToken));
    if (httpMethod === "POST") return jsonOk(yield* client.decks.create(body as SaveDeckRequest, env.sessionToken));
    return empty(405);
  }
  const deckId = Number(id);
  if (httpMethod === "GET") return jsonOk(yield* client.decks.get(deckId, env.sessionToken));
  if (httpMethod === "PUT") {
    return jsonOk(yield* client.decks.update(deckId, body as SaveDeckRequest, env.sessionToken));
  }
  if (httpMethod === "DELETE") {
    yield* client.decks.delete(deckId, env.sessionToken);
    return empty(204);
  }
  return empty(405);
});

const dispatchGame = Effect.fn(function* (
  tableId: string | undefined,
  method: string | undefined,
  body: unknown,
  env: RpcEnv,
) {
  if (!tableId || !isGameMethod(method)) return empty(404);
  const address = yield* Effect.tryPromise({
    try: () => env.resolveTableAddress(tableId),
    catch: (err) => err,
  });
  if (!address) return empty(404);
  const client = grpcClientFor(address, env);
  return yield* Match.value(method).pipe(
    Match.when("intent", () =>
      client.game.submitIntent(tableId, body as IntentEnvelope, env.sessionToken).pipe(Effect.map(jsonOk)),
    ),
    Match.when("yield", () =>
      client.game.setYield(tableId, (body as { enabled: boolean }).enabled, env.sessionToken).pipe(Effect.map(jsonOk)),
    ),
    Match.when("turn-yield", () =>
      client.game
        .setTurnYield(tableId, (body as { enabled: boolean }).enabled, env.sessionToken)
        .pipe(Effect.map(jsonOk)),
    ),
    Match.when("stack-dwell", () =>
      client.game
        .setStackDwell(tableId, (body as { dwelling: boolean }).dwelling, env.sessionToken)
        .pipe(Effect.map(jsonOk)),
    ),
    Match.when("stream", () =>
      Effect.succeed({
        kind: "stream" as const,
        frames: streamFrames(client.game.stream(tableId, env.sessionToken)),
      }),
    ),
    Match.exhaustive,
  );
});

function recoverRpcError(err: unknown): Effect.Effect<RpcOutcome, never> {
  return Effect.succeed(fromGrpcError(err));
}

/** Route `/api/rpc/<segments…>` to a gRPC call. `segments` already omit the leading `rpc`. */
export const dispatchRpc: (
  segments: ReadonlyArray<string>,
  httpMethod: string,
  body: unknown,
  query: URLSearchParams,
  env: RpcEnv,
) => Effect.Effect<RpcOutcome, never> = Effect.fn(function* (
  segments: ReadonlyArray<string>,
  httpMethod: string,
  body: unknown,
  query: URLSearchParams,
  env: RpcEnv,
) {
  const [group, ...rest] = segments;
  if (!isRpcGroup(group)) return empty(404);
  return yield* Match.value(group).pipe(
    Match.when("auth", () => dispatchAuth(rest[0], body, env)),
    Match.when("cards", () => dispatchCards(rest[0], query, env)),
    Match.when("decks", () => dispatchDecks(rest[0], httpMethod, body, env)),
    Match.when("game", () => dispatchGame(rest[0], rest[1], body, env)),
    Match.when("ratings", () => dispatchRatings(rest[0], httpMethod, query, env)),
    Match.exhaustive,
    Effect.catch(recoverRpcError),
  );
});
