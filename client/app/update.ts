import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
import { toString as urlToString } from "foldkit/url";
import type { Message as BoardMessage } from "./board/messages";
import { drainPlayModeIfSingleton, syncBoardWithGame, updateBoard } from "./board/submodel";
import { captureDeckCardFlipForNav } from "./deck-card-nav";
import { parseDeckIdParam, playDeckAccess } from "./deck-id";
import { gravatarHash } from "./domain/gravatar";
import { applyDeltaPure, applySnapshotPure, type DeltaEnvelope, setRejectPure } from "./game/fold";
import {
  GotAuthMessage,
  GotDeckBuilderMessage,
  GotDeckListMessage,
  type Message,
  NavigationCompleted,
  type ReceivedDelta,
  ReceivedMeGravatarHash,
} from "./messages";
import { emptyGameSlice, type GameSlice, type Model } from "./model";
import type { RpcClient } from "./resources";
import {
  isProtectedRoute,
  NotFoundRoute,
  nextFromUrl,
  normalizeAppRoute,
  pathWithSearch,
  routeFromUrl,
  routePath,
  safeNext,
  TableRoute,
} from "./routes";
import * as Auth from "./shell/auth";
import type { Message as AuthMessage } from "./shell/auth/messages";
import * as DeckBuilder from "./shell/decks/builder";
import type { Message as BuilderMessage } from "./shell/decks/builder/messages";
import * as DeckList from "./shell/decks/list";
import type { Message as ListMessage } from "./shell/decks/list/messages";
import type { Message as LeaderboardMessage } from "./shell/leaderboard/messages";
import { loadLeaderboard, update as updateLeaderboard } from "./shell/leaderboard/update";
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

export const HashMeGravatar = Command.define(
  "HashMeGravatar",
  { email: S.String },
  ReceivedMeGravatarHash,
)(({ email }) =>
  Effect.promise(() => gravatarHash(email)).pipe(Effect.map((hash) => ReceivedMeGravatarHash({ email, hash }))),
);

function loginRedirectFor(model: Model): string {
  return `/login?next=${encodeURIComponent(model.currentPath)}`;
}

function terminalStreamError(status: number): string {
  if (status === 401) return "Session expired — sign in again.";
  if (status === 404) return "Table no longer available.";
  return `Lost connection to the table (${status}).`;
}

function mergeGameFold(
  game: GameSlice,
  folded: ReturnType<typeof applyDeltaPure>,
): readonly [GameSlice, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const next = { ...game, ...folded };
  const synced = { ...next, board: syncBoardWithGame(next.board, next) };
  const [board, commands] = drainPlayModeIfSingleton(synced.board, synced, synced.tableId);
  return [{ ...synced, board }, commands];
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
      const [list, commands] = DeckList.loadDeckList(model.decks.list);
      return [{ ...model, decks: { ...model.decks, list } }, mapDeckListCommands(commands)];
    }
    case "LeaderboardRoute": {
      const [leaderboard, commands] = loadLeaderboard(model.leaderboard);
      return [{ ...model, leaderboard }, commands];
    }
    case "NewDeckRoute": {
      const [builder, commands] = DeckBuilder.enterBuilder(null);
      return [{ ...model, decks: { ...model.decks, builder } }, mapDeckBuilderCommands(commands)];
    }
    case "DeckRoute": {
      const [builder, commands] = DeckBuilder.enterBuilder(model.route.id);
      return [{ ...model, decks: { ...model.decks, builder } }, mapDeckBuilderCommands(commands)];
    }
    case "PlayRoute": {
      const [list, commands] = DeckList.loadDeckList(model.decks.list);
      return [
        {
          ...model,
          decks: { ...model.decks, list },
          lobby: enterLobby(model.lobby, { tableId: null, selectedDeckId: parseDeckIdParam(model.route.deckId) }),
          game: null,
        },
        mapDeckListCommands(commands),
      ];
    }
    case "TableRoute": {
      const [list, commands] = DeckList.loadDeckList(model.decks.list);
      return [
        {
          ...model,
          decks: { ...model.decks, list },
          lobby: enterLobby(model.lobby, {
            tableId: model.route.table,
            selectedDeckId: parseDeckIdParam(model.route.deckId),
          }),
          game: null,
        },
        mapDeckListCommands(commands),
      ];
    }
    case "LoginRoute":
    case "NotFoundRoute":
      return [model, []];
    default: {
      const exhaustive: never = model.route;
      return exhaustive;
    }
  }
}

function mapDeckListCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<ListMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, (message) => GotDeckListMessage({ message }));
}

function foldDeckList(
  model: Model,
  message: ListMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [list, commands] = DeckList.update(model.decks.list, message);
  return [{ ...model, decks: { ...model.decks, list } }, mapDeckListCommands(commands)];
}

function notFoundWhenPlayDeckMissing(model: Model): Model {
  if (model.route._tag !== "PlayRoute" && model.route._tag !== "TableRoute") return model;
  const deckId = parseDeckIdParam(model.route.deckId);
  const access = playDeckAccess(deckId, model.decks.list.decks, model.decks.list.loading, model.decks.list.error);
  if (access !== "missing") return model;
  return { ...model, route: NotFoundRoute({ path: model.currentPath }) };
}

function foldDeckBuilder(
  model: Model,
  message: BuilderMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [builder, commands] = DeckBuilder.update(model.decks.builder, message);
  return [{ ...model, decks: { ...model.decks, builder } }, mapDeckBuilderCommands(commands)];
}

function mapDeckBuilderCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<BuilderMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, (message) => GotDeckBuilderMessage({ message }));
}

function foldLeaderboard(
  model: Model,
  message: LeaderboardMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [leaderboard, commands] = updateLeaderboard(model.leaderboard, message);
  return [{ ...model, leaderboard }, commands];
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
    model.route._tag === "PlayRoute" && lobby.tableId != null && lobby.selectedDeckId != null
      ? [
          Redirect({
            path: routePath(TableRoute({ deckId: String(lobby.selectedDeckId), table: lobby.tableId })),
          }),
        ]
      : [];
  return [{ ...model, lobby, game }, [...commands, ...redirect]];
}

function foldAuth(
  model: Model,
  message: AuthMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [auth, commands] = Auth.update(model.auth, message);
  const mappedCommands = Command.mapMessages(commands, (child) => GotAuthMessage({ message: child }));

  if (message._tag !== "ReceivedMe") {
    return [{ ...model, auth }, mappedCommands];
  }

  const nextModel = {
    ...model,
    session: { me: message.me, meGravatarHash: null },
    sessionLoaded: true,
    auth: message.me == null ? auth : Auth.Model.initialAuthSubmodel(model.auth.next),
  };
  const [routeModel, routeCommands] = routeEntry(nextModel);
  const gravatarCommands = message.me == null ? [] : [HashMeGravatar({ email: message.me.email })];
  return [routeModel, [...mappedCommands, ...routeCommands, ...gravatarCommands]];
}

