import { type Document, html } from "foldkit/html";
import { type ViewMessage as BoardViewMessage, view as boardView } from "./board";
import { parseDeckIdParam, playDeckAccess } from "./deck-id";
import type { AppChromeMeta } from "./domain/ui/app-version";
import {
  GotAuthMessage,
  GotBoardMessage,
  GotCoverageMessage,
  GotDeckBuilderMessage,
  GotDeckListMessage,
  GotLeaderboardMessage,
  GotLobbyMessage,
  type Message,
} from "./messages";
import type { Model } from "./model";
import { CoverageRoute, HomeRoute, isProtectedRoute, NewDeckRoute, routePath } from "./routes";
import * as Auth from "./shell/auth";
import * as Coverage from "./shell/coverage";
import * as DeckBuilder from "./shell/decks/builder";
import * as DeckList from "./shell/decks/list";
import { shellFrame } from "./shell/frame/shell-frame";
import * as Leaderboard from "./shell/leaderboard";
import * as Lobby from "./shell/lobby";

const h = html<Message>();

function chromeMeta(model: Model): AppChromeMeta {
  return {
    version: model.apiVersion,
    faithfulCount: model.faithfulCount,
    oracleTotal: model.oracleTotal,
    coverageHref: routePath(CoverageRoute()),
  };
}

function nav(model: Model) {
  const user = model.session.me;

  return h.header(
    [h.Class("flex items-center justify-between gap-lg border-b border-white/10 p-lg")],
    [
      h.a([h.Class("text-display text-snow no-underline"), h.Href(routePath(HomeRoute()))], ["edh.reilley.dev"]),
      h.nav(
        [h.Class("flex items-center gap-md text-label text-lichen")],
        [
          h.a([h.Href(routePath(HomeRoute())), h.Class("underline")], ["Play"]),
          h.a([h.Href(routePath(NewDeckRoute())), h.Class("underline")], ["New deck"]),
          user == null
            ? h.a([h.Href("/login"), h.Class("underline")], ["Sign in"])
            : h.button(
                [
                  h.Type("button"),
                  h.Class("hit-quiet underline"),
                  h.OnClick(GotAuthMessage({ message: Auth.Message.RequestedLogout() })),
                ],
                [`Sign out ${user.username}`],
              ),
        ],
      ),
    ],
  );
}

function shell(model: Model, title: string, body: string) {
  return shellFrame(h, {
    atmosphere: "shell",
    title,
    chrome: chromeMeta(model),
    stage: h.section(
      [h.Class("mx-auto flex max-w-[960px] flex-col gap-md")],
      [h.p([h.Class("m-0 text-body text-snow/80")], [body])],
    ),
  });
}

function toParentDeckListMessage(message: DeckList.ViewMessage): Message {
  switch (message._tag) {
    case "CardArtTick":
    case "DeckCardFlipTick":
    case "GotAuthMessage":
    case "GotAccountMenuMessage":
      return message;
    default:
      return GotDeckListMessage({ message });
  }
}

function toParentDeckBuilderMessage(message: DeckBuilder.ViewMessage): Message {
  switch (message._tag) {
    case "GotAccountMenuMessage":
    case "GotAuthMessage":
    case "CardArtTick":
      return message;
    default:
      return GotDeckBuilderMessage({ message });
  }
}

function toParentLobbyMessage(message: Lobby.ViewMessage): Message {
  switch (message._tag) {
    case "CardArtTick":
    case "DeckCardFlipTick":
    case "GotAccountMenuMessage":
    case "GotAuthMessage":
      return message;
    default:
      return GotLobbyMessage({ message });
  }
}

function toParentBoardMessage(message: BoardViewMessage): Message {
  switch (message._tag) {
    case "CardArtTick":
      return message;
    default:
      return GotBoardMessage({ message });
  }
}

function toParentLeaderboardMessage(message: Leaderboard.ViewMessage): Message {
  switch (message._tag) {
    case "GotAccountMenuMessage":
    case "GotAuthMessage":
      return message;
    default:
      return GotLeaderboardMessage({ message });
  }
}

function toParentCoverageMessage(message: Coverage.ViewMessage): Message {
  switch (message._tag) {
    case "GotAccountMenuMessage":
    case "GotAuthMessage":
      return message;
    default:
      return GotCoverageMessage({ message });
  }
}

function boardMount(model: Model) {
  const tableId =
    model.game?.tableId ??
    model.lobby.tableId ??
    (model.route._tag === "PregameTableRoute" || model.route._tag === "GameTableRoute" ? model.route.table : null);
  const game = model.game;

  if (game != null) {
    return h.submodel({
      slotId: "board",
      model: { board: game.board, fold: game, tableId, connected: game.connected },
      view: boardView,
      toParentMessage: toParentBoardMessage,
    });
  }

  return h.main(
    [h.Class("min-h-screen bg-forest-floor text-snow"), h.DataAttribute("testid", "board-mount")],
    [
      nav(model),
      h.section(
        [h.Class("mx-auto flex max-w-[960px] flex-col gap-md p-xxl")],
        [
          h.h1([h.Class("m-0 text-title text-lichen")], ["Board"]),
          h.p(
            [h.Class("m-0 text-body text-snow/80")],
            [tableId == null ? "Board mount point ready." : `Board mount point for table ${tableId}.`],
          ),
        ],
      ),
    ],
  );
}

