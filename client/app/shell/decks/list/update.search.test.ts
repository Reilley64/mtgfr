import { describe, expect, it } from "vitest";
import { ChangedDeckListSearch, ClosedDeckListMenu, OpenedDeckListMenu } from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { update } from "./update";

describe("deck list search and menu update", () => {
  it("stores search query", () => {
    const [next] = update(initialDeckListSubmodel(), ChangedDeckListSearch({ query: "atraxa" }));
    expect(next.searchQuery).toBe("atraxa");
  });

  it("opens and closes context menu", () => {
    const [open] = update(initialDeckListSubmodel(), OpenedDeckListMenu({ deckId: 7, x: 10, y: 20 }));
    expect(open.contextMenu).toEqual({ deckId: 7, x: 10, y: 20 });
    const [closed] = update(open, ClosedDeckListMenu());
    expect(closed.contextMenu).toBeNull();
  });

  it("ignores OpenedDeckListMenu for precon ids", () => {
    const [next] = update(initialDeckListSubmodel(), OpenedDeckListMenu({ deckId: -1, x: 1, y: 2 }));
    expect(next.contextMenu).toBeNull();
  });
});
