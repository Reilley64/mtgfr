import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
import { toString as urlToString } from "foldkit/url";
import type { Message as BoardMessage } from "./board/messages";
import { syncBoardWithGame, updateBoard } from "./board/submodel";
import { applyDeltaPure, applySnapshotPure, type DeltaEnvelope, setRejectPure } from "./game/fold";
import { type Message, NavigationCompleted, type ReceivedDelta } from "./messages";
import { emptyGameSlice, type GameSlice, type Model } from "./model";
import type { RpcClient } from "./resources";
import { isProtectedRoute, nextFromUrl, pathWithSearch, routeFromUrl, routePath, safeNext, TableRoute } from "./routes";
import { initialAuthSubmodel } from "./shell/auth/submodel";
import { update as updateAuth } from "./shell/auth/update";
import type { Message as BuilderMessage } from "./shell/decks/builder/messages";
import { enterBuilder, update as updateBuilder } from "./shell/decks/builder/update";
import type { Message as ListMessage } from "./shell/decks/list/messages";
import { loadDeckList, update as updateDeckList } from "./shell/decks/list/update";
import type { Message as LobbyMessage } from "./shell/lobby/messages";
import { enterLobby } from "./shell/lobby/submodel";
import { update as updateLobby } from "./shell/lobby/update";

const Redirect = Command.define(
  "Redirect",
  { path: S.String },
  NavigationCompleted,
)(({ path }) => Navigation.replaceUrl(path).pipe(Effect.as(NavigationCompleted())));

const PushUrl = Command.define(
  "PushUrl",
  { url: S.String },
  NavigationCompleted,
)(({ url }) => Navigation.pushUrl(url).pipe(Effect.as(NavigationCompleted())));

const LoadExternalUrl = Command.define(
  "LoadExternalUrl",
  { href: S.String },
  NavigationCompleted,
)(({ href }) => Navigation.load(href).pipe(Effect.as(NavigationCompleted())));

function loginRedirectFor(model: Model): string {
  return `/login?next=${encodeURIComponent(model.currentPath)}`;
}

function deckFromCurrentPath(currentPath: string): number | null {
  const search = currentPath.split("?")[1];
  if (search == null) return null;
  const value = new URLSearchParams(search).get("deck");
  if (value == null) return null;
  const id = Number(value);
  return Number.isInteger(id) ? id : null;
}

function terminalStreamError(status: number): string {
  if (status === 401) return "Session expired — sign in again.";
  return `Lost connection to the table (${status}).`;
}

function mergeGameFold(game: GameSlice, folded: ReturnType<typeof applyDeltaPure>): GameSlice {
  const next = { ...game, ...folded };
  return { ...next, board: syncBoardWithGame(next.board, next) };
}

function deltaEnvelope(message: typeof ReceivedDelta.Type): DeltaEnvelope {
  return {
    seq: message.seq,
    state: message.state,
    events: [...message.events],
    auto_actions: message.auto_actions == null ? undefined : [...message.auto_actions],
  };
}

function sessionCommands(model: Model): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  if (!model.sessionLoaded) return [];

  if (model.session.me == null && isProtectedRoute(model.route)) {
    return [Redirect({ path: loginRedirectFor(model) })];
  }

  if (model.session.me != null && model.route._tag === "LoginRoute") {
    return [Redirect({ path: safeNext(model.auth.next) })];
  }

  return [];
}

function routeEntry(model: Model): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const authCommands = sessionCommands(model);
  if (authCommands.length > 0) return [model, authCommands];
  if (!model.sessionLoaded || model.session.me == null) return [model, []];

  switch (model.route._tag) {
    case "HomeRoute": {
      const [list, commands] = loadDeckList(model.decks.list);
      return [{ ...model, decks: { ...model.decks, list } }, commands];
    }
    case "NewDeckRoute": {
      const [builder, commands] = enterBuilder(null);
      return [{ ...model, decks: { ...model.decks, builder } }, commands];
    }
    case "DeckRoute": {
      const [builder, commands] = enterBuilder(model.route.id);
      return [{ ...model, decks: { ...model.decks, builder } }, commands];
    }
    case "PlayRoute": {
      const [list, commands] = loadDeckList(model.decks.list);
      return [
        {
          ...model,
          decks: { ...model.decks, list },
          lobby: enterLobby(model.lobby, { tableId: null, selectedDeckId: deckFromCurrentPath(model.currentPath) }),
          game: null,
        },
        commands,
      ];
    }
    case "TableRoute": {
      const [list, commands] = loadDeckList(model.decks.list);
      return [
        {
          ...model,
          decks: { ...model.decks, list },
          lobby: enterLobby(model.lobby, {
            tableId: model.route.table,
            selectedDeckId: deckFromCurrentPath(model.currentPath),
          }),
          game: null,
        },
        commands,
      ];
    }
    default:
      return [model, []];
  }
}

