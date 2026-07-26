import { Effect, Match as M, Schema as S } from "effect";
import type { Command as FoldkitCommand } from "foldkit";
import { Command, Navigation } from "foldkit";
import { toString as urlToString } from "foldkit/url";
import type { Message as BoardMessage } from "./board/messages";
import { type OutMessage as BoardOutMessage, updateBoard } from "./board/submodel";
import { captureDeckCardFlipForNav } from "./deck-card-nav";
import { parseDeckIdParam, playDeckAccess } from "./deck-id";
import { gravatarHash } from "./domain/gravatar";
import { updateGame } from "./game";
import {
  GotAuthMessage,
  GotBoardMessage,
  GotCoverageMessage,
  GotDeckBuilderMessage,
  GotDeckListMessage,
  GotGameMessage,
  GotLeaderboardMessage,
  GotLobbyMessage,
  type Message,
  NavigationCompleted,
  ReceivedMeGravatarHash,
} from "./messages";
import { emptyGameSlice, type Model } from "./model";
import type { RpcClient } from "./resources";
import {
  GameTableRoute,
  isProtectedRoute,
  NotFoundRoute,
  nextFromUrl,
  normalizeAppRoute,
  PregameTableRoute,
  pathWithSearch,
  routeFromUrl,
  routePath,
  safeNext,
} from "./routes";
import * as Auth from "./shell/auth";
import type { Message as AuthMessage } from "./shell/auth/messages";
import * as Coverage from "./shell/coverage";
import type { Message as CoverageMessage } from "./shell/coverage/messages";
import * as DeckBuilder from "./shell/decks/builder";
import type { Message as BuilderMessage } from "./shell/decks/builder/messages";
import * as DeckList from "./shell/decks/list";
import type { Message as ListMessage } from "./shell/decks/list/messages";
import * as Leaderboard from "./shell/leaderboard";
import type { Message as LeaderboardMessage } from "./shell/leaderboard/messages";
import * as Lobby from "./shell/lobby";
import type { Message as LobbyMessage } from "./shell/lobby/messages";

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

function toAppBoardMessage(message: BoardOutMessage): Message {
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
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, toAppBoardMessage);
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

function enterDeckListRoute(
  model: Model,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [list, commands] = DeckList.informRouteChanged(model.decks.list);
  return [{ ...model, decks: { ...model.decks, list } }, mapDeckListCommands(commands)];
}

function enterLeaderboardRoute(
  model: Model,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [leaderboard, commands] = Leaderboard.informRouteChanged(model.leaderboard);
  return [{ ...model, leaderboard }, mapLeaderboardCommands(commands)];
}

function enterCoverageRoute(
  model: Model,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [coverage, commands] = Coverage.informRouteChanged(model.coverage);
  return [{ ...model, coverage }, mapCoverageCommands(commands)];
}

function enterDeckBuilderRoute(
  model: Model,
  editingId: string | null,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [builder, commands] = DeckBuilder.informRouteChanged(model.decks.builder, editingId);
  return [{ ...model, decks: { ...model.decks, builder } }, mapDeckBuilderCommands(commands)];
}

function enterLobbyRoute(
  model: Model,
  args: { tableId: string | null; selectedDeckId: number | null },
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [list, deckListCommands] = DeckList.informRouteChanged(model.decks.list);
  const [lobby, lobbyCommands] = Lobby.informRouteChanged(model.lobby, args);
  return [
    {
      ...model,
      decks: { ...model.decks, list },
      game: null,
      lobby,
    },
    [...mapDeckListCommands(deckListCommands), ...mapLobbyCommands(lobbyCommands)],
  ];
}

function gameSliceForTableRoute(model: Model, tableId: string) {
  if (model.game?.tableId === tableId) {
    return { ...model.game, active: true };
  }

  return emptyGameSlice(tableId);
}

function enterGameTableRoute(
  model: Model,
  tableId: string,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [list, deckListCommands] = DeckList.informRouteChanged(model.decks.list);
  const [lobby, lobbyCommands] = Lobby.informRouteChanged(model.lobby, {
    tableId,
    selectedDeckId: null,
  });
  return [
    {
      ...model,
      decks: { ...model.decks, list },
      game: gameSliceForTableRoute(model, tableId),
      lobby,
    },
    [...mapDeckListCommands(deckListCommands), ...mapLobbyCommands(lobbyCommands)],
  ];
}

function routeEntry(model: Model): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const authCommands = sessionCommands(model);
  if (authCommands.length > 0) return [model, authCommands];
  if (!model.sessionLoaded || model.session.me == null) return [model, []];

  switch (model.route._tag) {
    case "HomeRoute":
      return enterDeckListRoute(model);
    case "LeaderboardRoute":
      return enterLeaderboardRoute(model);
    case "CoverageRoute":
      return enterCoverageRoute(model);
    case "NewDeckRoute":
      return enterDeckBuilderRoute(model, null);
    case "DeckRoute":
      return enterDeckBuilderRoute(model, model.route.id);
    case "PlayRoute":
      return enterLobbyRoute(model, {
        tableId: null,
        selectedDeckId: parseDeckIdParam(model.route.deckId),
      });
    case "PregameTableRoute":
      return enterLobbyRoute(model, {
        tableId: model.route.table,
        selectedDeckId: parseDeckIdParam(model.route.deckId),
      });
    case "GameTableRoute":
      return enterGameTableRoute(model, model.route.table);
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
  if (model.route._tag !== "PlayRoute" && model.route._tag !== "PregameTableRoute") return model;
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
  const [leaderboard, commands] = Leaderboard.update(model.leaderboard, message);
  return [{ ...model, leaderboard }, mapLeaderboardCommands(commands)];
}

