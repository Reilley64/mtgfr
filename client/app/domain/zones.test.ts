import { describe, expect, it } from "vitest";
import { ZONE } from "./zones";

describe("wire zone discriminants", () => {
  it("maps the battlefield and stack to the engine's declaration order", () => {
    expect(ZONE.Battlefield).toBe(2);
    expect(ZONE.Stack).toBe(6);
  });
});
