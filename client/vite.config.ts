import path from "node:path";
import { fileURLToPath } from "node:url";
import { foldkit } from "@foldkit/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  environments: {
    ssr: {},
  },
  server: {
    // Nitro binds IPv6 ::1 by default; dual-stack so `127.0.0.1` and LAN work too.
    host: true,
    port: 3000,
  },
  resolve: {
    alias: {
      "~": path.join(root, "lib"),
    },
  },
  plugins: [
    foldkit(),
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