function routeBody(model: Model) {
  if (isProtectedRoute(model.route) && (!model.sessionLoaded || model.session.me == null)) {
    // Spec: no persistent nav chrome. Blank gate until session resolves (avoids Play/Sign in flash).
    return h.main([h.Class("min-h-screen bg-forest-floor"), h.DataAttribute("testid", "session-gate")], []);
  }

  return (() => {
    switch (model.route._tag) {
      case "HomeRoute":
        return h.submodel({
          slotId: "deck-list",
          model: model.decks.list,
          view: DeckList.view,
          viewInputs: {
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            chrome: chromeMeta(model),
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentDeckListMessage,
        });
      case "LoginRoute":
        return h.submodel({
          slotId: "auth",
          model: model.auth,
          view: Auth.view,
          viewInputs: chromeMeta(model),
          toParentMessage: (message) => GotAuthMessage({ message }),
        });
      case "LeaderboardRoute":
        return h.submodel({
          slotId: "leaderboard",
          model: model.leaderboard,
          view: Leaderboard.view,
          viewInputs: {
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            chrome: chromeMeta(model),
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentLeaderboardMessage,
        });
      case "CoverageRoute":
        return h.submodel({
          slotId: "coverage",
          model: model.coverage,
          view: Coverage.view,
          viewInputs: {
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            chrome: chromeMeta(model),
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentCoverageMessage,
        });
      case "NewDeckRoute":
        return h.submodel({
          slotId: "deck-builder",
          model: model.decks.builder,
          view: DeckBuilder.view,
          viewInputs: {
            chrome: chromeMeta(model),
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentDeckBuilderMessage,
        });
      case "DeckRoute":
        return h.submodel({
          slotId: "deck-builder",
          model: model.decks.builder,
          view: DeckBuilder.view,
          viewInputs: {
            chrome: chromeMeta(model),
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentDeckBuilderMessage,
        });
      case "PlayRoute": {
        if (model.game?.active === true) return boardMount(model);
        const deckId = parseDeckIdParam(model.route.deckId);
        const access = playDeckAccess(deckId, model.decks.list.decks, model.decks.list.loading, model.decks.list.error);
        if (access === "missing") return shell(model, "Not found", `No Foldkit route for ${model.currentPath}.`);
        return h.submodel({
          slotId: "lobby-entry",
          model: model.lobby,
          view: Lobby.view,
          viewInputs: {
            decks: model.decks.list.decks,
            decksLoading: model.decks.list.loading,
            knownCommanders: model.decks.list.knownCommanders,
            chrome: chromeMeta(model),
            surface: "entry",
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentLobbyMessage,
        });
      }
      case "PregameTableRoute": {
        if (model.game?.active === true) return boardMount(model);
        const deckId = parseDeckIdParam(model.route.deckId);
        const access = playDeckAccess(deckId, model.decks.list.decks, model.decks.list.loading, model.decks.list.error);
        if (access === "missing") return shell(model, "Not found", `No Foldkit route for ${model.currentPath}.`);
        return h.submodel({
          slotId: "lobby-table",
          model: model.lobby,
          view: Lobby.view,
          viewInputs: {
            decks: model.decks.list.decks,
            decksLoading: model.decks.list.loading,
            knownCommanders: model.decks.list.knownCommanders,
            chrome: chromeMeta(model),
            surface: "table",
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentLobbyMessage,
        });
      }
      case "GameTableRoute":
        if (model.game?.active === true) return boardMount(model);
        return h.submodel({
          slotId: "lobby-table",
          model: model.lobby,
          view: Lobby.view,
          viewInputs: {
            decks: model.decks.list.decks,
            decksLoading: model.decks.list.loading,
            knownCommanders: model.decks.list.knownCommanders,
            chrome: chromeMeta(model),
            surface: "table",
            username: model.session.me?.username ?? "",
            meGravatarHash: model.session.meGravatarHash,
            accountMenu: model.accountMenu,
          },
          toParentMessage: toParentLobbyMessage,
        });
      case "NotFoundRoute":
        return shell(model, "Not found", `No Foldkit route for ${model.route.path}.`);
      default: {
        const exhaustive: never = model.route;
        return exhaustive;
      }
    }
  })();
}

export const view = (model: Model): Document => {
  return {
    title: "edh.reilley.dev",
    body: h.div(
      [
        h.DataAttribute("testid", "landscape-root"),
        ...(model.landscapeRotate.active ? [h.Class("landscape-rotate-root")] : []),
      ],
      [routeBody(model)],
    ),
  };
};
