import { describe, expect, it } from "vitest";
import { filterOptionLabels } from "./optionFilter";

describe("filterOptionLabels", () => {
  it("keeps all options for an empty query", () => {
    expect(filterOptionLabels(["Elf", "Goblin", "Human"], "  ")).toEqual(["Elf", "Goblin", "Human"]);
  });

  it("filters case-insensitively by substring", () => {
    expect(filterOptionLabels(["Elf", "Goblin", "Human", "Merfolk"], "man")).toEqual(["Human"]);
    expect(filterOptionLabels(["Elf", "Goblin", "Human", "Merfolk"], "EL")).toEqual(["Elf"]);
  });
});
