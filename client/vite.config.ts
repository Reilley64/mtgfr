import { foldkit } from "@foldkit/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";
import { clientBuildSourcemap } from "./lib/client-build-options";

export default defineConfig({
  build: {
    // Referenced .map files for large first-party JS (Chrome / Faro).
    sourcemap: clientBuildSourcemap,
  },
  environments: {
    ssr: {},
  },
  server: {
    // Nitro binds IPv6 ::1 by default; dual-stack so `127.0.0.1` and LAN work too.
    host: true,
    port: 3000,
  },
  resolve: {
    // Vite 8+: single source of truth is tsconfig.json compilerOptions.paths (`~/*` → `./lib/*`).
    tsconfigPaths: true,
  },
  plugins: [
    foldkit({ devToolsMcpPort: 9988 }),
    nitro({
      renderer: {
        static: true,
        template: "./index.html",
      },
      serverDir: "./server",
    }),
    tailwindcss(),
  ],
});
