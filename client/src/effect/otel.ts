// BFF OpenTelemetry — shared process lifetime via @effect/opentelemetry.
// Init once from the Nitro `otel.server` plugin; exporters only when
// OTEL_EXPORTER_OTLP_ENDPOINT is set (cluster). Local/dev uses an empty layer.

import { NodeSdk, OtelTracer } from "@effect/opentelemetry";
import { OTLPLogExporter } from "@opentelemetry/exporter-logs-otlp-http";
import { OTLPMetricExporter } from "@opentelemetry/exporter-metrics-otlp-http";
import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { BatchLogRecordProcessor } from "@opentelemetry/sdk-logs";
import { PeriodicExportingMetricReader } from "@opentelemetry/sdk-metrics";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import type * as Effect from "effect/Effect";
import * as ManagedRuntime from "effect/ManagedRuntime";
import { appVersion, gitCommit } from "~/lib/buildMeta";
import { parseTraceparent } from "~/lib/traceContext";

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
 * No-op when the header is missing or invalid — the effect stays a local root.
 */
export function continueIncomingTrace<A, E, R>(
  effect: Effect.Effect<A, E, R>,
  traceparent: string | null,
): Effect.Effect<A, E, R> {
  const parsed = parseTraceparent(traceparent);
  if (!parsed) return effect;
  return OtelTracer.withSpanContext(effect, {
    traceId: parsed.traceId,
    spanId: parsed.spanId,
    traceFlags: parsed.traceFlags,
    isRemote: true,
  }) as Effect.Effect<A, E, R>;
}

/** Flush/dispose exporters on Nitro `close`. */
export async function shutdownOtel(): Promise<void> {
  if (!runtime) return;
  await runtime.dispose();
  runtime = null;
}
