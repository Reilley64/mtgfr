import { describe, expect, it } from "vitest";
import { publicCssVarName, resolveAliases, serializeTokenCss, serializeTokenValue } from "./gen-tokens.mjs";

describe("gen-tokens helpers", () => {
  it("resolves DTCG aliases without mutating the source tree", () => {
    const root = {
      primitive: {
        color: {
          snow: {
            $type: "color",
            $value: {
              colorSpace: "oklch",
              components: [0.97, 0.03, 145],
            },
          },
        },
      },
      semantic: {
        color: {
          playable: { $type: "color", $value: "{primitive.color.snow}" },
        },
      },
    };

    const resolved = resolveAliases(root);

    expect(resolved.semantic.color.playable.$value).toEqual({
      colorSpace: "oklch",
      components: [0.97, 0.03, 145],
    });
    expect(root.semantic.color.playable.$value).toBe("{primitive.color.snow}");
  });

  it("maps public CSS names for semantic tokens and skips primitives", () => {
    expect(publicCssVarName(["primitive", "color", "snow"])).toBeNull();
    expect(publicCssVarName(["semantic", "color", "forest-floor"])).toBe("--color-forest-floor");
    expect(publicCssVarName(["text", "title", "font-weight"])).toBe("--text-title--font-weight");
  });

  it("serializes typed DTCG values for CSS output", () => {
    expect(
      serializeTokenValue("shadow", {
        offsetX: { value: 0, unit: "px" },
        offsetY: { value: 16, unit: "px" },
        blur: { value: 36, unit: "px" },
        color: {
          colorSpace: "srgb",
          components: [0, 0, 0],
          alpha: 0.72,
        },
      }),
    ).toBe("0px 16px 36px rgb(0 0 0 / 0.72)");
    expect(serializeTokenValue("cubicBezier", [0.22, 1, 0.36, 1])).toBe("cubic-bezier(0.22, 1, 0.36, 1)");
    expect(serializeTokenValue("duration", { value: 0.25, unit: "s" })).toBe("0.25s");
    expect(serializeTokenValue("css", "var(--legacy-bridge)")).toBe("var(--legacy-bridge)");
    expect(
      serializeTokenCss({
        path: ["semantic", "color", "snow"],
        $type: "color",
        $value: {
          colorSpace: "oklch",
          components: [0.97, 0.03, 145],
        },
      }),
    ).toBe("oklch(0.97 0.03 145)");
  });
});
