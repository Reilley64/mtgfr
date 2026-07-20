// Auto-discovered client plugin — Faro RUM + OTEL tracing (production-topology-and-operations spec).

import { getWebInstrumentations, initializeFaro } from "@grafana/faro-web-sdk";
import { TracingInstrumentation } from "@grafana/faro-web-tracing";
import { appVersion, gitCommit } from "~/lib/buildMeta";
import { ensureFaroSessionSampled } from "~/lib/faroSession";
import { defineClientPlugin } from "~/plugins/runtime";

const COLLECT_URL = "/api/faro/collect";

let started = false;

const plugin = defineClientPlugin(() => {
  if (started || typeof window === "undefined") return;
  started = true;

  // Repair resumed Faro sessions that lack isSampled / are stuck unsampled —
  // otherwise web-tracing never records spans but still injects traceparent.
  ensureFaroSessionSampled();

  initializeFaro({
    url: COLLECT_URL,
    app: {
      // Same service.name as the BFF; distinguish via telemetry.sdk.name in Grafana.
      name: "edh-web",
      version: appVersion(),
      gitHash: gitCommit(),
    },
    // Explicit 100% session sampling for self-hosted LGTM (Faro tracing keys off this).
    sessionTracking: {
      enabled: true,
      samplingRate: 1,
    },
    instrumentations: [
      ...getWebInstrumentations({
        captureConsole: false,
      }),
      new TracingInstrumentation({
        instrumentationOptions: {
          // Same-origin BFF only — do not leak traceparent to Scryfall/etc.
          propagateTraceHeaderCorsUrls: [/\/api(?:\/|$)/],
        },
      }),
    ],
    // Avoid instrumenting the collect endpoint itself (feedback loop).
    ignoreUrls: [COLLECT_URL],
  });
});

export default plugin;

// Eager boot when this module is imported as a side effect (entry-client).
// `started` keeps virtual:app-plugins-client from double-initializing.
void plugin.setup({});
