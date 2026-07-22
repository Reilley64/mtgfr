import { describe, expect, it } from "vitest";
import { feltClass, fieldClass, listRowClass, modalClass, panelClass } from "./surfaces";

describe("surface classes", () => {
  it("panel is forest-surface with vine border and table shadow", () => {
    const c = panelClass();
    expect(c).toContain("bg-forest-surface");
    expect(c).toContain("border-vine");
    expect(c).toContain("shadow-table");
    expect(c).toContain("rounded-panel");
    expect(c).not.toContain("bg-black/20");
  });

  it("felt is forest-floor body snow", () => {
    expect(feltClass()).toContain("bg-forest-floor");
    expect(feltClass()).toContain("text-snow");
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
});
