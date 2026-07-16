/// <reference types="vitest/config" />

import path from "node:path";
import { fileURLToPath } from "node:url";
import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      "~": path.join(root, "src"),
    },
    conditions: ["browser", "development"],
  },
  test: {
    environment: "node",
  },
});
