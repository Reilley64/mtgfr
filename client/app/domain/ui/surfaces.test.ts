import { describe, expect, it } from "vitest";
import { alertClass, fieldClass, listRowClass, modalClass, panelClass } from "./surfaces";

describe("surface classes", () => {
  it("panel is forest-surface with vine border and table shadow", () => {
    const c = panelClass();
    expect(c).toContain("bg-forest-surface");
    expect(c).toContain("border-vine");
    expect(c).toContain("shadow-table");
    expect(c).toContain("rounded-panel");
    expect(c).not.toContain("bg-black/20");
  });

  it("listRow uses glass-dim vine-dim with hover lift", () => {
    const c = listRowClass();
    expect(c).toContain("bg-glass-dim");
    expect(c).toContain("border-vine-dim");
    expect(c).toContain("hover:bg-white/8");
  });

  it("modal matches DESIGN.md modal recipe", () => {
    const c = modalClass();
    expect(c).toContain("rounded-modal");
    expect(c).toContain("bg-forest-surface");
    expect(c).toContain("shadow-table");
  });

  it("field is glass + vine control", () => {
    const c = fieldClass();
    expect(c).toContain("border-vine");
    expect(c).toContain("bg-glass");
    expect(c).toContain("rounded-control");
  });

  it("alert is a readable reconnect-rust stack", () => {
    const c = alertClass();
    expect(c).toContain("flex");
    expect(c).toContain("text-label");
    expect(c).toContain("text-reconnect-rust");
  });

  it("alert tone can override to burn-red", () => {
    expect(alertClass("text-burn-red")).toContain("text-burn-red");
    expect(alertClass("text-burn-red")).not.toContain("text-reconnect-rust");
  });
});
