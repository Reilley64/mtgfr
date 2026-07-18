import { describe, expect, it } from "vitest";
import { buttonClass } from "~/components/atoms/button";

describe("buttonClass (cva)", () => {
  it("defaults to primary", () => {
    const c = buttonClass();
    expect(c).toContain("bg-llanowar");
    expect(c).toContain("text-snow-mint");
  });

  it("lets game-quiet override game min-width and fill", () => {
    const c = buttonClass("game-quiet");
    expect(c).toContain("min-w-0");
    expect(c).not.toContain("min-w-[132px]");
    expect(c).toContain("bg-tapped-out");
    expect(c).toContain("hover:enabled:bg-quiet-hover");
  });

  it("layers game-yielded on quiet padding with amber fill", () => {
    const c = buttonClass("game-yielded");
    expect(c).toContain("bg-yielded");
    expect(c).toContain("text-yielded-ink");
    expect(c).toContain("min-w-0");
    expect(c).toContain("ease-state");
  });

  it("danger keeps control chrome with burn-red ink", () => {
    const c = buttonClass("danger");
    expect(c).toContain("border-burn-red");
    expect(c).toContain("text-burn-red");
    expect(c).toContain("rounded-control");
  });

  it("merges call-site utilities last", () => {
    expect(buttonClass("ghost", "fixed top-3")).toContain("fixed");
    expect(buttonClass("ghost", "text-burn-red")).toContain("text-burn-red");
  });
});
