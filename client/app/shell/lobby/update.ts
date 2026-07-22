import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { createTable, joinTable, readyUp, startGame } from "../../../lib/lobby/client";
import { parseTableCode } from "../../../lib/lobby/code";
import { lobbyIsHost } from "../../../lib/lobby/seat";
import type { LobbyView } from "../../../lib/lobby/types";
import { unlockTableAudio } from "../../../lib/tableAudio";
import { LobbyCopyCompleted, LobbyRequestFailed, LobbyTableCreated, type Message, ReceivedLobbyView } from "./messages";
import type { LobbySlice } from "./submodel";

const UNREACHABLE = "Unreachable";

function viewError(view: LobbyView): string | null {
  return view.error ?? null;
}

function applyView(model: LobbySlice, view: LobbyView): LobbySlice {
  return {
    ...model,
    tableId: view.table_id,
    view,
    started: view.started,
    error: viewError(view),
    submitting: false,
  };
}

function viewResult(view: LobbyView | null): typeof ReceivedLobbyView.Type | typeof LobbyRequestFailed.Type {
  return view == null ? LobbyRequestFailed({ message: UNREACHABLE }) : ReceivedLobbyView({ view });
}

function selectedDeckId(model: LobbySlice, deckIds: ReadonlyArray<number>): number | null {
  if (model.selectedDeckId != null) return model.selectedDeckId;
  return deckIds[0] ?? null;
}

function tableForJoin(model: LobbySlice): string | null {
  if (model.tableId != null) return model.tableId;
  return parseTableCode(model.code);
}

export const CreateLobbyTable = Command.define(
  "CreateLobbyTable",
  LobbyTableCreated,
  LobbyRequestFailed,
)(
  Effect.tryPromise(() => createTable()).pipe(
    Effect.map((created) =>
      created == null ? LobbyRequestFailed({ message: UNREACHABLE }) : LobbyTableCreated({ tableId: created.table_id }),
    ),
    Effect.catch(() => Effect.succeed(LobbyRequestFailed({ message: UNREACHABLE }))),
  ),
);

export const JoinLobbyTable = Command.define(
  "JoinLobbyTable",
  { tableId: S.String, deckId: S.Number },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId, deckId }) =>
  Effect.tryPromise(() => joinTable({ table_id: tableId, deck_id: deckId })).pipe(
    Effect.map(viewResult),
    Effect.catch(() => Effect.succeed(LobbyRequestFailed({ message: UNREACHABLE }))),
  ),
);

export const ReadyLobby = Command.define(
  "ReadyLobby",
  { tableId: S.String, ready: S.Boolean },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId, ready }) =>
  Effect.sync(() => unlockTableAudio()).pipe(
    Effect.flatMap(() => Effect.tryPromise(() => readyUp({ table_id: tableId, ready }))),
    Effect.map(viewResult),
    Effect.catch(() => Effect.succeed(LobbyRequestFailed({ message: UNREACHABLE }))),
  ),
);

export const StartLobbyGame = Command.define(
  "StartLobbyGame",
  { tableId: S.String },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId }) =>
  Effect.tryPromise(() => startGame({ table_id: tableId })).pipe(
    Effect.map(viewResult),
    Effect.catch(() => Effect.succeed(LobbyRequestFailed({ message: UNREACHABLE }))),
  ),
);

export const CopyLobbyLink = Command.define(
  "CopyLobbyLink",
  { tableId: S.String },
  LobbyCopyCompleted,
)(({ tableId }) =>
  Effect.tryPromise(() => navigator.clipboard.writeText(tableId)).pipe(
    Effect.as(LobbyCopyCompleted({ ok: true })),
    Effect.catch(() => Effect.succeed(LobbyCopyCompleted({ ok: false }))),
  ),
);

function joinCommand(
  model: LobbySlice,
  deckIds: ReadonlyArray<number>,
): readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message>>] {
  const tableId = tableForJoin(model);
  if (tableId == null) {
    return [{ ...model, error: "Enter the table code your host shared.", submitting: false }, []];
  }

  const deckId = selectedDeckId(model, deckIds);
  if (deckId == null) {
    return [{ ...model, tableId, error: "Pick a deck to bring first.", submitting: false }, []];
  }

  return [
    { ...model, tableId, selectedDeckId: deckId, error: null, submitting: true },
    [JoinLobbyTable({ tableId, deckId })],
  ];
}

export const update = (
  model: LobbySlice,
  message: Message,
  deckIds: ReadonlyArray<number>,
): readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message>>]>(),
    M.tagsExhaustive({
      ChangedLobbyDeck: ({ deckId }) => [{ ...model, selectedDeckId: deckId }, []],
      ChangedLobbyCode: ({ code }) => [{ ...model, code }, []],
      RequestedLobbyHost: () => {
        const deckId = selectedDeckId(model, deckIds);
        if (deckId == null) {
          return [{ ...model, error: "Pick a deck to bring first." }, []];
        }
        return [{ ...model, selectedDeckId: deckId, error: null, submitting: true }, [CreateLobbyTable()]];
      },
      LobbyTableCreated: ({ tableId }) => joinCommand({ ...model, tableId }, deckIds),
      RequestedLobbyJoin: () => joinCommand(model, deckIds),
      RequestedLobbyReady: ({ ready }) => {
        if (model.tableId == null) return [model, []];
        return [{ ...model, error: null, submitting: true }, [ReadyLobby({ tableId: model.tableId, ready })]];
      },
      RequestedLobbyStart: () => {
        if (model.tableId == null) return [model, []];
        return [{ ...model, error: null, submitting: true }, [StartLobbyGame({ tableId: model.tableId })]];
      },
      RequestedLobbyCopy: () => {
        if (model.tableId == null) return [model, []];
        return [model, [CopyLobbyLink({ tableId: model.tableId })]];
      },
      LobbyCopyCompleted: ({ ok }) => [{ ...model, copied: ok, clipboardFallback: !ok }, []],
      ReceivedLobbyView: ({ view }) => [applyView(model, view), []],
      LobbyRequestFailed: ({ message }) => [{ ...model, error: message, submitting: false }, []],
    }),
  );

export function lobbyReady(model: LobbySlice): boolean {
  const you = model.view?.you ?? null;
  if (you == null) return false;
  return model.view?.seats[you]?.ready ?? false;
}

export function lobbyHost(model: LobbySlice): boolean {
  return lobbyIsHost(model.view?.you, model.view?.seats);
}