export const update = (
  model: Model,
  message: Message,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] =>
  M.value(message).pipe(
    M.withReturnType<readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
    M.tagsExhaustive({
      Booted: () => [model, []],
      ReceivedApiVersion: ({ version, faithfulCount, oracleTotal }) => [
        { ...model, apiVersion: version, faithfulCount, oracleTotal },
        [],
      ],
      UrlChanged: ({ url }) => {
        const currentPath = pathWithSearch(url);
        const nextModel = {
          ...model,
          route: normalizeAppRoute(routeFromUrl(url), currentPath),
          currentPath,
          auth: { ...model.auth, next: nextFromUrl(url) },
        };
        return routeEntry(nextModel);
      },
      UrlRequested: ({ request }) =>
        M.value(request).pipe(
          M.withReturnType<readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>]>(),
          M.tagsExhaustive({
            Internal: ({ url }) => {
              const href = urlToString(url);
              captureDeckCardFlipForNav(model.currentPath, href);
              return [model, [PushUrl({ url: href })]];
            },
            External: ({ href }) => [model, [LoadExternalUrl({ href })]],
          }),
        ),
      NavigationCompleted: () => [model, []],
      PortraitGateChanged: ({ open }) => [{ ...model, portraitGate: { open } }, []],
      PortraitGateCancelled: () => [model, []],
      CompletedPortraitGateModal: () => [model, []],
      ReceivedMeGravatarHash: ({ email, hash }) => {
        if (model.session.me?.email !== email) return [model, []];
        return [{ ...model, session: { ...model.session, meGravatarHash: hash } }, []];
      },
      ModalOpened: () => [model, []],
      CardArtTick: () => [model, []],
      DeckCardFlipTick: () => [model, []],
      ArtLoaded: (boardMessage) => foldBoard(model, boardMessage),
      BoardCameraZoomed: (boardMessage) => foldBoard(model, boardMessage),
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
      PlayModeChosen: (boardMessage) => foldBoard(model, boardMessage),
      CommanderCastClicked: (boardMessage) => foldBoard(model, boardMessage),
      TargetChosen: (boardMessage) => foldBoard(model, boardMessage),
      ModalModesChosen: (boardMessage) => foldBoard(model, boardMessage),
      ModalTargetChosen: (boardMessage) => foldBoard(model, boardMessage),
      XDraftSet: (boardMessage) => foldBoard(model, boardMessage),
      XSubmitted: (boardMessage) => foldBoard(model, boardMessage),
      SacrificeChosen: (boardMessage) => foldBoard(model, boardMessage),
      DiscardChosen: (boardMessage) => foldBoard(model, boardMessage),
      GyExileChosen: (boardMessage) => foldBoard(model, boardMessage),
      GyExileConfirmed: (boardMessage) => foldBoard(model, boardMessage),
      DiscardCostConfirmed: (boardMessage) => foldBoard(model, boardMessage),
      PileCardClicked: (boardMessage) => foldBoard(model, boardMessage),
      CombatAttackerDropped: (boardMessage) => foldBoard(model, boardMessage),
      CombatBlockerDropped: (boardMessage) => foldBoard(model, boardMessage),
      CombatCancelAttacker: (boardMessage) => foldBoard(model, boardMessage),
      CombatCancelBlocker: (boardMessage) => foldBoard(model, boardMessage),
      PendingChoiceAnswered: (boardMessage) => foldBoard(model, boardMessage),
      PromptCardToggled: (boardMessage) => foldBoard(model, boardMessage),
      PromptSubmitted: (boardMessage) => foldBoard(model, boardMessage),
      PromptDeclined: (boardMessage) => foldBoard(model, boardMessage),
      PromptOrderMoved: (boardMessage) => foldBoard(model, boardMessage),
      PromptOrderRowClicked: (boardMessage) => foldBoard(model, boardMessage),
      PromptOrderDragEnded: (boardMessage) => foldBoard(model, boardMessage),
      PromptDamageSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptStringSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptCardFilterSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptOptionFilterSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptNumberSet: (boardMessage) => foldBoard(model, boardMessage),
      PromptModeChoiceToggled: (boardMessage) => foldBoard(model, boardMessage),
      PromptPartitionSet: (boardMessage) => foldBoard(model, boardMessage),
      ModalModeToggled: (boardMessage) => foldBoard(model, boardMessage),
      StackDwellChanged: (boardMessage) => foldBoard(model, boardMessage),
      StackExpandClicked: (boardMessage) => foldBoard(model, boardMessage),
      StackCollapseClicked: (boardMessage) => foldBoard(model, boardMessage),
      LogExpandToggled: (boardMessage) => foldBoard(model, boardMessage),
      LogCopyRequested: (boardMessage) => foldBoard(model, boardMessage),
      LogCopyCompleted: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeArmed: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeReleased: (boardMessage) => foldBoard(model, boardMessage),
      RadialWedgeHovered: (boardMessage) => foldBoard(model, boardMessage),
      RadialOptionPicked: (boardMessage) => foldBoard(model, boardMessage),
      RadialDismissed: (boardMessage) => foldBoard(model, boardMessage),
      AltDown: (boardMessage) => foldBoard(model, boardMessage),
      AltUp: (boardMessage) => foldBoard(model, boardMessage),
      InspectAuxHovered: (boardMessage) => foldBoard(model, boardMessage),
      InspectCardFetched: (boardMessage) => foldBoard(model, boardMessage),
      CardNameSuggestionsFetched: (boardMessage) => foldBoard(model, boardMessage),
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
      GotAuthMessage: ({ message }) => foldAuth(model, message),
      GotDeckListMessage: ({ message }) => {
        const [nextModel, commands] = foldDeckList(model, message);
        if (message._tag !== "ReceivedDecks") return [nextModel, commands];
        return [notFoundWhenPlayDeckMissing(nextModel), commands];
      },
      GotDeckBuilderMessage: ({ message }) => foldDeckBuilder(model, message),
      ToggledAccountMenu: () => {
        if (model.route._tag === "HomeRoute") {
          const list = model.decks.list;
          return [
            {
              ...model,
              decks: {
                ...model.decks,
                list: {
                  ...list,
                  accountMenuOpen: !list.accountMenuOpen,
                  contextMenu: null,
                },
              },
            },
            [],
          ];
        }
        if (model.route._tag === "LeaderboardRoute") {
          return [
            {
              ...model,
              leaderboard: {
                ...model.leaderboard,
                accountMenuOpen: !model.leaderboard.accountMenuOpen,
              },
            },
            [],
          ];
        }
        return [model, []];
      },
      ClosedAccountMenu: () => {
        if (model.route._tag === "HomeRoute") {
          return [
            {
              ...model,
              decks: {
                ...model.decks,
                list: { ...model.decks.list, accountMenuOpen: false },
              },
            },
            [],
          ];
        }
        if (model.route._tag === "LeaderboardRoute") {
          return [
            {
              ...model,
              leaderboard: { ...model.leaderboard, accountMenuOpen: false },
            },
            [],
          ];
        }
        return [model, []];
      },
      RequestedLeaderboardRefresh: (leaderboardMessage) => foldLeaderboard(model, leaderboardMessage),
      RequestedLeaderboardNextPage: (leaderboardMessage) => foldLeaderboard(model, leaderboardMessage),
      ReceivedLeaderboardPage: (leaderboardMessage) => foldLeaderboard(model, leaderboardMessage),
      LeaderboardLoadFailed: (leaderboardMessage) => foldLeaderboard(model, leaderboardMessage),
      ChangedLobbyCode: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyHost: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyOpenJoin: (lobbyMessage) => foldLobby(model, lobbyMessage),
      RequestedLobbyCancelJoin: (lobbyMessage) => foldLobby(model, lobbyMessage),
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
        const [game, commands] = mergeGameFold(model.game, applySnapshotPure(model.game, seq, state));
        return [{ ...model, game }, commands];
      },
      ReceivedDelta: (message) => {
        if (model.game == null) return [model, []];
        const [game, commands] = mergeGameFold(model.game, applyDeltaPure(model.game, deltaEnvelope(message)));
        return [{ ...model, game }, commands];
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
            game: {
              ...model.game,
              ...rejected,
              // Re-enable the frozen prompt draft so the player can correct and resubmit.
              board: { ...model.game.board, reject: reason, promptSubmitInFlight: false },
            },
          },
          [],
        ];
      },
    }),
  );
