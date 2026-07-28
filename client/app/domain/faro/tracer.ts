// Back Effect's tracer with the OpenTelemetry provider Faro registers.
//
// Effect's `HttpClient` opens a span per request and injects its `traceparent`. Without this the
// span comes from Effect's own tracer: real ids, sampled flag set, but exported by nothing. The
// BFF parents `rpc <path>` under it (`continueIncomingTrace`), the API parents under the BFF, and
// Tempo holds a trace whose root never arrives — `<root span not yet received>`, on every call,
// whether or not the player leaves the page.
//
// Sharing Faro's provider makes that span a real browser span: Faro batches and ships it to Tempo,
// so the trace is rooted in the click that caused it. With no provider registered (local dev, or
// Faro disabled) the global is a no-op, the injected traceparent is unsampled, and the BFF drops
// it and becomes the root itself.

import { OtelTracer, Resource } from "@effect/opentelemetry";
import * as Layer from "effect/Layer";
import { appVersion } from "../build-meta";

export const browserTracerLayer = OtelTracer.layerGlobal.pipe(
  Layer.provide(Resource.layer({ serviceName: "edh-web", serviceVersion: appVersion() })),
);
