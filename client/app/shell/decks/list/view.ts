import { Effect, Queue, Schema as S, Stream } from "effect";
import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { cn } from "../../../domain/cn";
import type { AppChromeMeta } from "../../../domain/ui/app-version";
import { buttonClass } from "../../../domain/ui/buttonClass";
import { confirmDialog } from "../../../domain/ui/confirmDialog";
import { fieldClass, listRowClass } from "../../../domain/ui/surfaces";
import type { CardArtTick, DeckCardFlipTick, GotAuthMessage, ModalOpened } from "../../../messages";
import { DeckRoute, NewDeckRoute, PlayRoute, routePath } from "../../../routes";
import type { ClosedAccountMenu, ToggledAccountMenu } from "../../account-chrome/messages";
import { accountChrome } from "../../account-chrome/view";
import { shellFrame } from "../../frame/shell-frame";
import { type DeckCardModel, renderDeckCard } from "../deck-card";
import {
  AskedDeckDelete,
  CancelledDeckDelete,
  ChangedDeckListSearch,
  ClosedDeckListMenu,
  type Message,
  OpenedDeckListMenu,
  RequestedDeckDelete,
} from "./messages";
import type { DeckListSubmodel } from "./submodel";
import { deckListContextMenuAllowed, visibleDecks } from "./visible";

export type ViewMessage =
  | Message
  | typeof ModalOpened.Type
  | typeof CardArtTick.Type
  | typeof DeckCardFlipTick.Type
  | typeof GotAuthMessage.Type
  | typeof ToggledAccountMenu.Type
  | typeof ClosedAccountMenu.Type;

const h = html<ViewMessage>();

const MENU_ITEM =
  "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine";

type ContextMenuMessage = typeof OpenedDeckListMenu.Type | typeof ClosedDeckListMenu.Type;

export const BindDeckListContextMenu = Mount.defineStream(
  "BindDeckListContextMenu",
  { deckId: S.Number },
  OpenedDeckListMenu,
  ClosedDeckListMenu,
)(
  (args) => (element) =>
    Stream.callback<ContextMenuMessage>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => {
            const onContextMenu = (event: Event) => {
              event.preventDefault();
              event.stopPropagation();
              if (!(event instanceof MouseEvent)) return;
              if (!deckListContextMenuAllowed(args.deckId)) return;
              Queue.offerUnsafe(queue, OpenedDeckListMenu({ deckId: args.deckId, x: event.clientX, y: event.clientY }));
            };
            element.addEventListener("contextmenu", onContextMenu);
            return () => element.removeEventListener("contextmenu", onContextMenu);
          }),
          (teardown) => Effect.sync(teardown),
        );
        return yield* Effect.never;
      }),
    ),
);

const CONTEXT_MENU_ROOT_SELECTOR = '[data-testid="deck-list-context-menu-root"]';

/** Window-level Escape while the deck list context menu is open. */
export const BindDeckListContextMenuEscape = Mount.defineStream(
  "BindDeckListContextMenuEscape",
  ClosedDeckListMenu,
)((_element) =>
  Stream.callback<typeof ClosedDeckListMenu.Type>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          const onKeyDown = (event: Event): void => {
            if (!(event instanceof KeyboardEvent)) return;
            if (event.key !== "Escape") return;
            if (document.querySelector(CONTEXT_MENU_ROOT_SELECTOR) == null) return;
            event.preventDefault();
            Queue.offerUnsafe(queue, ClosedDeckListMenu());
          };
          window.addEventListener("keydown", onKeyDown);
          return onKeyDown;
        }),
        (onKeyDown) =>
          Effect.sync(() => {
            window.removeEventListener("keydown", onKeyDown);
          }),
      );
      return yield* Effect.never;
    }),
  ),
);

function commanderName(model: DeckListSubmodel, id: string): string {
  return model.knownCommanders[id]?.name ?? id;
}

function commanderPrint(model: DeckListSubmodel, deck: DeckListSubmodel["decks"][number]): string {
  return deck.commander_print || model.knownCommanders[deck.commander]?.default_print || "";
}

function deckCardModel(model: DeckListSubmodel, deck: DeckListSubmodel["decks"][number]): DeckCardModel {
  const commander = model.knownCommanders[deck.commander];
  return {
    id: deck.id,
    name: deck.name,
    commander: deck.commander,
    commanderName: commanderName(model, deck.commander),
    print: commanderPrint(model, deck),
    colorIdentity: commander?.color_identity ?? [],
  };
}

function contextMenu(model: DeckListSubmodel): Html {
  const menu = model.contextMenu;
  if (menu == null) return null;

  const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
  const vh = typeof window !== "undefined" ? window.innerHeight : 720;
  const x = Math.min(menu.x, vw - 180);
  const y = Math.min(menu.y, vh - 120);

  return h.div(
    [h.DataAttribute("testid", "deck-list-context-menu-root")],
    [
      h.div(
        [
          h.Class("fixed inset-0 z-40"),
          h.DataAttribute("testid", "deck-list-context-menu-catcher"),
          h.OnClick(ClosedDeckListMenu()),
          h.OnContextMenu(ClosedDeckListMenu()),
        ],
        [],
      ),
      h.div(
        [
          h.DataAttribute("testid", "deck-list-context-menu"),
          h.Class(
            "fixed top-(--y) left-(--x) z-41 flex min-w-[160px] flex-col rounded-hud border border-vine bg-forest-surface p-xs shadow-table",
          ),
          h.Style({ "--x": `${x}px`, "--y": `${y}px` }),
        ],
        [
          h.a(
            [
              h.DataAttribute("testid", "deck-list-menu-edit"),
              h.Href(routePath(DeckRoute({ id: String(menu.deckId) }))),
              h.OnClick(ClosedDeckListMenu()),
              h.Class(cn(MENU_ITEM, "no-underline")),
            ],
            ["Edit"],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "deck-list-menu-delete"),
              h.OnClick(AskedDeckDelete({ id: menu.deckId })),
              h.Class(MENU_ITEM),
            ],
            ["Delete"],
          ),
        ],
      ),
    ],
  );
}

