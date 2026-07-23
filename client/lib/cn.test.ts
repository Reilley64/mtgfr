// `cn` merges Tailwind classes so a conditional can *override* the base instead of piling up
// beside it: `cn("cursor-default", actionable && "cursor-grab")`.
//
// tailwind-merge ships knowing stock Tailwind's scales, not ours. Left unconfigured it reads
// `text-caption` as a colour, decides it conflicts with `text-burn-red`, and silently drops the
// font size — on ~25 sites. So `cn` re-declares our @theme scales, and the first test here fails
// the build if that list ever drifts from global.css (the drift client-shell-deck-builder-and-observability spec exists to prevent).

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { cn, THEME_SCALES } from "~/cn";

// Read global.css off disk, not via Vite's `?raw` — the Tailwind plugin claims `.css` imports and
// hands back an empty string in the node test environment.
const css = readFileSync(new URL("../styles/global.css", import.meta.url), "utf8");

/** The keys defined under a `--<prefix>-*` namespace in @theme, ignoring modifiers like
 * `--text-title--font-weight`. */
function themeKeys(css: string, prefix: string): string[] {
  const keys = [...css.matchAll(new RegExp(`--${prefix}-([a-z0-9-]+)\\s*:`, "g"))].map((m) => m[1]);
  return [...new Set(keys.filter((k) => !k.includes("--")))].sort();
}

describe("THEME_SCALES", () => {
  // Without this, a failed read would make every assertion below compare [] to [] and pass.
  it("actually read global.css", () => {
    expect(css).toContain("@theme");
  });

  it.each(["text", "radius", "spacing"] as const)("mirrors global.css's --%s-* namespace", (scale) => {
    const found = themeKeys(css, scale);
    expect(found.length).toBeGreaterThan(0);
    expect([...THEME_SCALES[scale]].sort()).toEqual(found);
  });
});

describe("cn", () => {
  it("drops falsy conditionals", () => {
    expect(cn("flex", false && "hidden", undefined, null, "gap-sm")).toBe("flex gap-sm");
  });

  // The whole point: a later conditional beats the base for the same CSS property.
  it("lets a conditional override the base", () => {
    expect(cn("cursor-default", "cursor-grab")).toBe("cursor-grab");
    expect(cn("flex-row", "flex-row-reverse")).toBe("flex-row-reverse");
    expect(cn("opacity-100", "opacity-55", "opacity-25")).toBe("opacity-25");
    expect(cn("text-lichen", "text-mist")).toBe("text-mist");
  });

  it("keeps the base when the condition is false", () => {
    expect(cn("cursor-default", false && "cursor-grab")).toBe("cursor-default");
    expect(cn("opacity-100", false && "opacity-55")).toBe("opacity-100");
  });

  // The regression stock tailwind-merge would cause: a font size is not a colour.
  it("never confuses a custom font size with a colour", () => {
    expect(cn("text-caption", "text-burn-red")).toBe("text-caption text-burn-red");
    expect(cn("text-label", "text-lichen")).toBe("text-label text-lichen");
    expect(cn("text-title", "text-snow")).toBe("text-title text-snow");
    expect(cn("font-semibold", "text-caption", "text-turn-mint")).toBe("font-semibold text-caption text-turn-mint");
  });

  it("resolves our custom radius and spacing scales", () => {
    expect(cn("rounded-hud", "rounded-control")).toBe("rounded-control");
    expect(cn("px-lg", "px-md")).toBe("px-md");
    expect(cn("px-lg", "py-sm")).toBe("px-lg py-sm"); // different properties, both kept
  });

  // Utility recipes from `~/components/atoms` compose through cn like any other class list — later entries
  // override earlier ones for the same CSS property (e.g. quiet/yielded on top of game).
  it("lets button recipe utilities override via order", () => {
    expect(cn("bg-llanowar-deep", "bg-tapped-out", "bg-yielded")).toBe("bg-yielded");
    expect(cn("text-snow-mint", "text-mist", "text-yielded-ink")).toBe("text-yielded-ink");
    expect(cn("border-vine", "border-burn-red")).toBe("border-burn-red");
  });
});
