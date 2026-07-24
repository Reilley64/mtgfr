import { Effect } from "effect";
import { type Document, html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { view as boardView } from "./board/view";
import { CompletedPortraitGateModal, type Message, PortraitGateCancelled, RequestedLogout } from "./messages";
import type { Model } from "./model";
import { HomeRoute, isProtectedRoute, NewDeckRoute, PlayRoute, routePath } from "./routes";
import { view as authView } from "./shell/auth/view";
import { view as deckBuilderView } from "./shell/decks/builder/view";
import { view as deckListView } from "./shell/decks/list/view";
import { view as lobbyView } from "./shell/lobby/view";

const h = html<Message>();

export const OpenPortraitGateModal = Mount.define(
  "OpenPortraitGateModal",
  CompletedPortraitGateModal,
)((element) =>
  Effect.gen(function* () {
    yield* Effect.acquireRelease(
      Effect.sync(() => {
        if (typeof HTMLDialogElement === "undefined") return null;
        if (!(element instanceof HTMLDialogElement)) return null;

        const handle = { cancelled: false, dialog: element };
        queueMicrotask(() => {
          if (handle.cancelled || !element.isConnected || element.open) return;
          element.showModal();
        });
        return handle;
      }),
      (handle) =>
        Effect.sync(() => {
          if (handle == null) return;
          handle.cancelled = true;
          if (handle.dialog.open) handle.dialog.close();
        }),
    );
    return CompletedPortraitGateModal();
  }),
);

function nav(model: Model) {
  const user = model.session.me;

  return h.header(
    [h.Class("flex items-center justify-between gap-lg border-b border-white/10 p-lg")],
    [
      h.a([h.Class("text-display text-snow no-underline"), h.Href(routePath(HomeRoute()))], ["edh.reilley.dev"]),
      h.nav(
        [h.Class("flex items-center gap-md text-label text-lichen")],
        [
          h.a([h.Href(routePath(PlayRoute())), h.Class("underline")], ["Play"]),
          h.a([h.Href(routePath(NewDeckRoute())), h.Class("underline")], ["New deck"]),
          user == null
            ? h.a([h.Href("/login"), h.Class("underline")], ["Sign in"])
            : h.button(
                [h.Type("button"), h.Class("hit-quiet underline"), h.OnClick(RequestedLogout())],
                [`Sign out ${user.username}`],
              ),
        ],
      ),
    ],
  );
}

function shell(model: Model, title: string, body: string) {
  return h.main(
    [h.Class("min-h-screen bg-forest-floor text-snow")],
    [
      nav(model),
      h.section(
        [h.Class("mx-auto flex max-w-[960px] flex-col gap-md p-xxl")],
        [h.h1([h.Class("m-0 text-title text-lichen")], [title]), h.p([h.Class("m-0 text-body text-snow/80")], [body])],
      ),
    ],
  );
}

function boardMount(model: Model) {
  const tableId =
    model.game?.tableId ?? model.lobby.tableId ?? (model.route._tag === "TableRoute" ? model.route.table : null);
  const game = model.game;

  if (game != null) {
    return h.submodel({
      slotId: "board",
      model: { board: game.board, fold: game, tableId, connected: game.connected },
      view: boardView,
      toParentMessage: (message) => message,
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

function portraitGate() {
  return h.dialog(
    [
      h.Id("portrait-gate"),
      h.Class("portrait-gate bg-forest-floor font-sans text-body text-snow"),
      h.Attribute("aria-labelledby", "portrait-gate-title"),
      h.OnMount(OpenPortraitGateModal()),
      h.OnCancel(PortraitGateCancelled()),
    ],
    [
      h.div([h.Id("portrait-gate-title"), h.Class("text-title")], ["Rotate to landscape"]),
      h.div(
        [h.Class("max-w-[28ch] text-label text-lichen")],
        ["The table and deck builder are built for horizontal screens. Turn your device sideways to continue."],
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
        return deckListView(model.decks.list, model.session.me?.username ?? "", model.apiVersion);
      case "LoginRoute":
        return authView(model.auth, model.apiVersion);
      case "NewDeckRoute":
        return deckBuilderView(model.decks.builder, model.apiVersion);
      case "DeckRoute":
        return deckBuilderView(model.decks.builder, model.apiVersion);
      case "PlayRoute":
        return model.game?.active === true
          ? boardMount(model)
          : lobbyView(model.lobby, model.decks.list.decks, model.decks.list.loading, model.apiVersion);
      case "TableRoute":
        return model.game?.active === true
          ? boardMount(model)
          : lobbyView(model.lobby, model.decks.list.decks, model.decks.list.loading, model.apiVersion);
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
    body: h.div([], [routeBody(model), model.portraitGate.open ? portraitGate() : null]),
  };
};
