import { Schema as S } from "effect";
import { Me } from "../lib/wire/types";
import { type BoardModel, initialBoardModel } from "./board/submodel";
import { emptyGameFold, type GameFoldState } from "./game/fold";
import { AppRoute } from "./routes";
import { AuthSubmodel } from "./shell/auth/submodel";
import { DecksSubmodel } from "./shell/decks/submodel";
import { LobbySlice } from "./shell/lobby/submodel";

export const SessionSlice = S.Struct({ me: S.NullOr(Me) });
export type SessionSlice = typeof SessionSlice.Type;

export const PortraitGateSlice = S.Struct({ open: S.Boolean });
export type PortraitGateSlice = typeof PortraitGateSlice.Type;

export type GameSlice = GameFoldState & {
  active: boolean;
  board: BoardModel;
  tableId: string | null;
  connected: boolean;
};

export const GameSlice = S.Struct({
  active: S.Boolean,
  board: S.Any,
  tableId: S.NullOr(S.String),
  connected: S.Boolean,
  seq: S.Number,
  state: S.NullOr(S.Any),
  log: S.Array(S.Struct({ seq: S.Number, text: S.String, auto: S.optional(S.Boolean) })),
  reject: S.NullOr(S.String),
  provenance: S.Any,
  tableFeel: S.Struct({
    land: S.Boolean,
    stack: S.Boolean,
    resolve: S.Boolean,
    damage: S.Boolean,
  }),
});

export function emptyGameSlice(tableId: string | null = null): GameSlice {
  return {
    ...emptyGameFold(),
    active: tableId != null,
    board: initialBoardModel(),
    tableId,
    connected: true,
  };
}

export const Model = S.Struct({
  ready: S.Boolean,
  route: AppRoute,
  currentPath: S.String,
  session: SessionSlice,
  sessionLoaded: S.Boolean,
  apiVersion: S.NullOr(S.String),
  auth: AuthSubmodel,
  decks: DecksSubmodel,
  lobby: LobbySlice,
  game: S.NullOr(GameSlice),
  portraitGate: PortraitGateSlice,
});
type ModelFromSchema = typeof Model.Type;
export type Model = Omit<ModelFromSchema, "game"> & { game: GameSlice | null };
