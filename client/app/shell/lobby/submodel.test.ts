import { expect, test } from "vitest";
import { enterLobby, initialLobbySlice } from "./submodel";

test("enterLobby resets join entry state when the play deck changes", () => {
  const model = {
    ...initialLobbySlice(),
    selectedDeckId: 7,
    entryMode: "join" as const,
    code: "ABC123",
    error: "UnknownTable",
  };

  const next = enterLobby(model, { tableId: null, selectedDeckId: 9 });

  expect(next.selectedDeckId).toBe(9);
  expect(next.entryMode).toBe("choose");
  expect(next.code).toBe("");
  expect(next.error).toBeNull();
});
