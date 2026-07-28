import { describe, expect, it } from "vitest";
import { cva } from "./recipe";

describe("recipe seam", () => {
  it("lets a variant override the base for the same CSS property", () => {
    const recipe = cva({
      base: "bg-llanowar px-lg",
      variants: { tone: { quiet: "bg-tapped-out" } },
    });

    expect(recipe({ tone: "quiet" })).toContain("bg-tapped-out");
    expect(recipe({ tone: "quiet" })).not.toContain("bg-llanowar");
  });

  it("keeps this project's type scale out of the colour group", () => {
    const recipe = cva({ base: "text-caption text-burn-red" });

    // Unconfigured tailwind-merge treats `text-caption` as a colour and drops it.
    expect(recipe()).toContain("text-caption");
    expect(recipe()).toContain("text-burn-red");
  });

  it("merges call-site classes last", () => {
    const recipe = cva({ base: "px-lg" });

    expect(recipe({ class: "px-xs" })).toContain("px-xs");
    expect(recipe({ class: "px-xs" })).not.toContain("px-lg");
  });
});
