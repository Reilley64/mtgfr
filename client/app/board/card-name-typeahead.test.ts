import * as Combobox from "@foldkit/ui/combobox";
import { expect, test } from "vitest";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { CompletedCancelSearchCardNames, GotCardNameComboboxMessage, PromptStringSet } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function state(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: { kind: "choose_card_name", player: 0, source: 1 },
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
        username: "Alice",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function fold(visible: VisibleState = state()): GameFoldState {
  return {
    seq: 1,
    state: visible,
    log: [],
    reject: null,
    provenance: {
      zoneMoves: new Map(),
      resolvedFromStack: new Set(),
      leftStackToPile: new Set(),
      battlefieldExits: new Map(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false, destroy: false, exile: false },
  };
}

// `@foldkit/ui/combobox` exports UpdatedInputValue as a type only, so the keystroke message is
// written out; every other combobox message here has a constructor.
const typedInput = (value: string): Combobox.Message => ({ _tag: "UpdatedInputValue", value });

function namingBoard() {
  return {
    ...initialBoardModel(),
    promptDraft: { kind: "string" as const, value: "" },
    pendingChoiceKey: "choose_card_name",
  };
}

test("typing in the typeahead fills the draft and searches the catalog", () => {
  const [model, cmds] = updateBoard(
    namingBoard(),
    GotCardNameComboboxMessage({ message: typedInput("Sol") }),
    fold(),
    "T1",
  );
  expect(model.cardNameCombobox.inputValue).toBe("Sol");
  expect(model.promptDraft).toEqual({ kind: "string", value: "Sol" });
  // The keystroke stops the search before it; the catalog search for "Sol" goes out when that
  // cancellation lands.
  expect(cmds.some((c) => (c as { name?: string }).name === "SearchCardNames.Interrupt")).toBe(true);
  const [, searchCmds] = updateBoard(model, CompletedCancelSearchCardNames(), fold(), "T1");
  expect(searchCmds).toEqual([expect.objectContaining({ name: "SearchCardNames", args: { query: "Sol" } })]);
});

test("picking a suggestion names that card", () => {
  const typed = updateBoard(namingBoard(), GotCardNameComboboxMessage({ message: typedInput("Sol") }), fold(), "T1")[0];
  const [picked] = updateBoard(
    typed,
    GotCardNameComboboxMessage({
      message: Combobox.SelectedItem({ item: "Sol Ring", displayText: "Sol Ring", wasSelected: false }),
    }),
    fold(),
    "T1",
  );
  expect(picked.promptDraft).toEqual({ kind: "string", value: "Sol Ring" });
  expect(picked.cardNameCombobox.isOpen).toBe(false);
});

test("PromptStringSet searches catalog names once the query is long enough", () => {
  const game = fold();
  const board = {
    ...initialBoardModel(),
    promptDraft: { kind: "string" as const, value: "" },
    pendingChoiceKey: "choose_card_name",
  };
  const [short] = updateBoard(board, PromptStringSet({ value: "S" }), game, "T1");
  expect(short.promptDraft).toEqual({ kind: "string", value: "S" });
  expect(short.cardNameSuggestions).toBeNull();
  // Too short to search: the in-flight search is still cancelled, and nothing replaces it.
  expect(updateBoard(short, CompletedCancelSearchCardNames(), game, "T1")[1]).toEqual([]);

  const [ready] = updateBoard(short, PromptStringSet({ value: "Sol" }), game, "T1");
  expect(ready.promptDraft).toEqual({ kind: "string", value: "Sol" });
  const [, readyCmds] = updateBoard(ready, CompletedCancelSearchCardNames(), game, "T1");
  expect((readyCmds[0] as { name?: string } | undefined)?.name).toBe("SearchCardNames");
});
