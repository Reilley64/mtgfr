// The browser must never hand the BFF a `traceparent` for a span nobody exports.
//
// Effect's `HttpClient` opens a span for every request and injects its `traceparent`. Backed by
// Effect's own tracer that span is invisible to Faro, so the BFF parents `rpc <path>` under an id
// that never reaches Tempo and the trace renders as `<root span not yet received>` forever.

import { trace } from "@opentelemetry/api";
import { BasicTracerProvider, InMemorySpanExporter, SimpleSpanProcessor } from "@opentelemetry/sdk-trace-base";
import * as Effect from "effect/Effect";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { makeClient } from "../rpc-client";
import { browserTracerLayer } from "./tracer";

beforeAll(() => vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" }));
afterEach(() => trace.disable());

/** Stand in for the provider `initializeFaro` registers, but keep the spans in memory. */
function registerCollectingProvider(): InMemorySpanExporter {
  const exporter = new InMemorySpanExporter();
  const provider = new BasicTracerProvider({ spanProcessors: [new SimpleSpanProcessor(exporter)] });
  trace.disable();
  trace.setGlobalTracerProvider(provider);
  return exporter;
}

function recordingFetch(): { fetch: typeof fetch; traceparents: (string | null)[] } {
  const traceparents: (string | null)[] = [];
  const fetchImpl = ((_url: URL, init?: RequestInit) => {
    traceparents.push(new Headers(init?.headers).get("traceparent"));
    return Promise.resolve(
      new Response(JSON.stringify({ accepted: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
  }) as unknown as typeof fetch;
  return { fetch: fetchImpl, traceparents };
}

const envelope = { table_id: "t_abc", client_seq: 1, intent: {} } as never;

/** `00-<trace>-<span>-<flags>` */
function spanIdOf(traceparent: string | null): string | null {
  return traceparent?.split("-")[2] ?? null;
}

describe("browserTracerLayer", () => {
  it("exports the span whose traceparent the BFF will parent under", async () => {
    const exporter = registerCollectingProvider();
    const { fetch, traceparents } = recordingFetch();

    await Effect.runPromise(makeClient(fetch).submitIntent("t_abc", envelope).pipe(Effect.provide(browserTracerLayer)));

    const sent = spanIdOf(traceparents[0]);
    expect(sent).not.toBeNull();
    expect(exporter.getFinishedSpans().map((span) => span.spanContext().spanId)).toContain(sent);
  });

  it("marks the request unsampled when Faro never registered a provider, so the BFF stays the root", async () => {
    trace.disable();
    const { fetch, traceparents } = recordingFetch();

    await Effect.runPromise(makeClient(fetch).submitIntent("t_abc", envelope).pipe(Effect.provide(browserTracerLayer)));

    expect(traceparents[0]?.endsWith("-00")).toBe(true);
  });
});
