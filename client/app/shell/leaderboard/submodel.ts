import { Schema as S } from "effect";
import { LeaderboardEntry } from "../../../lib/wire/types";

export const LeaderboardStatus = S.Union([
  S.Literal("idle"),
  S.Literal("loading"),
  S.Literal("ready"),
  S.Literal("error"),
]);
export type LeaderboardStatus = typeof LeaderboardStatus.Type;

export const LeaderboardSubmodel = S.Struct({
  entries: S.Array(LeaderboardEntry),
  total: S.Number,
  status: LeaderboardStatus,
  error: S.NullOr(S.String),
});
export type LeaderboardSubmodel = typeof LeaderboardSubmodel.Type;

export function initialLeaderboardSubmodel(): LeaderboardSubmodel {
  return {
    entries: [],
    total: 0,
    status: "idle",
    error: null,
  };
}
