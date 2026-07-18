// Auto-discovered client plugin — Faro RUM + OTEL tracing (ADR 0034).

import { getWebInstrumentations, initializeFaro } from "@grafana/faro-web-sdk";
import { TracingInstrumentation } from "@grafana/faro-web-tracing";
import { appVersion, gitCommit } from "~/lib/buildMeta";
import { defineClientPlugin } from "~/plugins/runtime";

const COLLECT_URL = "/api/faro/collect";

let started = false;

export default defineClientPlugin(() => {
  if (started || typeof window === "undefined") return;
  started = true;

  initializeFaro({
    url: COLLECT_URL,
    app: {
      name: "edh-web",
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
