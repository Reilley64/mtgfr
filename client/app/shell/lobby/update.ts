import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command } from "foldkit";
import { parseTableCode } from "../../domain/lobby/code";
import type { LobbyClientError } from "../../domain/lobby/errors";
import { lobbyIsHost } from "../../domain/lobby/seat";
import type { LobbyView } from "../../domain/lobby/types";
import { unlockTableAudio } from "../../domain/tableAudio";
import { LobbyClient } from "../../resources";
import { LobbyCopyCompleted, LobbyRequestFailed, LobbyTableCreated, type Message, ReceivedLobbyView } from "./messages";
import type { LobbySlice } from "./submodel";

const UNREACHABLE = "Unreachable";
const UNKNOWN_TABLE = "UnknownTable";

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

function lobbyFailure(error: LobbyClientError): typeof LobbyRequestFailed.Type {
  if (error._tag === "LobbyNotFound") return LobbyRequestFailed({ message: UNKNOWN_TABLE });
  return LobbyRequestFailed({ message: UNREACHABLE });
}

function selectedDeckId(model: LobbySlice): number | null {
  return model.selectedDeckId;
}

function tableForJoin(model: LobbySlice): string | null {
  if (model.tableId != null) return model.tableId;
  return parseTableCode(model.code);
}

function applyRouteChange(
  model: LobbySlice,
  route: { tableId: string | null; selectedDeckId: number | null },
): LobbySlice {
  if (model.tableId !== route.tableId) {
    return {
      tableId: route.tableId,
      selectedDeckId: route.selectedDeckId,
      code: "",
      entryMode: "choose",
      view: null,
      started: false,
      error: null,
      copied: false,
      clipboardFallback: false,
      submitting: false,
    };
  }

  if (model.tableId == null && route.selectedDeckId != null && model.selectedDeckId !== route.selectedDeckId) {
    return {
      tableId: null,
      selectedDeckId: route.selectedDeckId,
      code: "",
      entryMode: "choose",
      view: null,
      started: false,
      error: null,
      copied: false,
      clipboardFallback: false,
      submitting: false,
    };
  }

  return {
    ...model,
    selectedDeckId: route.selectedDeckId,
  };
}

export const CreateLobbyTable = Command.define(
  "CreateLobbyTable",
  LobbyTableCreated,
  LobbyRequestFailed,
)(
  Effect.gen(function* () {
    const lobby = yield* LobbyClient;
    return yield* lobby.createTable().pipe(
      Effect.map((created) => LobbyTableCreated({ tableId: created.table_id })),
      Effect.catch((error) => Effect.succeed(lobbyFailure(error))),
    );
  }),
);

export const JoinLobbyTable = Command.define(
  "JoinLobbyTable",
  { tableId: S.String, deckId: S.Number },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId, deckId }) =>
  Effect.gen(function* () {
    const lobby = yield* LobbyClient;
    return yield* lobby.joinTable(tableId, { deck_id: deckId }).pipe(
      Effect.map(viewResult),
      Effect.catch((error) => Effect.succeed(lobbyFailure(error))),
    );
  }),
);

export const ReadyLobby = Command.define(
  "ReadyLobby",
  { tableId: S.String, ready: S.Boolean },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId, ready }) =>
  Effect.sync(() => unlockTableAudio()).pipe(
    Effect.flatMap(() =>
      Effect.gen(function* () {
        const lobby = yield* LobbyClient;
        return yield* lobby.readyUp(tableId, { ready });
      }),
    ),
    Effect.map(viewResult),
    Effect.catch((error) => Effect.succeed(lobbyFailure(error))),
  ),
);

export const StartLobbyGame = Command.define(
  "StartLobbyGame",
  { tableId: S.String },
  ReceivedLobbyView,
  LobbyRequestFailed,
)(({ tableId }) =>
  Effect.gen(function* () {
    const lobby = yield* LobbyClient;
    return yield* lobby.startGame(tableId).pipe(
      Effect.map(viewResult),
      Effect.catch((error) => Effect.succeed(lobbyFailure(error))),
    );
  }),
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
): readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message, never, LobbyClient>>] {
  const tableId = tableForJoin(model);
  if (tableId == null) {
    return [{ ...model, error: "Enter the table code your host shared.", submitting: false }, []];
  }

  const deckId = selectedDeckId(model);
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
  _deckIds: ReadonlyArray<number>,
): readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message, never, LobbyClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [LobbySlice, ReadonlyArray<FoldkitCommand.Command<Message, never, LobbyClient>>]>(),
    M.tagsExhaustive({
      ChangedLobbyRoute: ({ tableId, selectedDeckId }) => [applyRouteChange(model, { tableId, selectedDeckId }), []],
      ChangedLobbyCode: ({ code }) => [{ ...model, code }, []],
      RequestedLobbyOpenJoin: () => [{ ...model, entryMode: "join" }, []],
      RequestedLobbyCancelJoin: () => [{ ...model, entryMode: "choose", code: "", error: null }, []],
      RequestedLobbyHost: () => {
        const deckId = selectedDeckId(model);
        if (deckId == null) {
          return [{ ...model, error: "Pick a deck to bring first." }, []];
        }
        return [{ ...model, selectedDeckId: deckId, error: null, submitting: true }, [CreateLobbyTable()]];
      },
      LobbyTableCreated: ({ tableId }) => joinCommand({ ...model, tableId }),
      RequestedLobbyJoin: () => joinCommand(model),
      RequestedLobbyReady: ({ ready }) => {
        if (model.tableId == null) return [model, []];
        unlockTableAudio();
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
