import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { clientBuildSourcemap } from "./client-build-options";

describe("client production source maps", () => {
  it("emits referenced maps for large first-party JS (not hidden)", () => {
    // Chrome's "Missing source maps for large first-party JavaScript" insight
    // requires a //# sourceMappingURL= comment — only sourcemap: true adds it.
    expect(clientBuildSourcemap).toBe(true);
  });

  it("wires the option into vite.config build.sourcemap", () => {
    const viteConfig = readFileSync(resolve(import.meta.dirname, "../vite.config.ts"), "utf8");
    expect(viteConfig).toContain("clientBuildSourcemap");
    expect(viteConfig).toMatch(/sourcemap:\s*clientBuildSourcemap/);
  });
});
