import { Effect, Option, Queue, Schema as S, Stream } from "effect";
import { type Html, html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { cn } from "../../../../lib/cn";
import { cardHoverPreviewView } from "../../../../lib/deck-builder/card-hover-preview";
import { manaFontClass } from "../../../../lib/oracleText";
import { appVersionBadge } from "../../../../lib/ui/app-version";
import { buttonClass } from "../../../../lib/ui/buttonClass";
import { cardArt } from "../../../../lib/ui/card-art";
import { confirmDialog } from "../../../../lib/ui/confirmDialog";
import { feltClass, fieldClass, listRowClass } from "../../../../lib/ui/surfaces";
import type { Message } from "../../../messages";
import { RequestedLogout } from "../../../messages";
import { DeckRoute, NewDeckRoute, PlayRoute, routePath } from "../../../routes";
import {
  AskedDeckDelete,
  CancelledDeckDelete,
  ChangedDeckListSearch,
  ClearedDeckListHover,
  ClosedDeckListMenu,
  MovedDeckListHover,
  OpenedDeckListMenu,
  RequestedDeckDelete,
} from "./messages";
import type { DeckListSubmodel } from "./submodel";
import { deckListContextMenuAllowed, identityPipCodes, visibleDecks } from "./visible";

const h = html<Message>();

const MENU_ITEM =
  "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine";

export const BindDeckListCommanderHover = Mount.defineStream(
  "BindDeckListCommanderHover",
  { cardId: S.String, print: S.String },
  MovedDeckListHover,
  ClearedDeckListHover,
)(
  (args) => (element) =>
    Stream.callback<typeof MovedDeckListHover.Type | typeof ClearedDeckListHover.Type>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => {
            const onMouseMove = (event: Event) => {
              if (!(event instanceof MouseEvent)) return;
              Queue.offerUnsafe(
                queue,
                MovedDeckListHover({ id: args.cardId, print: args.print, x: event.clientX, y: event.clientY }),
              );
            };
            const onMouseLeave = () => {
              Queue.offerUnsafe(queue, ClearedDeckListHover());
            };
            element.addEventListener("mousemove", onMouseMove);
            element.addEventListener("mouseleave", onMouseLeave);
            return () => {
              element.removeEventListener("mousemove", onMouseMove);
              element.removeEventListener("mouseleave", onMouseLeave);
            };
          }),
          (teardown) => Effect.sync(teardown),
        );
        return yield* Effect.never;
      }),
    ),
);

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

function commanderName(model: DeckListSubmodel, id: string): string {
  return model.knownCommanders[id]?.name ?? id;
}

function commanderPrint(model: DeckListSubmodel, deck: DeckListSubmodel["decks"][number]): string {
  return deck.commander_print ?? model.knownCommanders[deck.commander]?.default_print ?? "";
}

