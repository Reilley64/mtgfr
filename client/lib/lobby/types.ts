import { Schema as S } from "effect";

export const SeatView = S.Struct({
  player: S.Number,
  claimed: S.Boolean,
  username: S.NullOr(S.String),
  deck_name: S.NullOr(S.String),
  deck_id: S.NullOr(S.Number),
  ready: S.Boolean,
  is_host: S.Boolean,
  is_you: S.Boolean,
});
export type SeatView = typeof SeatView.Type;

export const LobbyView = S.Struct({
  table_id: S.String,
  seats: S.Array(SeatView),
  you: S.NullOr(S.Number),
  started: S.Boolean,
  start_error: S.optional(S.NullOr(S.String)),
  error: S.optional(S.NullOr(S.String)),
});
export type LobbyView = typeof LobbyView.Type;
