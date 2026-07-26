// BFF helpers for the lobby route: me/deck/seed over gRPC; meta/version stays HTTP `/health/live`.

import * as Effect from "effect/Effect";
import { ensureOracleTotalRefresh, getCachedOracleTotal } from "./scryfall-oracle-total";
import { GrpcCallError, type GrpcRequestEnv, grpcClientFor, httpStatusOf } from "./wire/grpcClient";
import type { SaveDeckRequest, SeedRequest, SeedResponse } from "./wire/types";

export function apiUpstream(): string {
  return (process.env.API_UPSTREAM ?? "http://127.0.0.1:8080").replace(/\/$/, "");
}

/** Default tonic address: `GRPC_UPSTREAM`, else `apiUpstream()`'s host on `:50051`. */
export function grpcUpstream(): string {
  if (process.env.GRPC_UPSTREAM) return process.env.GRPC_UPSTREAM.replace(/\/$/, "");
  return `${new URL(apiUpstream()).hostname}:50051`;
}

export type Me = { id: number; email: string; username: string };

export type SeedGameResult = { ok: true; data: SeedResponse } | { ok: false; status: number };

/** Parse a `Me` value. Returns null when `id` is missing (stale API) or the shape is wrong. */
export function parseMePayload(body: unknown): Me | null {
  if (body === null || typeof body !== "object") return null;
  const rec = body as Record<string, unknown>;
  if (typeof rec.id !== "number" || !Number.isFinite(rec.id)) return null;
  if (typeof rec.email !== "string" || typeof rec.username !== "string") return null;
  return { id: rec.id, email: rec.email, username: rec.username };
}

export const fetchMe = Effect.fn(function* (env: GrpcRequestEnv) {
  if (!env.sessionToken) return null;
  return yield* grpcClientFor(grpcUpstream(), env)
    .auth.getMe(env.sessionToken)
    .pipe(Effect.catch(() => Effect.succeed(null)));
});

export const fetchDeckName = Effect.fn(function* (env: GrpcRequestEnv, deckId: number) {
  if (!env.sessionToken) return null;
  return yield* grpcClientFor(grpcUpstream(), env)
    .decks.get(deckId, env.sessionToken)
    .pipe(
      Effect.map((deck) => deck.name ?? null),
      Effect.catch(() => Effect.succeed(null)),
    );
});

export type LiveStatus = {
  version: string;
  faithfulCount: number | null;
  faithfulBySet: Readonly<Record<string, number>> | null;
};

function readFaithfulBySet(value: unknown): Readonly<Record<string, number>> | null {
  if (value === null || typeof value !== "object") return null;

  const entries = Object.entries(value);
  const faithfulBySet: Record<string, number> = {};

  for (const [code, count] of entries) {
    if (typeof count !== "number" || !Number.isFinite(count)) continue;
    faithfulBySet[code] = count;
  }

  return faithfulBySet;
}

export function parseLiveStatus(body: unknown): LiveStatus | null {
  if (body === null || typeof body !== "object") return null;
  if (!("version" in body)) return null;
  if (typeof body.version !== "string" || body.version.length === 0) return null;
  const faithfulBySet = "faithful_by_set" in body ? readFaithfulBySet(body.faithful_by_set) : null;
  if (!("faithful_count" in body)) {
    return { version: body.version, faithfulCount: null, faithfulBySet };
  }
  if (typeof body.faithful_count !== "number" || !Number.isFinite(body.faithful_count)) {
    return { version: body.version, faithfulCount: null, faithfulBySet };
  }
  return { version: body.version, faithfulCount: body.faithful_count, faithfulBySet };
}

function unavailableApiMeta() {
  return {
    version: null,
    faithfulCount: null,
    oracleTotal: getCachedOracleTotal(),
  };
}

export async function fetchApiMeta(): Promise<{
  version: string | null;
  faithfulCount: number | null;
  oracleTotal: number | null;
}> {
  ensureOracleTotalRefresh();
  try {
    const res = await fetch(`${apiUpstream()}/health/live`);
    if (!res.ok) return unavailableApiMeta();
    const parsed = parseLiveStatus(await res.json());
    if (!parsed) return unavailableApiMeta();
    return {
      version: parsed.version,
      faithfulCount: parsed.faithfulCount,
      oracleTotal: getCachedOracleTotal(),
    };
  } catch {
    return unavailableApiMeta();
  }
}

export type { SeedResponse };

function seedOk(data: SeedResponse): SeedGameResult {
  return { ok: true, data };
}

function seedError(status: number): SeedGameResult {
  return { ok: false, status };
}

export const seedGame = Effect.fn(function* (env: GrpcRequestEnv, body: SeedRequest) {
  if (!env.sessionToken) return seedError(401);
  return yield* grpcClientFor(grpcUpstream(), env)
    .tables.seed(body, env.sessionToken)
    .pipe(
      Effect.map(seedOk),
      Effect.catch((err) => {
        if (err instanceof GrpcCallError) return Effect.succeed(seedError(httpStatusOf(err.code)));
        return Effect.succeed(seedError(500));
      }),
    );
});

export type { SaveDeckRequest };
