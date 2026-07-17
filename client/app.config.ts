import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { appPlugins, discoverAppPlugins } from "./vite/app-plugins";

const { server: serverPlugins } = discoverAppPlugins();

// `/api` BFF is a request-time route (`API_UPSTREAM` + table_routes); Nitro routeRules would bake the target.
export default defineConfig({
  ssr: false,
  server: {
    preset: "node_server",
    // Nitro runtime plugins — auto-discovered from `src/plugins/*.server.ts`.
    plugins: serverPlugins,
  },
  vite: {
    plugins: [appPlugins(), tailwindcss()],
    test: {
      environment: "node",
    },
  },
});
