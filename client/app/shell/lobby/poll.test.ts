import { Effect, Schedule, Stream } from "effect";
import { describe, expect, it } from "vitest";
import type { LobbyView } from "../../domain/lobby/types";
import { lobbyPoll } from "./poll";

const waiting: LobbyView = { table_id: "ABC123", started: false, seats: [], you: null, start_error: null, error: null };
const started: LobbyView = { table_id: "ABC123", started: true, seats: [], you: null, start_error: null, error: null };

describe("lobbyPoll", () => {
  it("skips null poll recoveries without emitting a lobby view", async () => {
    let calls = 0;
    const views: ReadonlyArray<LobbyView | null> = [null, waiting, null, started];

    const seen = await Effect.runPromise(
      lobbyPoll("ABC123", {
        fetchLobby: () =>
          Effect.fail(new Error("poll failed")).pipe(
            Effect.catch(() =>
              Effect.sync(() => {
                calls++;
                return views[calls - 1] ?? null;
              }),
            ),
          ),
        schedule: Schedule.recurs(10),
      }).pipe(Stream.runCollect),
    );

    expect(Array.from(seen)).toEqual([waiting, started]);
    expect(calls).toBe(4);
  });

  it("emits the started view and stops polling", async () => {
    let calls = 0;
    const views = [waiting, waiting, started, waiting];

    const seen = await Effect.runPromise(
      lobbyPoll("ABC123", {
        fetchLobby: () =>
          Effect.sync(() => {
            calls++;
            return views[calls - 1] ?? waiting;
          }),
        schedule: Schedule.recurs(10),
      }).pipe(Stream.runCollect),
    );

    expect(Array.from(seen)).toEqual([waiting, waiting, started]);
    expect(calls).toBe(3);
  });
});
