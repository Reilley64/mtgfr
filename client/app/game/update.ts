import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import {
  type OutMessage as BoardOutMessage,
  drainPlayModeIfSingleton,
  dropHeldSeeds,
  syncBoardWithGame,
} from "../board/submodel";
import { type Message as AppMessage, GotBoardMessage, GotGameMessage } from "../messages";
import type { GameSlice } from "../model";
import type { RpcClient } from "../resources";
import { applyDeltaPure, applySnapshotPure, type DeltaEnvelope, setRejectPure } from "./fold";
import type { Message } from "./messages";

function terminalStreamError(status: number): string {
  if (status === 401) return "Session expired — sign in again.";
  if (status === 404) return "Table no longer available.";
  return `Lost connection to the table (${status}).`;
}

function mergeGameFold(
  game: GameSlice,
  folded: ReturnType<typeof applyDeltaPure>,
): readonly [GameSlice, ReadonlyArray<FoldkitCommand.Command<BoardOutMessage, never, RpcClient>>] {
  const next = { ...game, ...folded };
  const synced = { ...next, board: syncBoardWithGame(next.board, next) };
  const [board, commands] = drainPlayModeIfSingleton(synced.board, synced, synced.tableId);
  return [{ ...synced, board }, commands];
}

function deltaEnvelope(message: Extract<Message, { _tag: "ReceivedDelta" }>): DeltaEnvelope {
  return {
    seq: message.seq,
    state: message.state,
    events: [...message.events],
    auto_actions: message.auto_actions == null ? undefined : [...message.auto_actions],
  };
}

function toAppMessage(message: BoardOutMessage): AppMessage {
  switch (message._tag) {
    case "ReceivedSnapshot":
    case "ReceivedDelta":
    case "StreamStatus":
    case "StreamTerminalError":
    case "IntentAcked":
    case "IntentRejected":
      return GotGameMessage({ message });
    default:
      return GotBoardMessage({ message });
  }
}

function mapBoardCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<BoardOutMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<AppMessage, never, RpcClient>> {
  return Command.mapMessages(commands, toAppMessage);
}

export function updateGame(
  game: GameSlice,
  message: Message,
): readonly [GameSlice, ReadonlyArray<FoldkitCommand.Command<AppMessage, never, RpcClient>>] {
  switch (message._tag) {
    case "ReceivedSnapshot": {
      const [next, commands] = mergeGameFold(game, applySnapshotPure(game, message.seq, message.state));
      return [next, mapBoardCommands(commands)];
    }
    case "ReceivedDelta": {
      const [next, commands] = mergeGameFold(game, applyDeltaPure(game, deltaEnvelope(message)));
      return [next, mapBoardCommands(commands)];
    }
    case "StreamStatus":
      return [{ ...game, connected: message.connected }, []];
    case "StreamTerminalError": {
      const rejected = setRejectPure(game, terminalStreamError(message.status));
      return [{ ...game, ...rejected, connected: false }, []];
    }
    case "IntentAcked":
      return [{ ...game, reject: null, board: { ...game.board, reject: null } }, []];
    case "IntentRejected": {
      const rejected = setRejectPure(game, message.reason);
      return [
        {
          ...game,
          ...rejected,
          // Re-enable the frozen prompt draft so the player can correct and resubmit.
          // Also clear optimistic combat confirm latches — a rejected goad/empty declare
          // must not leave staging disabled for the rest of the step.
          board: {
            // The play never happened: drop the optimistic flight seed and unhide its hand tile.
            ...dropHeldSeeds(game.board),
            reject: message.reason,
            promptSubmitInFlight: false,
            promptSubmitSeq: null,
            attackersConfirmed: false,
            blockersConfirmed: false,
          },
        },
        [],
      ];
    }
    default: {
      const exhaustive: never = message;
      return exhaustive;
    }
  }
}
