import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";

// BFF `/api` proxy lives in `src/routes/api/[...path].ts` so `API_UPSTREAM` is read at
// request time (Nitro `routeRules` bake the target at build time).
export default defineConfig({
  ssr: false,
  server: {
    preset: "node_server",
  },
  vite: {
    plugins: [tailwindcss()],
    // Pure camera/hit-test modules are plain math — no DOM needed for unit tests.
    test: {
      environment: "node",
    },
  },
});
