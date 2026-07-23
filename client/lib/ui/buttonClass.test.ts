import { describe, expect, it } from "vitest";
import { buttonClass } from "./buttonClass";

describe("buttonClass (DESIGN.md §5)", () => {
  it("defaults to primary", () => {
    const c = buttonClass();
    expect(c).toContain("bg-llanowar");
    expect(c).toContain("text-snow-mint");
  });

  it("danger keeps control chrome with burn-red ink", () => {
    const c = buttonClass("danger");
    expect(c).toContain("border-burn-red");
    expect(c).toContain("text-burn-red");
    expect(c).toContain("rounded-control");
  });

  it("link uses vine underline and is markable as data-ui=link via caller", () => {
    const c = buttonClass("link");
    expect(c).toContain("text-vine");
    expect(c).toContain("underline");
    expect(c).toContain("bg-transparent");
  });

  it("lets game-quiet override game min-width and fill", () => {
    const c = buttonClass("game-quiet");
    expect(c).toContain("min-w-0");
    expect(c).not.toContain("min-w-[132px]");
    expect(c).toContain("bg-tapped-out");
  });

  it("merges call-site utilities last", () => {
    expect(buttonClass("ghost", "fixed top-3")).toContain("fixed");
    expect(buttonClass("ghost", "text-burn-red")).toContain("text-burn-red");
  });
});
