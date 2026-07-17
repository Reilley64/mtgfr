// Same-origin `/api/rpc` gateway: cookie/body plumbing around `dispatchRpc`.

import type { APIEvent } from "@solidjs/start/server";
import * as Effect from "effect/Effect";
import * as Match from "effect/Match";
import { deleteCookie, getCookie, setCookie } from "vinxi/http";
import { createWebDb } from "~/db/client";
import { runTraced } from "~/effect/otel";
import { grpcUpstreamFromPodDns } from "~/lib/apiUpstream";
import { grpcUpstream } from "~/lib/apiUpstreamAuth";
import { lookupTableRoute } from "~/lib/lobbyStore";
import { GrpcCallError, httpStatusOf, runWithTraceparent } from "~/wire/grpcClient";
import { dispatchRpc, type RpcOutcome } from "~/wire/rpcServer";
import type { StreamFrame } from "~/wire/types";

const SESSION_COOKIE = "session";
const COOKIE_MAX_AGE_SECONDS = 30 * 24 * 60 * 60; // 30 days — mirrors crates/server/src/auth.rs

function cookieOptions() {
  return {
    httpOnly: true,
    sameSite: "lax" as const,
    secure: process.env.COOKIE_SECURE === "true",
    domain: process.env.COOKIE_DOMAIN || undefined,
    path: "/",
  };
}

/** Resolve a table id to its owning pod's gRPC address. `null` for genuinely unknown tables;
 * falls back to the default (unrouted) address in dev, where every table lives in one process —
 * same semantics as the retired HTTP `resolveGameUpstream`. */
async function resolveTableAddress(tableId: string): Promise<string | null> {
  if (!process.env.WEB_DATABASE_URL) return grpcUpstream();
  const db = createWebDb(process.env.WEB_DATABASE_URL);
  const pod = await lookupTableRoute(db, tableId);
  if (!pod) return null;
  // Legacy seed fallback wrote bare `instance_id` ("local"); that is not DNS.
  if (pod === "local") return grpcUpstream();
  return grpcUpstreamFromPodDns(pod);
}

function jsonResponse(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

function sseChunk(frame: StreamFrame): Uint8Array {
  return new TextEncoder().encode(`data: ${JSON.stringify(frame)}\n\n`);
}

/** Turn a game stream into a `text/event-stream` `Response`. Pulls the first frame before
 * responding at all — a connect-time failure (bad table, expired session) must surface as an HTTP
 * status the reconnect loop can branch on (`statusOf`/`onError`), not as a broken stream body. */
async function streamResponse(frames: AsyncIterable<StreamFrame>): Promise<Response> {
  const iterator = frames[Symbol.asyncIterator]();
  let first: IteratorResult<StreamFrame>;
  try {
    first = await iterator.next();
  } catch (err) {
    if (err instanceof GrpcCallError) {
      return new Response(JSON.stringify({ error: err.message }), {
        status: httpStatusOf(err.code),
        headers: { "content-type": "application/json" },
      });
    }
    return new Response(null, { status: 500 });
  }
  if (first.done) return new Response(null, { status: 200 });

  const body = new ReadableStream<Uint8Array>({
    async start(controller) {
      controller.enqueue(sseChunk(first.value));
      try {
        while (true) {
          const next = await iterator.next();
          if (next.done) break;
          controller.enqueue(sseChunk(next.value));
        }
      } catch {
        // A dropped upstream mid-stream just ends the body; the client's reconnect loop treats a
        // clean end the same as a drop (see `~/effect/stream.ts`).
      }
      controller.close();
    },
    cancel() {
      const withReturn = iterator as AsyncIterator<StreamFrame> & { return?: () => void };
      withReturn.return?.();
    },
  });
  return new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } });
}

function outcomeToResponse(outcome: RpcOutcome): Response | Promise<Response> {
  return Match.value(outcome).pipe(
    Match.discriminatorsExhaustive("kind")({
      json: (o) => jsonResponse(o.body, o.status),
      empty: (o) => new Response(null, { status: o.status }),
      stream: (o) => streamResponse(o.frames),
    }),
  );
}

async function handle(event: APIEvent): Promise<Response> {
  const segments = (event.params.path ?? "").split("/").filter(Boolean);
  const sessionToken = getCookie(event.nativeEvent, SESSION_COOKIE) ?? null;
  const traceparent = event.request.headers.get("traceparent");

  let body: unknown;
  if (event.request.method === "POST" || event.request.method === "PUT") {
    try {
      body = await event.request.json();
    } catch {
      return jsonResponse({ error: "BadJson" }, 400);
    }
  }

  const spanName = `rpc ${segments.join("/") || "root"}`;
  const outcome = await runTraced(
    Effect.gen(function* () {
      return yield* Effect.tryPromise({
        try: () =>
          runWithTraceparent(traceparent, () =>
            dispatchRpc(segments, event.request.method, body, new URL(event.request.url).searchParams, {
              sessionToken,
              defaultAddress: grpcUpstream(),
              resolveTableAddress,
            }),
          ),
        catch: (err) => (err instanceof Error ? err : new Error(String(err))),
      });
    }).pipe(
      Effect.withSpan(spanName, {
        attributes: {
          "http.method": event.request.method,
          "rpc.path": segments.join("/"),
        },
      }),
    ),
  );

  if (outcome.kind === "json" && outcome.setSessionToken) {
    setCookie(event.nativeEvent, SESSION_COOKIE, outcome.setSessionToken, {
      ...cookieOptions(),
      maxAge: COOKIE_MAX_AGE_SECONDS,
    });
  }
  if (outcome.kind !== "stream" && outcome.clearSession) {
    deleteCookie(event.nativeEvent, SESSION_COOKIE, cookieOptions());
  }

  return outcomeToResponse(outcome);
}

export const GET = handle;
export const POST = handle;
export const PUT = handle;
export const DELETE = handle;
