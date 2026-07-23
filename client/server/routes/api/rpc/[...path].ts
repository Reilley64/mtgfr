// Same-origin `/api/rpc` gateway: cookie/body plumbing around `dispatchRpc`.

import * as Effect from "effect/Effect";
import {
  defineEventHandler,
  deleteCookie,
  getCookie,
  getMethod,
  getRequestHeader,
  getRequestURL,
  getRouterParam,
  type H3Event,
  readRawBody,
  setCookie,
} from "nitro/h3";
import { grpcUpstreamFromPodDns } from "../../../../lib/api-upstream";
import { grpcUpstream } from "../../../../lib/api-upstream-auth";
import { lookupTableRoute } from "../../../../lib/lobby-store";
import { grpcRequestEnv, runTracedRequest } from "../../../../lib/otel";
import { GrpcCallError, httpStatusOf } from "../../../../lib/wire/grpcClient";
import { dispatchRpc, type RpcOutcome } from "../../../../lib/wire/rpcServer";
import type { StreamFrame } from "../../../../lib/wire/types";
import { createWebDb } from "../../../db/client";

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
  const db = createWebDb();
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
        // clean end the same as a drop (see `src/effect/stream.ts`).
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
  switch (outcome.kind) {
    case "json":
      return jsonResponse(outcome.body, outcome.status);
    case "empty":
      return new Response(null, { status: outcome.status });
    case "stream":
      return streamResponse(outcome.frames);
    default: {
      const exhaustive: never = outcome;
      return exhaustive;
    }
  }
}

type RpcDispatchArgs = {
  segments: string[];
  method: string;
  body: unknown;
  searchParams: URLSearchParams;
  sessionToken: string | null;
};

/**
 * Dispatch under the request span. Unnamed `Effect.fn` — stack traces without a
 * second named span; the edge opens `rpc <path>` via `runTracedRequest` / `withSpan`.
 * Faro → BFF → API: inject *this* BFF span into gRPC (not the raw inbound header).
 */
const dispatchRpcTraced = Effect.fn(function* (args: RpcDispatchArgs) {
  const path = args.segments.join("/");
  yield* Effect.annotateCurrentSpan({
    "http.method": args.method,
    "rpc.path": path,
  });
  const env = yield* grpcRequestEnv(args.sessionToken);
  return yield* Effect.tryPromise({
    try: () =>
      dispatchRpc(args.segments, args.method, args.body, args.searchParams, {
        ...env,
        defaultAddress: grpcUpstream(),
        resolveTableAddress,
      }),
    catch: (err) => (err instanceof Error ? err : new Error(String(err))),
  });
});

function routeSegments(event: H3Event): string[] {
  return (getRouterParam(event, "path") ?? "").split("/").filter(Boolean);
}

async function jsonBody(event: H3Event): Promise<unknown> {
  const raw = await readRawBody(event, "utf8");
  return JSON.parse(raw ?? "");
}

const ALLOWED_METHODS = new Set(["GET", "POST", "PUT", "DELETE"]);

async function handle(event: H3Event): Promise<Response> {
  const method = getMethod(event);
  if (!ALLOWED_METHODS.has(method)) {
    return new Response("Method Not Allowed", { status: 405 });
  }

  const segments = routeSegments(event);
  const sessionToken = getCookie(event, SESSION_COOKIE) ?? null;
  const traceparent = getRequestHeader(event, "traceparent") ?? null;

  let body: unknown;
  if (method === "POST" || method === "PUT") {
    try {
      body = await jsonBody(event);
    } catch {
      return jsonResponse({ error: "BadJson" }, 400);
    }
  }

  const outcome = await runTracedRequest(
    traceparent,
    `rpc ${segments.join("/") || "root"}`,
    dispatchRpcTraced({
      segments,
      method,
      body,
      searchParams: getRequestURL(event).searchParams,
      sessionToken,
    }),
  );

  if (outcome.kind === "json" && outcome.setSessionToken) {
    setCookie(event, SESSION_COOKIE, outcome.setSessionToken, {
      ...cookieOptions(),
      maxAge: COOKIE_MAX_AGE_SECONDS,
    });
  }
  if (outcome.kind !== "stream" && outcome.clearSession) {
    deleteCookie(event, SESSION_COOKIE, cookieOptions());
  }

  return outcomeToResponse(outcome);
}

export default defineEventHandler(handle);
