import { Schema as S, Stream } from "effect";
import { Subscription } from "foldkit";
import { type Message, ReceivedLobbyView } from "./messages";
import { lobbyPoll } from "./poll";
import type { LobbySlice } from "./submodel";

export const subscriptions = Subscription.make<LobbySlice, Message>()((entry) => ({
  lobbyPoll: entry(
    { tableId: S.NullOr(S.String), started: S.Boolean },
    {
      modelToDependencies: (model) => ({ tableId: model.tableId, started: model.started }),
      dependenciesToStream: ({ tableId, started }) => {
        if (tableId == null) return Stream.empty;
        if (started) return Stream.empty;
        return lobbyPoll(tableId).pipe(Stream.map((view) => ReceivedLobbyView({ view })));
      },
    },
  ),
}));
