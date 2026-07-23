/// <reference types="vitest/config" />

import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    // Match vite.config.ts — path aliases come from tsconfig.json (`~/*`).
    // Requires root vite@8 (see package.json overrides); vitest may otherwise nest vite@6.
    tsconfigPaths: true,
  },
  test: {
    environment: "node",
    exclude: [...configDefaults.exclude, ".output/**"],
  },
});
