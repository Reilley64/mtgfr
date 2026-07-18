// BFF helpers for the lobby route: me/deck/seed over gRPC; version stays HTTP `/health/live`.

import { GrpcCallError, grpcClientFor, httpStatusOf, type GrpcRequestEnv } from "~/wire/grpcClient";
import type { SaveDeckRequest, SeedRequest, SeedResponse } from "~/wire/types";

export function apiUpstream(): string {
  return (process.env.API_UPSTREAM ?? "http://127.0.0.1:8080").replace(/\/$/, "");
}

/** Default tonic address: `GRPC_UPSTREAM`, else `apiUpstream()`'s host on `:50051`. */
export function grpcUpstream(): string {
  if (process.env.GRPC_UPSTREAM) return process.env.GRPC_UPSTREAM.replace(/\/$/, "");
  return `${new URL(apiUpstream()).hostname}:50051`;
}

export type Me = { id: number; email: string; username: string };

/** Parse a `Me` value. Returns null when `id` is missing (stale API) or the shape is wrong. */
export function parseMePayload(body: unknown): Me | null {
  if (body === null || typeof body !== "object") return null;
  const rec = body as Record<string, unknown>;
  if (typeof rec.id !== "number" || !Number.isFinite(rec.id)) return null;
  if (typeof rec.email !== "string" || typeof rec.username !== "string") return null;
  return { id: rec.id, email: rec.email, username: rec.username };
}

export async function fetchMe(env: GrpcRequestEnv): Promise<Me | null> {
  if (!env.sessionToken) return null;
  try {
    return await grpcClientFor(grpcUpstream(), env).auth.getMe(env.sessionToken);
  } catch {
    return null;
  }
}

export async function fetchDeckName(env: GrpcRequestEnv, deckId: number): Promise<string | null> {
  if (!env.sessionToken) return null;
  try {
    const deck = await grpcClientFor(grpcUpstream(), env).decks.get(deckId, env.sessionToken);
    return deck.name ?? null;
  } catch {
    return null;
  }
}

export async function fetchApiVersion(): Promise<string | null> {
  const res = await fetch(`${apiUpstream()}/health/live`);
  if (!res.ok) return null;
  const body = (await res.json()) as { version?: string };
  return body.version ?? null;
}

export type { SeedResponse };

export async function seedGame(
  env: GrpcRequestEnv,
  body: SeedRequest,
): Promise<{ ok: true; data: SeedResponse } | { ok: false; status: number }> {
  if (!env.sessionToken) return { ok: false, status: 401 };
  try {
    const data = await grpcClientFor(grpcUpstream(), env).tables.seed(body, env.sessionToken);
    return { ok: true, data };
  } catch (err) {
    if (err instanceof GrpcCallError) {
      return { ok: false, status: httpStatusOf(err.code) };
    }
    return { ok: false, status: 500 };
  }
}

export type { SaveDeckRequest };