function hoverPreview(model: DeckListSubmodel): Html | null {
  const hover = model.hover;
  if (hover == null) return null;
  return cardHoverPreviewView(h, {
    hover,
    card: model.knownCommanders[hover.id],
    testId: "deck-list-hover-preview",
  });
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
          h.OnKeyDownPreventDefault((key) => (key === "Escape" ? Option.some(ClosedDeckListMenu()) : Option.none())),
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

export function view(model: DeckListSubmodel, username: string, apiVersion: string | null): Html {
  const visible = visibleDecks(model.decks, model.knownCommanders, model.searchQuery);

  return h.main(
    [
      h.Class(
        feltClass(
          "h-full overflow-y-auto p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]",
        ),
      ),
      h.DataAttribute("testid", "decks-page"),
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
      h.div(
        [h.Class("mx-auto mb-5 flex max-w-[720px] flex-wrap items-center justify-between gap-md")],
        [
          h.h1([h.Class("m-0 text-title")], ["Your decks"]),
          h.div(
            [h.Class("flex flex-wrap items-center gap-md")],
            [
              h.span([h.Class("text-label text-lichen")], [username]),
              h.button([h.Type("button"), h.OnClick(RequestedLogout()), h.Class(buttonClass("ghost"))], ["Sign out"]),
              h.a([h.Href(routePath(NewDeckRoute())), h.Class(buttonClass("primary"))], ["New deck"]),
            ],
          ),
        ],
      ),
      h.section(
        [h.Class("mx-auto max-w-[960px]")],
        [
          model.error == null
            ? null
            : h.div([h.Role("alert"), h.Class("text-label text-reconnect-rust")], [model.error]),
          model.loading ? h.div([h.Class("text-label text-lichen")], ["Loading decks…"]) : null,
          !model.loading && model.decks.length === 0
            ? h.div([h.Class("text-label text-lichen")], ["No decks yet — build one to get started."])
            : null,
          !model.loading && model.decks.length > 0
            ? h.input([
                h.Type("search"),
                h.DataAttribute("testid", "deck-list-search"),
                h.Placeholder("Search decks…"),
                h.Value(model.searchQuery),
                h.OnInput((value) => ChangedDeckListSearch({ query: value })),
                h.Class(fieldClass("mb-md w-full max-w-[720px]")),
              ])
            : null,
          !model.loading && model.decks.length > 0 && visible.length === 0
            ? h.div([h.Class("text-label text-lichen")], ["No decks match."])
            : null,
          !model.loading && visible.length > 0
            ? h.div(
                [h.Class("mx-auto grid max-w-[960px] grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-md")],
                visible.map((deck) => {
                  const commander = model.knownCommanders[deck.commander];
                  const print = commanderPrint(model, deck);
                  const pips = identityPipCodes(commander?.color_identity ?? []);

                  return h.a(
                    [
                      h.Href(`${routePath(PlayRoute())}?deck=${deck.id}`),
                      h.DataAttribute("testid", `deck-tile-${deck.id}`),
                      h.Class(
                        listRowClass("relative flex flex-col overflow-hidden rounded-hud no-underline text-snow"),
                      ),
                      h.OnMount(BindDeckListContextMenu({ deckId: deck.id })),
                    ],
                    [
                      h.div(
                        [
                          h.Class("flex flex-1 flex-col"),
                          h.OnMount(
                            BindDeckListCommanderHover({
                              cardId: deck.commander,
                              print,
                            }),
                          ),
                        ],
                        [
                          print === ""
                            ? h.div([h.Class("h-[110px] w-full bg-glass")], [])
                            : cardArt(h, {
                                print,
                                size: "art_crop",
                                alt: "",
                                className: "h-[110px] w-full object-cover",
                              }),
                          h.div(
                            [h.Class("flex min-h-[86px] flex-col gap-xs p-md")],
                            [
                              h.div(
                                [h.Class("truncate text-label font-semibold")],
                                [
                                  deck.name,
                                  deck.id < 0
                                    ? h.span(
                                        [
                                          h.Class(
                                            "ml-sm rounded-full bg-lichen/14 px-[7px] py-px align-middle text-chip text-lichen",
                                          ),
                                        ],
                                        ["Precon"],
                                      )
                                    : null,
                                ],
                              ),
                              h.div(
                                [h.Class("truncate text-chip text-lichen")],
                                [commanderName(model, deck.commander)],
                              ),
                              pips.length === 0
                                ? null
                                : h.div(
                                    [h.Class("mt-auto flex gap-[3px] text-[14px] text-snow")],
                                    pips.map((code) => {
                                      const ms = manaFontClass(code);
                                      if (ms == null) return null;
                                      return h.i([h.Class(`ms ms-cost ms-${ms}`)], []);
                                    }),
                                  ),
                            ],
                          ),
                        ],
                      ),
                    ],
                  );
                }),
              )
            : null,
        ],
      ),
      appVersionBadge(h, apiVersion),
      hoverPreview(model),
      contextMenu(model),
    ],
  );
}