function foldCoverage(
  model: Model,
  message: CoverageMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const [coverage, commands] = Coverage.update(model.coverage, message);
  return [{ ...model, coverage }, mapCoverageCommands(commands)];
}

function foldBoard(
  model: Model,
  message: BoardMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  if (message._tag === "LeaveGame") {
    const path = "/";
    return [model, [Redirect({ path })]];
  }
  if (model.game == null) return [model, []];
  const [board, commands] = updateBoard(model.game.board, message, model.game, model.game.tableId);
  return [{ ...model, game: { ...model.game, board } }, mapBoardCommands(commands)];
}

function foldLobby(
  model: Model,
  message: LobbyMessage,
): readonly [Model, ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>>] {
  const deckIds = model.decks.list.decks.map((deck) => deck.id);
  const [lobby, commands] = Lobby.update(model.lobby, message, deckIds);
  const game =
    lobby.started && lobby.tableId != null
      ? model.game?.tableId === lobby.tableId
        ? { ...model.game, active: true }
        : emptyGameSlice(lobby.tableId)
      : model.game;
  const redirectPath =
    lobby.tableId == null
      ? null
      : lobby.started
        ? model.route._tag === "PlayRoute" || model.route._tag === "PregameTableRoute"
          ? routePath(GameTableRoute({ table: lobby.tableId }))
          : null
        : model.route._tag === "PlayRoute" && lobby.selectedDeckId != null
          ? routePath(PregameTableRoute({ deckId: String(lobby.selectedDeckId), table: lobby.tableId }))
          : null;
  const redirect = redirectPath == null ? [] : [Redirect({ path: redirectPath })];
  return [{ ...model, lobby, game }, [...mapLobbyCommands(commands), ...redirect]];
}

function mapLeaderboardCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<LeaderboardMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, (message) => GotLeaderboardMessage({ message }));
}

function mapCoverageCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<CoverageMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, (message) => GotCoverageMessage({ message }));
}

function mapLobbyCommands(
  commands: ReadonlyArray<FoldkitCommand.Command<LobbyMessage, never, RpcClient>>,
): ReadonlyArray<FoldkitCommand.Command<Message, never, RpcClient>> {
  return Command.mapMessages(commands, (message) => GotLobbyMessage({ message }));
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
      LandscapeRotateChanged: ({ active }) => [{ ...model, landscapeRotate: { active } }, []],
      ReceivedMeGravatarHash: ({ email, hash }) => {
        if (model.session.me?.email !== email) return [model, []];
        return [{ ...model, session: { ...model.session, meGravatarHash: hash } }, []];
      },
      ModalOpened: () => [model, []],
      CardArtTick: () => [model, []],
      DeckCardFlipTick: () => [model, []],
      GotBoardMessage: ({ message }) => foldBoard(model, message),
      GotAuthMessage: ({ message }) => foldAuth(model, message),
      GotDeckListMessage: ({ message }) => {
        const [nextModel, commands] = foldDeckList(model, message);
        if (message._tag !== "ReceivedDecks") return [nextModel, commands];
        return [notFoundWhenPlayDeckMissing(nextModel), commands];
      },
      GotDeckBuilderMessage: ({ message }) => foldDeckBuilder(model, message),
      GotGameMessage: ({ message }) => {
        if (model.game == null) return [model, []];
        const [game, commands] = updateGame(model.game, message);
        return [{ ...model, game }, commands];
      },
      GotLeaderboardMessage: ({ message }) => foldLeaderboard(model, message),
      GotCoverageMessage: ({ message }) => foldCoverage(model, message),
      GotLobbyMessage: ({ message }) => foldLobby(model, message),
      ToggledAccountMenu: () => {
        if (
          model.route._tag === "HomeRoute" ||
          model.route._tag === "NewDeckRoute" ||
          model.route._tag === "DeckRoute" ||
          model.route._tag === "PlayRoute" ||
          model.route._tag === "PregameTableRoute" ||
          model.route._tag === "GameTableRoute"
        ) {
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
        if (model.route._tag === "CoverageRoute") {
          return [
            {
              ...model,
              coverage: {
                ...model.coverage,
                accountMenuOpen: !model.coverage.accountMenuOpen,
              },
            },
            [],
          ];
        }
        return [model, []];
      },
      ClosedAccountMenu: () => {
        if (
          model.route._tag === "HomeRoute" ||
          model.route._tag === "NewDeckRoute" ||
          model.route._tag === "DeckRoute" ||
          model.route._tag === "PlayRoute" ||
          model.route._tag === "PregameTableRoute" ||
          model.route._tag === "GameTableRoute"
        ) {
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
        if (model.route._tag === "CoverageRoute") {
          return [
            {
              ...model,
              coverage: { ...model.coverage, accountMenuOpen: false },
            },
            [],
          ];
        }
        return [model, []];
      },
    }),
  );