function foldDeckList(
  model: Model,
  message: ListMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [list, commands] = updateDeckList(model.decks.list, message);
  return [{ ...model, decks: { ...model.decks, list } }, commands];
}

function foldDeckBuilder(
  model: Model,
  message: BuilderMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [builder, commands] = updateBuilder(model.decks.builder, message);
  return [{ ...model, decks: { ...model.decks, builder } }, commands];
}

function foldBoard(
  model: Model,
  message: BoardMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  if (model.game == null) return [model, []];
  const [board, commands] = updateBoard(model.game.board, message, model.game, model.game.tableId);
  return [{ ...model, game: { ...model.game, board } }, commands];
}

function foldLobby(
  model: Model,
  message: LobbyMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const deckIds = model.decks.list.decks.map((deck) => deck.id);
  const [lobby, commands] = updateLobby(model.lobby, message, deckIds);
  const game =
    lobby.started && lobby.tableId != null
      ? model.game?.tableId === lobby.tableId
        ? { ...model.game, active: true }
        : emptyGameSlice(lobby.tableId)
      : model.game;
  const redirect =
    model.route._tag === "PlayRoute" && lobby.tableId != null
      ? [
          Redirect({
            path:
              lobby.selectedDeckId != null
                ? `${routePath(TableRoute({ table: lobby.tableId }))}?deck=${lobby.selectedDeckId}`
                : routePath(TableRoute({ table: lobby.tableId })),
          }),
        ]
      : [];
  return [{ ...model, lobby, game }, [...commands, ...redirect]];
}

