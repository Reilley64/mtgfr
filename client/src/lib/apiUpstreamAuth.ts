// BFF-side helpers the lobby route handler (`~/routes/api/[...path].ts`) calls against the API:
// `fetchMe`/`fetchDeckName`/`seedGame` now dial tonic over gRPC (ADR 0032) — the BFF's session
// cookie becomes the `x-session-token` metadata the gRPC services authenticate with. Only
// `fetchApiVersion` stays HTTP: health stays Axum's `/health/live` on :8080 (ADR 0032's
// "health stays HTTP").

import { GrpcCallError, grpcClient, httpStatusOf } from "~/wire/grpcClient";
import type { SaveDeckRequest, SeedRequest, SeedResponse } from "~/wire/types";

export function apiUpstream(): string {
  return (process.env.API_UPSTREAM ?? "http://127.0.0.1:8080").replace(/\/$/, "");
}

/** The default (unrouted) tonic gRPC address: `GRPC_UPSTREAM` if set, else `apiUpstream()`'s
 * host on the gRPC port (50051) — the same instance as the default HTTP upstream in dev/single
 * process, since both come up together (`main.rs::run_serve`). */
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

export async function fetchMe(sessionToken: string | null): Promise<Me | null> {
  if (!sessionToken) return null;
  try {
    const me = await grpcClient(grpcUpstream()).auth.getMe(sessionToken);
    return parseMePayload(me);
  } catch {
    return null;
  }
}

export async function fetchDeckName(sessionToken: string | null, deckId: number): Promise<string | null> {
  if (!sessionToken) return null;
  try {
    const deck = await grpcClient(grpcUpstream()).decks.get(deckId, sessionToken);
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
  sessionToken: string | null,
  body: SeedRequest,
): Promise<{ ok: true; data: SeedResponse } | { ok: false; status: number }> {
  if (!sessionToken) return { ok: false, status: 401 };
  try {
    const data = await grpcClient(grpcUpstream()).tables.seed(body, sessionToken);
    return { ok: true, data };
  } catch (err) {
    if (err instanceof GrpcCallError) {
      return { ok: false, status: httpStatusOf(err.code) };
    }
    return { ok: false, status: 500 };
  }
}

export type { SaveDeckRequest };
