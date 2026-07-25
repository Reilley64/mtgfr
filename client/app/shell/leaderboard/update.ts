import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { RpcClient } from "../../resources";
import { LeaderboardLoadFailed, type Message, ReceivedLeaderboardPage } from "./messages";
import type { LeaderboardSubmodel } from "./submodel";

const LEADERBOARD_PAGE_SIZE = 50;

export const FetchLeaderboard = Command.define(
  "FetchLeaderboard",
  { limit: S.Number, offset: S.Number },
  ReceivedLeaderboardPage,
  LeaderboardLoadFailed,
)(({ limit, offset }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.ratings.leaderboard({ limit, offset }).pipe(
      Effect.map((leaderboard) => ReceivedLeaderboardPage({ leaderboard, offset })),
      Effect.catch(() => Effect.succeed(LeaderboardLoadFailed({ message: "Could not load the leaderboard." }))),
    );
  }),
);

export function loadLeaderboard(
  model: LeaderboardSubmodel,
  offset = 0,
): readonly [LeaderboardSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const entries = offset === 0 ? [] : model.entries;
  return [
    { ...model, entries, error: null, status: "loading" },
    [FetchLeaderboard({ limit: LEADERBOARD_PAGE_SIZE, offset })],
  ];
}

export const update = (
  model: LeaderboardSubmodel,
  message: Message,
): readonly [LeaderboardSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<
      readonly [LeaderboardSubmodel, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]
    >(),
    M.tagsExhaustive({
      RequestedLeaderboardRefresh: () => loadLeaderboard(model),
      RequestedLeaderboardNextPage: () => loadLeaderboard(model, model.entries.length),
      ReceivedLeaderboardPage: ({ leaderboard, offset }) => {
        const entries = offset === 0 ? [...leaderboard.entries] : [...model.entries, ...leaderboard.entries];
        return [{ ...model, entries, error: null, status: "ready", total: leaderboard.total }, []];
      },
      LeaderboardLoadFailed: ({ message }) => [{ ...model, error: message, status: "error" }, []],
    }),
  );
