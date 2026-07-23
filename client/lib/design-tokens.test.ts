import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const cssPath = new URL("../styles/tokens.generated.css", import.meta.url);
const tsPath = new URL("./design-tokens.generated.ts", import.meta.url);

describe("tokens.generated.css", () => {
  it("exists and defines forest-floor in @theme", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toContain("@theme");
    expect(css).toMatch(/--color-forest-floor\s*:\s*#0b1310/i);
  });

  it("exports CSS-only and nested Tailwind theme tokens", () => {
    const css = readFileSync(cssPath, "utf8");
    expect(css).toContain("--color-hud-edge: #5A786966;");
    expect(css).toContain("--shadow-table: 0 12px 40px rgb(0 0 0 / 0.6);");
    expect(css).toContain("--text-title--font-weight: 700;");
    expect(css).toContain("--drop-shadow-drag: 0 10px 24px rgb(0 0 0 / 0.6);");
    expect(css).toContain("--font-sans: system-ui, sans-serif;");
  });
});

describe("design-tokens.generated.ts", () => {
  it("exports required named colors for canvas", async () => {
    expect(readFileSync(tsPath, "utf8")).toContain("export const colors");
    const mod = await import("./design-tokens.generated");
    const required = [
      "forestFloor",
      "priorityGold",
      "playableBorder",
      "commanderGold",
      "graveyardOutline",
      "exileOutline",
      "oracleIvory",
      "morphSlate",
      "mountainRed",
      "wallGreen",
      "islandBlue",
      "llanowar",
      "llanowarDeep",
      "reconnectRust",
      "damageCrimson",
      "phaseMint",
    ] as const;
    for (const key of required) {
      expect(mod.colors[key]).toMatch(/^#[0-9A-Fa-f]{6,8}$/);
    }
  });
});