export const update = (
  model: Model,
  message: Message,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      Booted: () => [model, []],
      ReceivedApiVersion: ({ version }) => [{ ...model, apiVersion: version }, []],
      UrlChanged: ({ url }) => {
        const nextModel = {
          ...model,
          route: routeFromUrl(url),
          currentPath: pathWithSearch(url),
          auth: { ...model.auth, next: nextFromUrl(url) },
        };
        return routeEntry(nextModel);
      },
      UrlRequested: ({ request }) =>
        M.value(request).pipe(
          M.withReturnType<readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
          M.tagsExhaustive({
            Internal: ({ url }) => [model, [PushUrl({ url: urlToString(url) })]],
            External: ({ href }) => [model, [LoadExternalUrl({ href })]],
          }),
        ),
      NavigationCompleted: () => [model, []],
      PortraitGateChanged: ({ open }) => [{ ...model, portraitGate: { open } }, []],
      PortraitGateCancelled: () => [model, []],
      CompletedPortraitGateModal: () => [model, []],
      ModalOpened: () => [model, []],
      CardArtTick: () => [model, []],
      ArtLoaded: (boardMessage) => foldBoard(model, boardMessage),
      BoardPointerDown: (boardMessage) => foldBoard(model, boardMessage),
      BoardPointerMove: (boardMessage) => foldBoard(model, boardMessage),
      BoardPointerUp: (boardMessage) => foldBoard(model, boardMessage),
      FlightsSynced: (boardMessage) => foldBoard(model, boardMessage),
      HandActionActivated: (boardMessage) => foldBoard(model, boardMessage),
      HandDragStarted: (boardMessage) => foldBoard(model, boardMessage),
      HandDragMoved: (boardMessage) => foldBoard(model, boardMessage),
      HandDragEnded: (boardMessage) => foldBoard(model, boardMessage),
      HandActionHovered: (boardMessage) => foldBoard(model, boardMessage),
      PrimaryClicked: (boardMessage) => foldBoard(model, boardMessage),
      PassClicked: (boardMessage) => foldBoard(model, boardMessage),
      KeepHandClicked: (boardMessage) => foldBoard(model, boardMessage),
      MulliganClicked: (boardMessage) => foldBoard(model, boardMessage),
      StackYieldArmed: (boardMessage) => foldBoard(model, boardMessage),
      TurnYieldToggled: (boardMessage) => foldBoard(model, boardMessage),
      CancelActionClicked: (boardMessage) => foldBoard(model, boardMessage),
      CommanderCastClicked: (boardMessage) => foldBoard(model, boardMessage),
      TargetChosen: (boardMessage) => foldBoard(model, boardMessage),
      ModalModesChosen: (boardMessage) => foldBoard(model, boardMessage),
      ModalTargetChosen: (boardMessage) => foldBoard(model, boardMessage),
      XDraftSet: (boardMessage) => foldBoard(model, boardMessage),
      XSubmitted: (boardMessage) => foldBoard(model, boardMessage),
      SacrificeChosen: (boardMessage) => foldBoard(model, boardMessage),
      DiscardChosen: (boardMessage) => foldBoard(model, boardMessage),
      GyExileChosen: (boardMessage) => foldBoard(model, boardMessage),
      CombatAttackerDropped: (boardMessage) => foldBoard(model, boardMessage),
      CombatBlockerDropped: (boardMessage) => foldBoard(model, boardMessage),
      CombatCancelAttacker: (boardMessage) => foldBoard(model, boardMessage),
      CombatCancelBlocker: (boardMessage) => foldBoard(model, boardMessage),
      PendingChoiceAnswered: (boardMessage) => foldBoard(model, boardMessage),
      PromptCardToggled: (boardMessage) => foldBoard(model, boardMessage),
      PromptSubmitted: (boardMessage) => foldBoard(model, boardMessage),
      PromptDeclined: (boardMessage) => foldBoard(model, boardMessage),
      PromptOrderMoved: (boardMessage) => foldBoard(model, boardMessage),
      PromptDamageSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptModeChoiceToggled: (boardMessage) => foldBoard(model, boardMessage),
      PromptPartitionSet: (boardMessage) => foldBoard(model, boardMessage),
      ModalModeToggled: (boardMessage) => foldBoard(model, boardMessage),
      StackDwellChanged: (boardMessage) => foldBoard(model, boardMessage),
      StackExpandClicked: (boardMessage) => foldBoard(model, boardMessage),
      StackCollapseClicked: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeArmed: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeReleased: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeHovered: (boardMessage) => foldBoard(model, boardMessage),
      RadialOptionPicked: (boardMessage) => foldBoard(model, boardMessage),
      RadialDismissed: (boardMessage) => foldBoard(model, boardMessage),
      AltDown: (boardMessage) => foldBoard(model, boardMessage),
      AltUp: (boardMessage) => foldBoard(model, boardMessage),
      InspectAuxHovered: (boardMessage) => foldBoard(model, boardMessage),
      InspectCardFetched: (boardMessage) => foldBoard(model, boardMessage),
      InspectFlipFace: (boardMessage) => foldBoard(model, boardMessage),
      InspectDismissed: (boardMessage) => foldBoard(model, boardMessage),
      PileExpanded: (boardMessage) => foldBoard(model, boardMessage),
      PileOverlayClosed: (boardMessage) => foldBoard(model, boardMessage),
      ConcedeClicked: (boardMessage) => foldBoard(model, boardMessage),
      ConcedeCancelled: (boardMessage) => foldBoard(model, boardMessage),
      ConcedeConfirmed: (boardMessage) => foldBoard(model, boardMessage),
      ResultSeen: (boardMessage) => foldBoard(model, boardMessage),
      LeaveGame: () => {
        const path = "/";
        return [model, [Redirect({ path })]];
      },
      KeyboardSpacePressed: (boardMessage) => foldBoard(model, boardMessage),
      KeyboardEnterPressed: (boardMessage) => foldBoard(model, boardMessage),
      KeyboardEscape: (boardMessage) => foldBoard(model, boardMessage),
      HintDismissed: (boardMessage) => foldBoard(model, boardMessage),
      HintAutoHidden: (boardMessage) => foldBoard(model, boardMessage),
      SoundToggled: (boardMessage) => foldBoard(model, boardMessage),
      PriorityElapsed: (boardMessage) => foldBoard(model, boardMessage),
      LegendToggled: (boardMessage) => foldBoard(model, boardMessage),
      ChangedAuthMode: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      ChangedAuthEmail: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      ChangedAuthUsername: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      ChangedAuthPassword: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      SubmittedAuth: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      RequestedLogout: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      ReceivedMe: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        const nextModel = {
          ...model,
          session: { me: authMessage.me },
          sessionLoaded: true,
          auth: authMessage.me == null ? auth : initialAuthSubmodel(model.auth.next),
        };
        const [routeModel, routeCommands] = routeEntry(nextModel);
        return [routeModel, [...commands, ...routeCommands]];
      },
      AuthFailed: (authMessage) => {
        const [auth, commands] = updateAuth(model.auth, authMessage);
        return [{ ...model, auth }, commands];
      },
      RequestedDecksRefresh: (decksMessage) => foldDeckList(model, decksMessage),
      ReceivedDecks: (decksMessage) => foldDeckList(model, decksMessage),
      DecksLoadFailed: (decksMessage) => foldDeckList(model, decksMessage),
      ReceivedDeckListCommanders: (decksMessage) => foldDeckList(model, decksMessage),
      MovedDeckListHover: (decksMessage) => foldDeckList(model, decksMessage),
      ClearedDeckListHover: (decksMessage) => foldDeckList(model, decksMessage),
      AskedDeckDelete: (decksMessage) => foldDeckList(model, decksMessage),
      CancelledDeckDelete: (decksMessage) => foldDeckList(model, decksMessage),
      RequestedDeckDelete: (decksMessage) => foldDeckList(model, decksMessage),
      DeckDeleted: (decksMessage) => foldDeckList(model, decksMessage),
      DeckDeleteFailed: (decksMessage) => foldDeckList(model, decksMessage),
      ChangedBuilderName: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ChangedBuilderQuery: (decksMessage) => foldDeckBuilder(model, decksMessage),
      RequestedNextBuilderPage: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ReceivedBuilderSearchPage: (decksMessage) => foldDeckBuilder(model, decksMessage),
      BuilderSearchFailed: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ReceivedDeckForBuilder: (decksMessage) => foldDeckBuilder(model, decksMessage),
      DeckBuilderLoadFailed: (decksMessage) => foldDeckBuilder(model, decksMessage),
      HydratedBuilderCards: (decksMessage) => foldDeckBuilder(model, decksMessage),
      AddedBuilderCard: (decksMessage) => foldDeckBuilder(model, decksMessage),
      RemovedBuilderCard: (decksMessage) => foldDeckBuilder(model, decksMessage),
      SetBuilderCommander: (decksMessage) => foldDeckBuilder(model, decksMessage),
      OpenedBuilderPrintPicker: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ReceivedBuilderPrints: (decksMessage) => foldDeckBuilder(model, decksMessage),
      BuilderPrintSearchFailed: (decksMessage) => foldDeckBuilder(model, decksMessage),
      PickedBuilderPrint: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ClosedBuilderPrintPicker: (decksMessage) => foldDeckBuilder(model, decksMessage),
      SubmittedDeckSave: (decksMessage) => foldDeckBuilder(model, decksMessage),
      DeckSaved: (decksMessage) => foldDeckBuilder(model, decksMessage),
      DeckSaveFailed: (decksMessage) => foldDeckBuilder(model, decksMessage),
      MovedBuilderHover: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ClearedBuilderHover: (decksMessage) => foldDeckBuilder(model, decksMessage),
      OpenedBuilderMenu: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ClosedBuilderMenu: (decksMessage) => foldDeckBuilder(model, decksMessage),
      RanBuilderMenuAction: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ActivatedBuilderTarget: (decksMessage) => foldDeckBuilder(model, decksMessage),
      RequestedBuilderCancel: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ConfirmedBuilderDiscard: (decksMessage) => foldDeckBuilder(model, decksMessage),
      CancelledBuilderDiscard: (decksMessage) => foldDeckBuilder(model, decksMessage),
      NavigatedAwayFromBuilder: (decksMessage) => foldDeckBuilder(model, decksMessage),
      ChangedLobbyDeck: (lobbyMessage) => foldLobby(model, lobbyMessage),
      ChangedLobbyCode: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyHost: (lobbyMessage) => foldLobby(model, lobbyMessage),
      LobbyTableCreated: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyJoin: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyReady: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyStart: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyCopy: (lobbyMessage) => foldLobby(model, lobbyMessage),
      LobbyCopyCompleted: (lobbyMessage) => foldLobby(model, lobbyMessage),
      ReceivedLobbyView: (lobbyMessage) => foldLobby(model, lobbyMessage),
      LobbyRequestFailed: (lobbyMessage) => foldLobby(model, lobbyMessage),
      ReceivedSnapshot: ({ seq, state }) => {
        if (model.game == null) return [model, []];
        return [{ ...model, game: mergeGameFold(model.game, applySnapshotPure(model.game, seq, state)) }, []];
      },
      ReceivedDelta: (message) => {
        if (model.game == null) return [model, []];
        return [{ ...model, game: mergeGameFold(model.game, applyDeltaPure(model.game, deltaEnvelope(message))) }, []];
      },
      StreamStatus: ({ connected }) => {
        if (model.game == null) return [model, []];
        return [{ ...model, game: { ...model.game, connected } }, []];
      },
      StreamTerminalError: ({ status }) => {
        if (model.game == null) return [model, []];
        const rejected = setRejectPure(model.game, terminalStreamError(status));
        return [{ ...model, game: { ...model.game, ...rejected, connected: false } }, []];
      },
      IntentAcked: () => {
        if (model.game == null) return [model, []];
        return [
          {
            ...model,
            game: { ...model.game, reject: null, board: { ...model.game.board, reject: null } },
          },
          [],
        ];
      },
      IntentRejected: ({ reason }) => {
        if (model.game == null) return [model, []];
        const rejected = setRejectPure(model.game, reason);
        return [
          {
            ...model,
            game: { ...model.game, ...rejected, board: { ...model.game.board, reject: reason } },
          },
          [],
        ];
      },
    }),
  );
