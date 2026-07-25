import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = resolve(import.meta.dirname, "..");

describe("favicon assets", () => {
  const html = readFileSync(resolve(root, "index.html"), "utf8");
  const svg = readFileSync(resolve(root, "public/favicon.svg"), "utf8");
  const ico = readFileSync(resolve(root, "public/favicon.ico"));

  it("prefers SVG icon then ICO fallback in index.html", () => {
    expect(html).toContain('<link rel="icon" href="/favicon.svg" type="image/svg+xml" />');
    expect(html).toContain('<link rel="icon" href="/favicon.ico" sizes="any" />');
  });

  it("ships a forest-floor circle with transparent dragon cutout", () => {
    expect(svg).toContain('viewBox="0 0 32 32"');
    expect(svg).toContain('aria-label="edh.reilley.dev"');
    expect(svg).toContain('fill="#0B1310"');
    expect(svg).toMatch(/fill-rule=["']evenodd["']/);
    expect(svg).not.toMatch(/<rect\b/i);
    expect(svg).not.toMatch(/#E9B84A/i);
    expect(svg).toMatch(/<path\b/i);
  });

  it("ships a multi-image ICO (16 and 32)", () => {
    // ICONDIR: reserved=0, type=1 (icon), count>=2
    expect(ico[0]).toBe(0);
    expect(ico[1]).toBe(0);
    expect(ico[2]).toBe(1);
    expect(ico[3]).toBe(0);
    const count = ico[4]! | (ico[5]! << 8);
    expect(count).toBeGreaterThanOrEqual(2);
  });
});
