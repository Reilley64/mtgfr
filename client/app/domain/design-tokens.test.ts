import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const cssPath = new URL("../../styles/tokens.generated.css", import.meta.url);
const tsPath = new URL("./design-tokens.generated.ts", import.meta.url);
const tokensPath = new URL("../../../design.tokens.json", import.meta.url);

type ColorValue =
  | string
  | {
      colorSpace: string;
      components: number[];
      alpha?: number;
    };

type DtcgToken = {
  $type?: string;
  $value: ColorValue;
};

type DesignTokens = {
  primitive: {
    color: Record<string, DtcgToken>;
    space: Record<string, DtcgToken>;
  };
  semantic: {
    color: Record<string, DtcgToken>;
    font: Record<string, DtcgToken>;
    text: Record<string, DtcgToken>;
    radius: Record<string, DtcgToken>;
    spacing: Record<string, DtcgToken>;
    size: Record<string, DtcgToken>;
  };
};

function readTokens(): DesignTokens {
  return JSON.parse(readFileSync(tokensPath, "utf8"));
}

function visitTokens(
  value: unknown,
  visit: (token: { $type?: unknown; $value?: unknown }, path: string) => void,
  path = "",
): void {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return;
  }

  if ("$value" in value) {
    visit(value, path);
  }
  for (const [key, child] of Object.entries(value)) {
    if (key.startsWith("$")) {
      continue;
    }
    visitTokens(child, visit, path ? `${path}.${key}` : key);
  }
}

describe("design.tokens.json", () => {
  it("has no $type css passthroughs", () => {
    const raw = readFileSync(tokensPath, "utf8");
    expect(raw).not.toMatch(/"\$type"\s*:\s*"css"/);
  });

  it("uses primitive and semantic top-level groups", () => {
    const json = readTokens();
    expect(Object.keys(json).sort()).toEqual(["primitive", "semantic"]);
    expect(json.primitive?.color).toBeTypeOf("object");
    expect(json.primitive?.space).toBeTypeOf("object");
    expect(json.semantic?.color).toBeTypeOf("object");
    expect(json.semantic?.font).toBeTypeOf("object");
    expect(json.semantic?.text).toBeTypeOf("object");
    expect(json.semantic?.radius).toBeTypeOf("object");
    expect(json.semantic?.spacing).toBeTypeOf("object");
    expect(json.semantic?.size).toBeTypeOf("object");
  });

  it("keeps color literals private to primitives and publishes semantic aliases", () => {
    const json = readTokens();
    const primitiveColors = json.primitive.color;
    const semanticColors = json.semantic.color;

    for (const [name, token] of Object.entries(primitiveColors)) {
      expect(token, name).toMatchObject({ $type: "color" });
      expect(String(token.$value), name).not.toMatch(/^\{/);
      expect(token.$value, name).toMatchObject({
        colorSpace: "oklch",
        components: expect.any(Array),
      });
    }

    for (const [name, token] of Object.entries(semanticColors)) {
      expect(token, name).toMatchObject({ $type: "color" });
      expect(token.$value, name).toMatch(/^\{(?:primitive|semantic)\.color\.[a-z0-9-]+\}$/);
    }

    expect(semanticColors["playable-border"].$value).toBe("{semantic.color.snow-mint}");
  });

  it("aliases public spacing through primitive space tokens", () => {
    const json = readTokens();
    expect(json.semantic.spacing.xxl.$value).toBe("{primitive.space.xxl}");
    expect(json.semantic.spacing["shell-gutter"].$value).toBe("{primitive.space.xxl}");
    expect(json.semantic.spacing["shell-header-y"].$value).toBe("{primitive.space.shell-header-y}");
  });

  it("authors every shadow layer with an explicit spread", () => {
    const json = readTokens();
    const missing: string[] = [];

    visitTokens(json, (token, path) => {
      if (token.$type !== "shadow") {
        return;
      }

      const layers = Array.isArray(token.$value) ? token.$value : [token.$value];
      layers.forEach((layer, index) => {
        if (!layer || typeof layer !== "object" || Array.isArray(layer)) {
          return;
        }
        if (!("spread" in layer)) {
          missing.push(`${path}[${index}]`);
        }
      });
    });

    expect(missing).toEqual([]);
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
    expect(css).toMatch(/--animate-shell-enter\s*:\s*shell-enter 0\.2s var\(--ease-state\)/);
    expect(css).toMatch(/--drop-shadow-drag\s*:\s*0 16px 36px rgb\(0 0 0 \/ 0\.72\);/);
  });

  it("does not leave unresolved token aliases in generated outputs", () => {
    expect(readFileSync(cssPath, "utf8")).not.toMatch(/\{[a-z0-9.-]+\}/i);
    expect(readFileSync(tsPath, "utf8")).not.toMatch(/\{[a-z0-9.-]+\}/i);
  });
});

describe("design-tokens.generated.ts", () => {
  it("exports oklch colors, alias equality, shadowDrag, hexFallbacks", async () => {
    expect(readFileSync(tsPath, "utf8")).toContain("export const colors");
    const mod = await import("./design-tokens.generated");
    expect(mod.colors.forestFloor).toMatch(/^oklch\(/);
    expect(mod.colors.priorityGold).toMatch(/^oklch\(/);
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
