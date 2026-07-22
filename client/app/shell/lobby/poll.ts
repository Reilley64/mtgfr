import { Effect, Schedule, Stream } from "effect";
import { lobbyState } from "../../../lib/lobby/client";
import type { LobbyView } from "../../../lib/lobby/types";

type LobbyPollOptions = {
  fetchLobby?: (tableId: string) => Effect.Effect<LobbyView | null, never>;
  schedule?: Schedule.Schedule<unknown>;
};

function fetchLobbyState(tableId: string): Effect.Effect<LobbyView | null, never> {
  return Effect.tryPromise(() => lobbyState(tableId)).pipe(Effect.catch(() => Effect.succeed(null)));
}

export function lobbyPoll(tableId: string, options: LobbyPollOptions = {}): Stream.Stream<LobbyView> {
  const fetchLobby = options.fetchLobby ?? fetchLobbyState;
  const schedule = options.schedule ?? Schedule.spaced("1 second");

  return Stream.fromEffectSchedule(fetchLobby(tableId), schedule).pipe(
    Stream.filter((view): view is LobbyView => view != null),
    Stream.takeUntil((view) => view.started),
  );
}
