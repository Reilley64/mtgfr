import * as Effect from "effect/Effect";
import { getCookie, type H3Event } from "nitro/h3";
import { fetchMe, type Me } from "../app/domain/api-upstream-auth";
import { type LobbySnapshot, sweepWebDb } from "../app/domain/lobby-store";
import { grpcRequestEnv, runTracedRequest } from "../app/domain/otel";
import { HTTP_RESPONSE_STATUS_CODE, httpServerAttrs } from "../app/domain/otel/semconv";
import type { GrpcRequestEnv } from "../app/domain/wire/grpcClient";
import { type WebDb, WebDbLive } from "./db/client";

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

function annotateHttpStatus(response: Response): Effect.Effect<void> {
  return Effect.annotateCurrentSpan({
    [HTTP_RESPONSE_STATUS_CODE]: response.status,
  });
}

type LobbyAuthCtx = {
  me: Me;
  env: GrpcRequestEnv;
};

export async function withLobbyAuth<E>(
  event: H3Event,
  spanName: string,
  body: (ctx: LobbyAuthCtx) => Effect.Effect<Response, E, WebDb>,
): Promise<Response> {
  const sessionToken = getCookie(event, SESSION_COOKIE) ?? null;
  const traceparent = event.req.headers.get("traceparent");
  try {
    return await runTracedRequest(
      traceparent,
      spanName,
      Effect.gen(function* () {
        yield* Effect.annotateCurrentSpan(httpServerAttrs({ method: event.req.method, route: spanName }));
        const env = yield* grpcRequestEnv(sessionToken);
        const me = yield* fetchMe(env);
        if (!me) {
          const response = new Response("Unauthorized", { status: 401 });
          yield* annotateHttpStatus(response);
          return response;
        }
        yield* sweepWebDb();
        const response = yield* body({ me, env });
        yield* annotateHttpStatus(response);
        return response;
      }).pipe(Effect.provide(WebDbLive)),
    );
  } catch (err) {
    return json({ error: "LobbyDb", message: lobbyDbErrorMessage(err) }, 500);
  }
}

export async function runMetaGet<E>(
  event: H3Event,
  spanName: string,
  body: () => Effect.Effect<Response, E>,
): Promise<Response> {
  const traceparent = event.req.headers.get("traceparent");
  return runTracedRequest(
    traceparent,
    spanName,
    Effect.gen(function* () {
      yield* Effect.annotateCurrentSpan(httpServerAttrs({ method: event.req.method, route: spanName }));
      const response = yield* body();
      yield* annotateHttpStatus(response);
      return response;
    }),
  );
}
