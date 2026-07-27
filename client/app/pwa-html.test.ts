import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { hexFallbacks } from "./domain/design-tokens.generated";

const root = resolve(import.meta.dirname, "..");

describe("PWA HTML metadata", () => {
  const html = readFileSync(resolve(root, "index.html"), "utf8");

  it("declares the theme color and Apple touch icon", () => {
    expect(html).toContain(`<meta name="theme-color" content="${hexFallbacks.forestFloor}" />`);
    expect(html).toContain('<link rel="apple-touch-icon" href="/apple-touch-icon.png" />');
  });
});
