import { describe, expect, it } from "vitest";
import { menuItemClass, menuPanelClass } from "./menu";

describe("menuItemClass", () => {
  it("is a transparent, borderless row", () => {
    const c = menuItemClass();

    expect(c).toContain("bg-transparent");
    expect(c).toContain("border-none");
    expect(c).toContain("cursor-pointer");
  });

  it("highlights on hover", () => {
    expect(menuItemClass()).toContain("hover:bg-white/8");
  });

  it("shows a focus-visible ring instead of the default outline", () => {
    const c = menuItemClass();

    expect(c).toContain("focus-visible:outline-none");
    expect(c).toContain("focus-visible:ring-1");
    expect(c).toContain("focus-visible:ring-vine");
  });

  it("keeps the base row chrome when a call site adds its own utility", () => {
    const c = menuItemClass("no-underline");

    expect(c).toContain("no-underline");
    expect(c).toContain("bg-transparent");
  });
});

describe("menuPanelClass", () => {
  it("is forest-surface chrome with a vine border and table shadow", () => {
    const c = menuPanelClass();

    expect(c).toContain("rounded-hud");
    expect(c).toContain("border-vine");
    expect(c).toContain("bg-forest-surface");
    expect(c).toContain("shadow-table");
    expect(c).toContain("flex-col");
  });

  it("lets a call-site padding win over the base instead of stacking beside it", () => {
    const c = menuPanelClass("p-lg");

    expect(c).toContain("p-lg");
    expect(c).not.toContain("p-xs");
  });
});
