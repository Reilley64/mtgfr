import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const appRoot = import.meta.dirname;

describe("network-only PWA service worker", () => {
  it("keeps the fetch handler network-only without cache writes", () => {
    const swPath = resolve(appRoot, "sw.ts");

    expect(existsSync(swPath)).toBe(true);

    const sw = readFileSync(swPath, "utf8");

    expect(sw).toMatch(/respondWith\(fetch\(event\.request\)\)/);
    expect(sw).not.toMatch(/caches\./);
    expect(sw).not.toMatch(/cache\.put/);
    expect(sw).not.toMatch(/runtimeCaching/);
  });

  it("keeps vite-plugin-pwa out of precache mode", () => {
    const viteConfig = readFileSync(resolve(appRoot, "../vite.config.ts"), "utf8");

    expect(viteConfig).toContain("VitePWA");
    expect(viteConfig).toContain('outDir: ".output/public"');
    expect(viteConfig).toContain("globPatterns: []");
    expect(viteConfig).toContain("injectionPoint: undefined");
    expect(viteConfig).toContain("devOptions: { enabled: false }");
    expect(viteConfig).not.toMatch(/runtimeCaching\s*:/);
  });
});
