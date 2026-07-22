// Faro RUM client boot — Grafana Faro Web SDK + tracing instrumentation.
//
// Mirrors the retired SolidStart `client/src/plugins/otel.client.ts`:
// - Same-origin `/api/faro/collect` collector (Nitro forwards to `FARO_COLLECT_UPSTREAM`).
// - Session sampling repair for resumed sessions (avoids Tempo orphans).
// - Traceparent propagation restricted to `/api/*` — never leaks to Scryfall/etc.
// - No hand/library/intent payloads ever touch telemetry (that data lives in the wire types,
//   not in DOM events or window state Faro captures).

import { getWebInstrumentations, initializeFaro } from "@grafana/faro-web-sdk";
import { TracingInstrumentation } from "@grafana/faro-web-tracing";
import { appVersion, gitCommit } from "../lib/build-meta";
import { ensureFaroSessionSampled } from "../lib/faro/session";

const COLLECT_URL = "/api/faro/collect";

let started = false;

export function initFaro(): void {
  if (started) return;
  if (typeof window === "undefined") return;
  started = true;

  ensureFaroSessionSampled();

  initializeFaro({
    url: COLLECT_URL,
    app: {
      name: "edh-web",
      version: appVersion(),
      gitHash: gitCommit(),
    },
    sessionTracking: {
      enabled: true,
      samplingRate: 1,
    },
    instrumentations: [
      ...getWebInstrumentations({ captureConsole: false }),
      new TracingInstrumentation({
        instrumentationOptions: {
          propagateTraceHeaderCorsUrls: [/\/api(?:\/|$)/],
        },
      }),
    ],
    ignoreUrls: [COLLECT_URL],
  });
}
