import { Story } from "foldkit";
import { expect, test } from "vitest";
import { update as appUpdate, init } from "../../main-exports";
import { GotLeaderboardMessage } from "../../messages";
import { ReceivedLeaderboardPage, RequestedLeaderboardRefresh } from "./messages";
import { FetchLeaderboard } from "./update";

test("GotLeaderboardMessage updates the leaderboard through the parent update", () => {
  const [model] = init();
  const load = FetchLeaderboard({ limit: 50, offset: 0 });
  const page = ReceivedLeaderboardPage({
    leaderboard: { entries: [{ rank: 1, rating: 1200, user_id: 1, username: "alice" }], total: 1 },
    offset: 0,
  });

  Story.story(
    appUpdate,
    Story.given({
      ...model,
      leaderboard: {
        ...model.leaderboard,
        entries: [{ rank: 2, rating: 1188, user_id: 2, username: "bob" }],
        error: "Could not load the leaderboard.",
        status: "error",
        total: 2,
      },
    }),
    Story.message(GotLeaderboardMessage({ message: RequestedLeaderboardRefresh() })),
    Story.Command.expectExact(load),
    Story.model((next) => {
      expect(next.leaderboard.entries).toEqual([]);
      expect(next.leaderboard.error).toBeNull();
      expect(next.leaderboard.status).toBe("loading");
    }),
    Story.Command.resolve(load, page),
    Story.model((next) => {
      expect(next.leaderboard.entries).toEqual([{ rank: 1, rating: 1200, user_id: 1, username: "alice" }]);
      expect(next.leaderboard.status).toBe("ready");
    }),
  );
});
