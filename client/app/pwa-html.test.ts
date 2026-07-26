import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = resolve(import.meta.dirname, "..");

describe("PWA HTML metadata", () => {
  const html = readFileSync(resolve(root, "index.html"), "utf8");

  it("declares the theme color and Apple touch icon", () => {
    expect(html).toContain('<meta name="theme-color" content="#0B1310" />');
    expect(html).toContain('<link rel="apple-touch-icon" href="/apple-touch-icon.png" />');
  });
});
