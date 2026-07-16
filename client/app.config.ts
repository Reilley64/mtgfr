import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";

// `/api` BFF is a request-time route (`API_UPSTREAMS`); Nitro routeRules would bake the target.
export default defineConfig({
  ssr: false,
  server: {
    preset: "node_server",
  },
  vite: {
    plugins: [tailwindcss()],
    test: {
      environment: "node",
    },
  },
});
