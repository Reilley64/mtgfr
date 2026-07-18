// Auto-discovered client plugin — Faro RUM + OTEL tracing (ADR 0034).

import { getWebInstrumentations, initializeFaro } from "@grafana/faro-web-sdk";
import { TracingInstrumentation } from "@grafana/faro-web-tracing";
import { appVersion, gitCommit } from "~/lib/buildMeta";
import { defineClientPlugin } from "~/plugins/runtime";

const COLLECT_URL = "/api/faro/collect";

let started = false;

const plugin = defineClientPlugin(() => {
  if (started || typeof window === "undefined") return;
  started = true;

  initializeFaro({
    url: COLLECT_URL,
    app: {
      // Distinct from BFF OTEL `edh-web` so Tempo/Loki can tell browser RUM apart.
      name: "edh-browser",
      version: appVersion(),
      gitHash: gitCommit(),
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
