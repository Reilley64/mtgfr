import * as Effect from "effect/Effect";
import { getCookie, type H3Event } from "nitro/h3";
import { fetchMe } from "../app/domain/api-upstream-auth";
import { type LobbySnapshot, sweepWebDb } from "../app/domain/lobby-store";
import { grpcRequestEnv, runTracedRequest } from "../app/domain/otel";
import type { GrpcRequestEnv } from "../app/domain/wire/grpcClient";
import { createWebDb } from "./db/client";

export const SESSION_COOKIE = "session";

export function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json" },
  });
}

export function tableParam(event: H3Event): string | null {
  const table = event.context?.params?.table;
  if (typeof table !== "string" || table.length === 0) return null;
  return table;
}

export async function readJsonObject(event: H3Event): Promise<Record<string, unknown> | null> {
  try {
    const raw = await event.req.text();
    const value: unknown = JSON.parse(raw || "");
    if (value === null || typeof value !== "object" || Array.isArray(value)) return null;
    return value as Record<string, unknown>;
  } catch {
    return null;
  }
}

export function unknownLobby(tableId: string): LobbySnapshot {
  return { tableId, hostUserId: 0, startedAt: null, seats: [] };
}

function lobbyDbErrorMessage(err: unknown): string {
  if (err instanceof Error) return err.message.slice(0, 300);
  return String(err).slice(0, 300);
}

type LobbyAuthCtx = {
  me: NonNullable<Awaited<ReturnType<typeof fetchMe>>>;
  env: GrpcRequestEnv;
  db: ReturnType<typeof createWebDb>;
};

export async function withLobbyAuth(
  event: H3Event,
  spanName: string,
  fn: (ctx: LobbyAuthCtx) => Promise<Response>,
): Promise<Response> {
  const sessionToken = getCookie(event, SESSION_COOKIE) ?? null;
  const traceparent = event.req.headers.get("traceparent");
  try {
    return await runTracedRequest(
      traceparent,
      spanName,
      Effect.gen(function* () {
        yield* Effect.annotateCurrentSpan({
          "http.method": event.req.method,
          "http.route": spanName,
        });
        const env = yield* grpcRequestEnv(sessionToken);
        return yield* Effect.tryPromise({
          try: async () => {
            const me = await fetchMe(env);
            if (!me) return new Response("Unauthorized", { status: 401 });
            const db = createWebDb();
            await sweepWebDb(db);
            return fn({ me, env, db });
          },
          catch: (err) => (err instanceof Error ? err : new Error(String(err))),
        });
      }),
    );
  } catch (err) {
    return json({ error: "LobbyDb", message: lobbyDbErrorMessage(err) }, 500);
  }
}

export async function runMetaGet(event: H3Event, spanName: string, fn: () => Promise<Response>): Promise<Response> {
  const traceparent = event.req.headers.get("traceparent");
  return runTracedRequest(
    traceparent,
    spanName,
    Effect.gen(function* () {
      yield* Effect.annotateCurrentSpan({
        "http.method": event.req.method,
        "http.route": spanName,
      });
      return yield* Effect.tryPromise({
        try: fn,
        catch: (err) => (err instanceof Error ? err : new Error(String(err))),
      });
    }),
  );
}
