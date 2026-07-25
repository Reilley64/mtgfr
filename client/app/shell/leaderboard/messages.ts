import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { Leaderboard } from "../../../lib/wire/types";

export const RequestedLeaderboardRefresh = m("RequestedLeaderboardRefresh");
export const RequestedLeaderboardNextPage = m("RequestedLeaderboardNextPage");
export const ReceivedLeaderboardPage = m("ReceivedLeaderboardPage", { leaderboard: Leaderboard, offset: S.Number });
export const LeaderboardLoadFailed = m("LeaderboardLoadFailed", { message: S.String });

export const Message = S.Union([
  RequestedLeaderboardRefresh,
  RequestedLeaderboardNextPage,
  ReceivedLeaderboardPage,
  LeaderboardLoadFailed,
]);
export type Message = typeof Message.Type;
