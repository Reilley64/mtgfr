import { expect, test } from "vitest";
import { informRouteChanged } from "./inform";
import { initialLobbySlice } from "./submodel";

test("informRouteChanged resets join entry state when the play deck changes", () => {
  const model = {
    ...initialLobbySlice(),
    selectedDeckId: 7,
    code: "ABC123",
    error: "UnknownTable",
  };

  const [next, commands] = informRouteChanged(model, { tableId: null, selectedDeckId: 9 });

  expect(next.selectedDeckId).toBe(9);
  expect(next.code).toBe("");
  expect(next.error).toBeNull();
  expect(next).not.toHaveProperty("entryMode");
  expect(commands).toEqual([]);
});

test("informRouteChanged resets lobby state when the table changes", () => {
  const model = {
    ...initialLobbySlice(),
    tableId: "ABC123",
    selectedDeckId: 7,
    started: true,
    code: "ABC123",
    copied: true,
    clipboardFallback: true,
    submitting: true,
    error: "UnknownTable",
  };

  const [next, commands] = informRouteChanged(model, { tableId: "XYZ789", selectedDeckId: 9 });

  expect(next).toEqual({
    ...initialLobbySlice(),
    tableId: "XYZ789",
    selectedDeckId: 9,
  });
  expect(commands).toEqual([]);
});

test("informRouteChanged clears a stale selected deck when a table route carries no deck", () => {
  const model = {
    ...initialLobbySlice(),
    tableId: "ABC123",
    selectedDeckId: 7,
    code: "ABC123",
    started: true,
    copied: true,
    clipboardFallback: true,
    submitting: true,
    error: "UnknownTable",
  };

  const [next, commands] = informRouteChanged(model, { tableId: "XYZ789", selectedDeckId: null });

  expect(next).toEqual({
    ...initialLobbySlice(),
    tableId: "XYZ789",
    selectedDeckId: null,
  });
  expect(commands).toEqual([]);
});
