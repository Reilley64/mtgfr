import { describe, expect, it } from "vitest";
import { cardPickIsSearchable, dedupeChoiceItems, filterChoiceItems, searchableChoiceItems } from "./cardPickSearch";
import type { ChoiceItem } from "./wire/types";

function item(id: number, label: string): ChoiceItem {
  return { id, label };
}

describe("cardPickIsSearchable", () => {
  it("is true only for search_library", () => {
    expect(cardPickIsSearchable("search_library")).toBe(true);
    expect(cardPickIsSearchable("scry")).toBe(false);
    expect(cardPickIsSearchable("choose_dredge")).toBe(false);
  });
});

describe("dedupeChoiceItems", () => {
  it("keeps the first item per face label", () => {
    expect(dedupeChoiceItems([item(1, "Forest"), item(2, "Forest"), item(3, "Island")])).toEqual([
      item(1, "Forest"),
      item(3, "Island"),
    ]);
  });
});

describe("filterChoiceItems", () => {
  it("keeps all items for an empty query", () => {
    const items = [item(1, "Sol Ring"), item(2, "Mana Crypt")];
    expect(filterChoiceItems(items, "  ")).toEqual(items);
  });

  it("filters by case-insensitive substring on the label", () => {
    const items = [item(1, "Sol Ring"), item(2, "Mana Crypt"), item(3, "Sol Talisman")];
    expect(filterChoiceItems(items, "sol")).toEqual([item(1, "Sol Ring"), item(3, "Sol Talisman")]);
  });
});

describe("searchableChoiceItems", () => {
  it("dedupes then filters for pick-one library search", () => {
    const items = [item(1, "Forest"), item(2, "Forest"), item(3, "Island")];
    expect(searchableChoiceItems(items, "for")).toEqual([item(1, "Forest")]);
  });
});
