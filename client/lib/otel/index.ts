// BFF OpenTelemetry — process-scoped `@effect/opentelemetry` (production-topology-and-operations spec).
//
// Architecture ([Effect tracing](https://effect.website/docs/observability/tracing) +
// `OtelTracer.withSpanContext`):
// - Business/request code uses Effect APIs (`Effect.fn`, `Effect.withSpan`, annotations).
// - OTEL JS exporters live once in `NodeSdk.layer` — not inside handlers.
// - W3C `traceparent` bridging stays at the HTTP/RPC edge only.
// - Parent spans live in Effect **fiber Context**. A second `ManagedRuntime` (gRPC)
//   does not inherit them — capture via `currentTraceparent` and pass explicitly
//   into `grpcClient` / `RpcEnv`. Never rely on Node ALS across that boundary.

import { NodeSdk, OtelTracer } from "@effect/opentelemetry";
import { OTLPLogExporter } from "@opentelemetry/exporter-logs-otlp-http";
import { OTLPMetricExporter } from "@opentelemetry/exporter-metrics-otlp-http";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
import { PeriodicExportingMetricReader } from "@opentelemetry/sdk-metrics";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import * as Effect from "effect/Effect";
import * as ManagedRuntime from "effect/ManagedRuntime";
import { appVersion, gitCommit } from "../build-meta";
import { formatTraceparent, parseTraceparent } from "../trace-context";

function otlpEndpoint(): string | null {
  const raw = process.env.OTEL_EXPORTER_OTLP_ENDPOINT?.trim();
  return raw && raw.length > 0 ? raw.replace(/\/$/, "") : null;
}

function serviceName(): string {
  return process.env.OTEL_SERVICE_NAME?.trim() || "edh-web";
}

function buildLayer() {
  const endpoint = otlpEndpoint();
  if (!endpoint) {
    return NodeSdk.layerEmpty;
  }

  const tracesUrl = `${endpoint}/v1/traces`;
  const logsUrl = `${endpoint}/v1/logs`;
  const metricsUrl = `${endpoint}/v1/metrics`;

  return NodeSdk.layer(() => ({
    resource: {
      serviceName: serviceName(),
      serviceVersion: appVersion(),
      attributes: {
        "vcs.ref.head.revision": gitCommit(),
      },
    },
    spanProcessor: new BatchSpanProcessor(new OTLPTraceExporter({ url: tracesUrl })),
    logRecordProcessor: new BatchLogRecordProcessor({
      exporter: new OTLPLogExporter({ url: logsUrl }),
    }),
    metricReader: new PeriodicExportingMetricReader({
      exporter: new OTLPMetricExporter({ url: metricsUrl }),
      exportIntervalMillis: 15_000,
    }),
  }));
}

type OtelRuntime = ManagedRuntime.ManagedRuntime<never, never>;

let runtime: OtelRuntime | null = null;

/** Idempotent — Nitro plugin calls this once at server start. */
export function initOtel(): void {
  if (runtime) return;
  runtime = ManagedRuntime.make(buildLayer()) as OtelRuntime;
}

async function ensureRuntime(): Promise<OtelRuntime> {
  if (!runtime) initOtel();
  if (!runtime) throw new Error("otel runtime failed to initialize");
  return runtime;
}

/** Run an Effect on the process-scoped OTEL runtime (no per-request SDK teardown). */
export async function runTraced<A, E>(effect: Effect.Effect<A, E>): Promise<A> {
  const rt = await ensureRuntime();
  return rt.runPromise(effect as Effect.Effect<A, E, never>);
}

/**
 * Parent this effect's spans under an incoming W3C `traceparent` (Faro / upstream).
 * Integration boundary only — keep business ops oblivious to propagation.
 * No-op when the header is missing, invalid, or **unsampled** — the effect stays a
 * local root. Faro's tracing sampler often marks sessions NOT_RECORD while fetch
 * instrumentation still injects a traceparent for the non-recording span; parenting
 * under that span leaves Tempo `<root span not yet received>` orphans.
 */
export function continueIncomingTrace<A, E, R>(
  effect: Effect.Effect<A, E, R>,
  traceparent: string | null,
): Effect.Effect<A, E, R> {
  const parsed = parseTraceparent(traceparent);
  if (!parsed) return effect;
  // W3C: bit 0 = sampled. Unsamped contexts are never exported by Faro/OTEL web.
  if ((parsed.traceFlags & 0x01) === 0) return effect;
  return OtelTracer.withSpanContext(effect, {
    traceId: parsed.traceId,
    spanId: parsed.spanId,
    traceFlags: parsed.traceFlags,
    isRemote: true,
  }) as Effect.Effect<A, E, R>;
}

/**
 * HTTP/RPC edge: continue inbound W3C context, open `spanName`, run on the OTEL runtime.
 * Prefer this over hand-rolling `runTraced` + `continueIncomingTrace` + `withSpan` at each route.
 */
export async function runTracedRequest<A, E>(
  traceparent: string | null,
  spanName: string,
  body: Effect.Effect<A, E>,
): Promise<A> {
  return runTraced(continueIncomingTrace(body.pipe(Effect.withSpan(spanName)), traceparent));
}

/**
 * W3C `traceparent` for the current Effect span (outbound gRPC parenting).
 * Call only under an active `Effect.withSpan` (e.g. inside `runTracedRequest`).
 * Unnamed `Effect.fn` — stack traces without an extra named span (guide default).
 */
export const currentTraceparent = Effect.fn(function* () {
  const span = yield* Effect.currentSpan;
  return formatTraceparent({
    traceId: span.traceId,
    spanId: span.spanId,
    sampled: span.sampled,
  });
});

/**
 * Build the shared `{ sessionToken, traceparent }` bag once per request edge.
 * Both `/api/rpc` and `/api` lobby use this — same design, no per-route variants.
 */
export const grpcRequestEnv = Effect.fn(function* (sessionToken: string | null) {
  return {
    sessionToken,
    traceparent: yield* currentTraceparent(),
  } as const;
});

/** Flush/dispose exporters on Nitro `close`. */
export async function shutdownOtel(): Promise<void> {
  if (!runtime) return;
  await runtime.dispose();
  runtime = null;
}
