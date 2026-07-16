/// <reference types="vitest/config" />

import tailwindcss from "@tailwindcss/vite";
import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [tailwindcss(), solid()],
  resolve: {
    tsconfigPaths: true,
  },
  // Proxy every `/api/*` call to the Axum backend, stripped of the `/api` prefix, so the client
  // stays same-origin (the generated client and the hand-written stream both use relative paths).
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:8080",
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  // The pure camera/hit-test modules are plain math — no DOM needed.
  test: {
    environment: "node",
  },
});
