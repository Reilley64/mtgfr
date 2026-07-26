import { foldkit } from "@foldkit/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";
import { VitePWA } from "vite-plugin-pwa";
import { clientBuildSourcemap } from "./app/domain/client-build-options";

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
    // Vite 8+: single source of truth is tsconfig.json compilerOptions.paths (`~/*` → `./app/domain/*`).
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
    // Do not add precache/runtimeCaching without a product decision (Wave 3 design).
    VitePWA({
      strategies: "injectManifest",
      outDir: ".output/public",
      srcDir: "app",
      filename: "sw.ts",
      injectRegister: false,
      registerType: "autoUpdate",
      manifest: {
        id: "/",
        name: "edh.reilley.dev",
        short_name: "edh.reilley.dev",
        start_url: "/",
        scope: "/",
        display: "standalone",
        background_color: "#0B1310",
        theme_color: "#0B1310",
        icons: [
          {
            src: "pwa-192.png",
            sizes: "192x192",
            type: "image/png",
            purpose: "any",
          },
          {
            src: "pwa-512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "any",
          },
          {
            src: "pwa-512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "maskable",
          },
        ],
      },
      injectManifest: {
        globPatterns: [],
        injectionPoint: undefined,
      },
      devOptions: { enabled: false },
    }),
  ],
});
