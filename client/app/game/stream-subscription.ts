import { Effect, Queue, Schema as S, Stream } from "effect";
import { Subscription } from "foldkit";
import { type Client, client as defaultClient } from "~/effect/client";
import { streamDeltas as streamDeltasEffect } from "~/effect/stream";
import type { StreamFrame } from "../../lib/wire/types";
import type { Message as AppMessage } from "../messages";
import type { Model } from "../model";
import {
  type Message as GameMessage,
  ReceivedDelta,
  ReceivedSnapshot,
  StreamStatus,
  StreamTerminalError,
} from "./messages";

export type { StreamCallbacks } from "~/effect/stream";
export const streamDeltas = streamDeltasEffect;

function frameToMessage(frame: Exclude<StreamFrame, { frame: "heartbeat" }>): GameMessage {
  if (frame.frame === "snapshot") {
    return ReceivedSnapshot({ seq: frame.seq, state: frame.state });
  }

  return ReceivedDelta({
    seq: frame.seq,
    state: frame.state,
    events: frame.events,
    auto_actions: frame.auto_actions,
  });
}

export function streamMessages(
  table: string,
  random: () => number = Math.random,
  client: Client = defaultClient,
): Stream.Stream<GameMessage> {
  return Stream.callback<GameMessage>((queue) =>
    streamDeltas(
      table,
      {
        onFrame: (frame) => Queue.offerUnsafe(queue, frameToMessage(frame)),
        onStatus: (connected) => Queue.offerUnsafe(queue, StreamStatus({ connected })),
        onError: (status) => Queue.offerUnsafe(queue, StreamTerminalError({ status })),
      },
      random,
      client,
    ).pipe(Effect.asVoid),
  );
}

export const subscriptions = Subscription.make<Model, AppMessage>()((entry) => ({
  gameStream: entry(
    { table: S.NullOr(S.String), gameTable: S.NullOr(S.String), active: S.Boolean },
    {
      modelToDependencies: (model) => {
        const table = model.route._tag === "TableRoute" ? model.route.table : null;
        return {
          table,
          gameTable: model.game?.tableId ?? null,
          active: model.game?.active ?? false,
        };
      },
      dependenciesToStream: ({ table, gameTable, active }) => {
        if (table == null) return Stream.empty;
        if (!active) return Stream.empty;
        if (gameTable !== table) return Stream.empty;
        return streamMessages(table);
      },
    },
  ),
}));
