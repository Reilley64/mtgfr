import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const cssPath = new URL("../../styles/tokens.generated.css", import.meta.url);
const tsPath = new URL("./design-tokens.generated.ts", import.meta.url);
const tokensPath = new URL("../../../design.tokens.json", import.meta.url);

describe("design.tokens.json", () => {
  it("has no $type css passthroughs", () => {
    const raw = readFileSync(tokensPath, "utf8");
    expect(raw).not.toMatch(/"\$type"\s*:\s*"css"/);
  });

  it("uses primitive and semantic top-level groups", () => {
    const json = JSON.parse(readFileSync(tokensPath, "utf8"));
    expect(json.primitive?.color).toBeTypeOf("object");
    expect(json.semantic?.color).toBeTypeOf("object");
  });
});

describe("tokens.generated.css", () => {
  it("defines forest-floor as oklch in @theme under the public name", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toContain("@theme");
    expect(css).toMatch(/--color-forest-floor\s*:\s*oklch\(/);
    expect(css).not.toContain("--color-primitive-");
    expect(css).not.toMatch(/--semantic-color-/);
  });

  it("emits typed shadow, ease, duration, and composed animate vars", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toMatch(/--shadow-table\s*:\s*[^;]+;/);
    expect(css).toMatch(/--ease-state\s*:\s*cubic-bezier\(0\.22,\s*1,\s*0\.36,\s*1\)/);
    expect(css).toMatch(/--duration-stack-in\s*:\s*0\.25s/);
    expect(css).toMatch(/--animate-stack-in\s*:\s*stack-in 0\.25s ease-out/);
    expect(css).toMatch(/--animate-shell-enter\s*:\s*shell-enter 0\.2s var\(--ease-state\)/);
    expect(css).toMatch(/--drop-shadow-drag\s*:\s*0 16px 36px /);
  });
});

describe("design-tokens.generated.ts", () => {
  it("exports oklch colors, alias equality, shadowDrag, hexFallbacks", async () => {
    expect(readFileSync(tsPath, "utf8")).toContain("export const colors");
    const mod = await import("./design-tokens.generated");
    expect(mod.colors.forestFloor).toMatch(/^oklch\(/);
    expect(mod.colors.playableBorder).toBe(mod.colors.snowMint);
    expect(mod.shadowDrag).toEqual({
      css: expect.stringMatching(/^0 16px 36px /),
      offsetY: 16,
      blur: 36,
      color: expect.stringMatching(/^(rgba?\(|oklch\()/),
    });
    expect(mod.hexFallbacks.forestFloor).toMatch(/^#[0-9A-Fa-f]{6}$/);
  });
});
