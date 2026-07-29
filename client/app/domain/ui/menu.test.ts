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

  it("carries the shell overlay z so a portaled panel beats the fixed shell frame", () => {
    expect(menuPanelClass()).toContain("z-41");
  });

  it("lets a call-site z win over the shell default", () => {
    const c = menuPanelClass("z-50");

    expect(c).toContain("z-50");
    expect(c).not.toContain("z-41");
  });
});

describe("the hud variant", () => {
  it("dresses the panel in the board's translucent prompt chrome", () => {
    const c = menuPanelClass(undefined, "hud");

    expect(c).toContain("bg-forest-hud");
    expect(c).toContain("shadow-hud");
    expect(c).not.toContain("bg-forest-surface");
  });

  it("sizes rows for the board rather than the shell", () => {
    const c = menuItemClass(undefined, "hud");

    expect(c).toContain("text-body");
    expect(c).not.toContain("text-label");
  });
});
