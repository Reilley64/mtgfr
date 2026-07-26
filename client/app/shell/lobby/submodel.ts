import { Schema as S } from "effect";
import { LobbyView } from "../../domain/lobby/types";

export const LobbyEntryMode = S.Union([S.Literal("choose"), S.Literal("join")]);
export type LobbyEntryMode = typeof LobbyEntryMode.Type;

export const LobbySlice = S.Struct({
  tableId: S.NullOr(S.String),
  selectedDeckId: S.NullOr(S.Number),
  code: S.String,
  entryMode: LobbyEntryMode,
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
    entryMode: "choose",
    view: null,
    started: false,
    error: null,
    copied: false,
    clipboardFallback: false,
    submitting: false,
  };
}
