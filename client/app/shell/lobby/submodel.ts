import { Schema as S } from "effect";
import { LobbyView } from "../../../lib/lobby/types";

export const LobbySlice = S.Struct({
  tableId: S.NullOr(S.String),
  selectedDeckId: S.NullOr(S.Number),
  code: S.String,
  view: S.NullOr(LobbyView),
  started: S.Boolean,
  error: S.NullOr(S.String),
  copied: S.Boolean,
  clipboardFallback: S.Boolean,
  submitting: S.Boolean,
});
export type LobbySlice = typeof LobbySlice.Type;

export function initialLobbySlice(): LobbySlice {
  return {
    tableId: null,
    selectedDeckId: null,
    code: "",
    view: null,
    started: false,
    error: null,
    copied: false,
    clipboardFallback: false,
    submitting: false,
  };
}

export function enterLobby(
  model: LobbySlice,
  opts: { tableId: string | null; selectedDeckId: number | null },
): LobbySlice {
  if (model.tableId !== opts.tableId) {
    return {
      ...initialLobbySlice(),
      tableId: opts.tableId,
      selectedDeckId: opts.selectedDeckId ?? model.selectedDeckId,
    };
  }

  return {
    ...model,
    selectedDeckId: opts.selectedDeckId ?? model.selectedDeckId,
  };
}