export type ViewInputs = {
  readonly username: string;
  readonly meGravatarHash: string | null;
  readonly chrome: AppChromeMeta;
};

export const view = Submodel.defineView<DeckListSubmodel, ViewMessage, ViewInputs>((model, viewInputs) => {
  const visible = visibleDecks(model.decks, model.knownCommanders, model.searchQuery);

  return shellFrame(h, {
    atmosphere: "shell",
    title: "Your decks",
    chrome: viewInputs.chrome,
    trailing: accountChrome(h, {
      username: viewInputs.username,
      gravatarHash: viewInputs.meGravatarHash,
      menuOpen: model.accountMenuOpen,
      showLeaderboardLink: true,
    }),
    stage: h.div(
      [
        h.Class("h-full overflow-y-auto"),
        h.DataAttribute("testid", "decks-page"),
        h.OnMount(BindDeckListContextMenuEscape()),
      ],
      [
        model.confirmingDeleteId != null
          ? confirmDialog(h, {
              title: `Delete "${model.decks.find((d) => d.id === model.confirmingDeleteId)?.name ?? ""}"?`,
              body: "This deck and its card list are gone for good.",
              confirmLabel: "Delete deck",
              danger: true,
              onConfirm: RequestedDeckDelete({ id: model.confirmingDeleteId }),
              onCancel: CancelledDeckDelete(),
              testId: "confirm-delete-dialog",
            })
          : null,
        h.section(
          [h.Class("mx-auto max-w-[960px]")],
          [
            model.error == null
              ? null
              : h.div([h.Role("alert"), h.Class("text-label text-reconnect-rust")], [model.error]),
            model.loading ? h.div([h.Class("text-label text-lichen")], ["Loading decks…"]) : null,
            !model.loading && model.decks.length > 0
              ? h.input([
                  h.Type("search"),
                  h.DataAttribute("testid", "deck-list-search"),
                  h.AriaLabel("Search decks"),
                  h.Placeholder("Search decks…"),
                  h.Value(model.searchQuery),
                  h.OnInput((value) => ChangedDeckListSearch({ query: value })),
                  h.Class(fieldClass("mb-md w-full max-w-[960px]")),
                ])
              : null,
            !model.loading && model.decks.length > 0 && visible.length === 0
              ? h.div([h.Class("text-label text-lichen")], ["No decks match."])
              : null,
            !model.loading && model.decks.length === 0
              ? h.div(
                  [
                    h.DataAttribute("testid", "deck-list-empty"),
                    h.Class(
                      listRowClass(
                        "mb-md flex flex-col items-center gap-sm rounded-panel border border-dashed border-vine bg-glass p-xl text-center",
                      ),
                    ),
                  ],
                  [
                    h.h2([h.Class("m-0 text-title text-snow")], ["Build your first Commander deck"]),
                    h.p(
                      [h.Class("m-0 max-w-[34rem] text-label text-lichen")],
                      ["Create a deck, choose a commander, then use it to host or join a table."],
                    ),
                    h.a(
                      [h.Href(routePath(NewDeckRoute())), h.Class(buttonClass("primary", "mt-xs no-underline"))],
                      ["Create a deck"],
                    ),
                  ],
                )
              : null,
            !model.loading
              ? h.div(
                  [
                    h.DataAttribute("testid", "deck-list-grid"),
                    h.Class("mx-auto grid max-w-[960px] grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-md"),
                  ],
                  [
                    h.a(
                      [
                        h.Href(routePath(NewDeckRoute())),
                        h.DataAttribute("testid", "deck-list-new-deck"),
                        h.Class(
                          listRowClass(
                            "flex aspect-auto min-h-[200px] flex-col items-center justify-center gap-sm border border-dashed border-vine bg-transparent no-underline",
                          ),
                        ),
                        h.AriaLabel("New deck"),
                      ],
                      [
                        h.span([h.Class("text-title text-lichen")], ["+"]),
                        h.span([h.Class("text-label font-semibold text-snow")], ["New deck"]),
                      ],
                    ),
                    ...visible.map((deck) => {
                      return renderDeckCard(h, deckCardModel(model, deck), {
                        mode: "link",
                        href: routePath(PlayRoute({ deckId: String(deck.id) })),
                        rootAttrs: [h.OnMount(BindDeckListContextMenu({ deckId: deck.id }))],
                        testId: `deck-tile-${deck.id}`,
                      });
                    }),
                  ],
                )
              : null,
          ],
        ),
        contextMenu(model),
      ],
    ),
  });
});
