// Auto-discovered Nitro plugin — process-scoped BFF OpenTelemetry (ADR 0034).

import { initOtel, shutdownOtel } from "~/effect/otel";
import { defineServerPlugin } from "~/plugins/runtime";

export default defineServerPlugin((nitroApp) => {
  initOtel();
  nitroApp.hooks.hook("close", async () => {
    await shutdownOtel();
  });
});
