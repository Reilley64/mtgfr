import * as Menu from "@foldkit/ui/menu";
import { Schema as S } from "effect";
import { type BoardModel, initialBoardModel } from "./board/submodel";
import { Me } from "./domain/wire/types";
import { emptyGameFold, type GameFoldState } from "./game/fold";
import { AppRoute } from "./routes";
import { AuthSubmodel } from "./shell/auth/submodel";
import { CoverageSubmodel } from "./shell/coverage/submodel";
import { DecksSubmodel } from "./shell/decks/submodel";
import { LeaderboardSubmodel } from "./shell/leaderboard/submodel";
import { LobbySlice } from "./shell/lobby/submodel";

export const SessionSlice = S.Struct({
  me: S.NullOr(Me),
  meGravatarHash: S.NullOr(S.String),
});
export type SessionSlice = typeof SessionSlice.Type;

export const LandscapeRotateSlice = S.Struct({ active: S.Boolean });
export type LandscapeRotateSlice = typeof LandscapeRotateSlice.Type;

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
    destroy: S.Boolean,
    exile: S.Boolean,
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
  /** One account menu for the whole shell — the avatar chrome is the same element on every route. */
  accountMenu: Menu.Model,
  route: AppRoute,
  currentPath: S.String,
  session: SessionSlice,
  sessionLoaded: S.Boolean,
  apiVersion: S.NullOr(S.String),
  faithfulCount: S.NullOr(S.Number),
  oracleTotal: S.NullOr(S.Number),
  auth: AuthSubmodel,
  decks: DecksSubmodel,
  leaderboard: LeaderboardSubmodel,
  coverage: CoverageSubmodel,
  lobby: LobbySlice,
  game: S.NullOr(GameSlice),
  landscapeRotate: LandscapeRotateSlice,
});
type ModelFromSchema = typeof Model.Type;
export type Model = Omit<ModelFromSchema, "game"> & { game: GameSlice | null };
