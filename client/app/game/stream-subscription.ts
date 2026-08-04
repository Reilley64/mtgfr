import { Schema as S, Stream } from "effect";
import { Subscription } from "foldkit";
import { type Client, client as defaultClient } from "~/effect/client";
import { type GameFrame, type GameStreamEvent, streamDeltas as streamDeltasEffect } from "~/effect/stream";
import type { Model } from "../model";
import {
  type Message as GameMessage,
  ReceivedDelta,
  ReceivedSnapshot,
  StreamStatus,
  StreamTerminalError,
} from "./messages";

export type { GameStreamEvent } from "~/effect/stream";
export const streamDeltas = streamDeltasEffect;

function frameToMessage(frame: GameFrame): GameMessage {
  if (frame.frame === "snapshot") {
    return ReceivedSnapshot({ seq: frame.seq, state: frame.state, card_text: frame.card_text ?? [] });
  }

  return ReceivedDelta({
    seq: frame.seq,
    state: frame.state,
    events: frame.events,
    auto_actions: frame.auto_actions,
  });
}

function eventToMessage(event: GameStreamEvent): GameMessage {
  switch (event.kind) {
    case "frame":
      return frameToMessage(event.frame);
    case "status":
      return StreamStatus({ connected: event.connected });
    case "terminal-error":
      return StreamTerminalError({ status: event.status });
    default: {
      const _exhaustive: never = event;
      return _exhaustive;
    }
  }
}

export function streamMessages(
  table: string,
  random: () => number = Math.random,
  client: Client = defaultClient,
): Stream.Stream<GameMessage> {
  return streamDeltas(table, random, client).pipe(Stream.map(eventToMessage));
}

export const subscriptions = Subscription.make<Model, GameMessage>()((entry) => ({
  gameStream: entry(
    { table: S.NullOr(S.String), gameTable: S.NullOr(S.String), active: S.Boolean },
    {
      modelToDependencies: (model) => {
        const table =
          model.route._tag === "PregameTableRoute" || model.route._tag === "GameTableRoute" ? model.route.table : null;
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
