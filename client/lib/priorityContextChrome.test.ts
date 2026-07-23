import { describe, expect, it } from "vitest";
import { priorityPrimaryClass } from "~/priorityContextChrome";

describe("priorityPrimaryClass", () => {
  it("emphasizes the primary when this seat must act", () => {
    const c = priorityPrimaryClass(true);
    expect(c).toContain("shadow-glow");
    expect(c).toContain("min-w-[156px]");
  });

  it("leaves the disabled primary without act emphasis", () => {
    expect(priorityPrimaryClass(false)).toBe("");
  });
});
