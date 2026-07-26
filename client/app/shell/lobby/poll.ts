import { Effect, Schedule, Stream } from "effect";
import { client as lobbyHttp } from "../../domain/lobby/client";
import type { LobbyView } from "../../domain/lobby/types";

type LobbyPollOptions = {
  fetchLobby?: (tableId: string) => Effect.Effect<LobbyView | null, never>;
  schedule?: Schedule.Schedule<unknown>;
};

function fetchLobbyState(tableId: string): Effect.Effect<LobbyView | null, never> {
  return lobbyHttp.lobbyState(tableId).pipe(Effect.catch(() => Effect.succeed(null)));
}

export function lobbyPoll(tableId: string, options: LobbyPollOptions = {}): Stream.Stream<LobbyView> {
  const fetchLobby: Effect.Effect<LobbyView | null, never> =
    options.fetchLobby == null ? fetchLobbyState(tableId) : options.fetchLobby(tableId);
  const schedule = options.schedule ?? Schedule.spaced("1 second");

  return Stream.fromEffectSchedule(fetchLobby, schedule).pipe(
    Stream.filter((view): view is LobbyView => view != null),
    Stream.takeUntil((view) => view.started),
  );
}
