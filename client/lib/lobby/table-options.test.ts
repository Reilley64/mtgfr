import { describe, expect, it } from "vitest";
import { parseTableOptionsBody } from "./table-options";

describe("parseTableOptionsBody", () => {
  it("rejects missing commander_damage_enabled", () => {
    expect(parseTableOptionsBody({ table_id: "ABC" })).toBe("BadJson");
  });

  it("rejects non-boolean commander_damage_enabled", () => {
    expect(parseTableOptionsBody({ table_id: "ABC", commander_damage_enabled: "false" })).toBe("BadJson");
    expect(parseTableOptionsBody({ table_id: "ABC", commander_damage_enabled: 0 })).toBe("BadJson");
  });

  it("accepts a boolean commander_damage_enabled", () => {
    expect(parseTableOptionsBody({ table_id: "ABC", commander_damage_enabled: false })).toEqual({
      tableId: "ABC",
      commanderDamageEnabled: false,
    });
    expect(parseTableOptionsBody({ table_id: "ABC", commander_damage_enabled: true })).toEqual({
      tableId: "ABC",
      commanderDamageEnabled: true,
    });
  });
});
