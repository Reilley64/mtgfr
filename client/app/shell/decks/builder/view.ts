import type * as Menu from "@foldkit/ui/menu";
import { Effect, Option, Queue, Schema as S, Stream } from "effect";
import { Submodel } from "foldkit";
import type { Html, HtmlBuilder } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { cn } from "../../../domain/cn";
import { cardHoverPreviewView } from "../../../domain/deck-builder/card-hover-preview";
import { DECK_SIZE, deckCount, sortedDeckList } from "../../../domain/deck-builder/cards";
import { formatReleasedAt } from "../../../domain/deck-builder/print";
import type { ImageSize, ScryfallPrint } from "../../../domain/deck-builder/scryfall";
import type { AppChromeMeta } from "../../../domain/ui/app-version";
import { button } from "../../../domain/ui/button";
import { cardArt } from "../../../domain/ui/card-art";
import { confirmDialog } from "../../../domain/ui/confirmDialog";
import { modalDialog } from "../../../domain/ui/dialog";
import { input } from "../../../domain/ui/input";
import { menuItemClass, menuPanelClass } from "../../../domain/ui/menu";
import { alertClass, listRowClass } from "../../../domain/ui/surfaces";
import { windowedGrid } from "../../../domain/ui/windowedGrid";
import { type CardArtTick, GotAccountMenuMessage, type GotAuthMessage } from "../../../messages";
import { accountChrome } from "../../account-chrome/view";
import { shellFrame } from "../../frame/shell-frame";
import {
  ActivatedBuilderTarget,
  ChangedBuilderName,
  ChangedBuilderQuery,
  ClearedBuilderHover,
  ClosedBuilderMenu,
  ConfirmedBuilderDiscard,
  GotDiscardDialogMessage,
  GotPoolGridMessage,
  GotPrintDialogMessage,
  GotPrintGridMessage,
  MeasuredPoolGrid,
  type Message,
  MovedBuilderHover,
  OpenedBuilderMenu,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  RequestedBuilderCancel,
  SubmittedDeckSave,
} from "./messages";
import {
  type BuilderPrintPicker,
  type DeckBuilderSubmodel,
  POOL_GRID_ID,
  PRINT_DIALOG_ID,
  PRINT_GRID_COLUMNS,
  PRINT_GRID_ID,
  poolGridColumns,
} from "./submodel";

export type ViewMessage =
  | Message
  | typeof CardArtTick.Type
  | typeof GotAccountMenuMessage.Type
  | typeof GotAuthMessage.Type;

const CONTEXT_MENU_PRESS_MS = 500;

const POOL_CARD = cn(
  listRowClass(),
  "flex cursor-pointer flex-col items-center gap-1 rounded-hud p-sm text-caption focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const DECK_ROW = cn(
  listRowClass(),
  "flex w-full cursor-pointer items-center gap-xs rounded-control px-sm py-1 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const PRINT_PICKER_COL = "w-[min(38vw,200px)]";
const PRINT_TILE = cn(
  PRINT_PICKER_COL,
  "flex cursor-pointer flex-col items-center gap-1.5 rounded-hud p-md text-label hover:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const PRINT_PICKER_GRID = "grid w-fit grid-cols-2 gap-md";
/** Classes on one row of the windowed print grid. `windowedGrid` adds `grid`. */
const PRINT_PICKER_ROW = "grid-cols-2 gap-md";
// Two badge lines, always reserved: set / collector / date wrap to a second line on narrow tiles,
// and the windowed grid needs every tile the same height. `h-10` is the 40px `submodel.ts` budgets.
const PRINT_BADGE_ROW = "flex h-10 w-full flex-wrap content-center items-center justify-center gap-1";
const PRINT_BADGE =
  "rounded-full border border-vine-dim bg-glass-dim px-[7px] py-px font-semibold text-chip text-lichen";
const CARD_ART = cn("aspect-[0.72] w-full rounded-control object-cover");
const PRINT_SKELETON = cn(PRINT_PICKER_COL, "flex cursor-default flex-col items-center gap-1.5 p-md");

/** Reports the pool column's width. The pool grid is windowed, and both its column count and its row
 *  height come from that width — VirtualList's own observer reports height only. */
export const ObservePoolWidth = Mount.defineStream(
  "ObservePoolWidth",
  MeasuredPoolGrid,
)((element) =>
  Stream.callback<typeof MeasuredPoolGrid.Type>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          const report = (width: number) => {
            if (width > 0) Queue.offerUnsafe(queue, MeasuredPoolGrid({ width }));
          };

          if (typeof ResizeObserver === "undefined") {
            report(element.clientWidth);
            return null;
          }

          const observer = new ResizeObserver((entries) => {
            const width = entries[0]?.contentRect.width;
            if (width !== undefined) report(width);
          });
          observer.observe(element);
          return observer;
        }),
        (observer) =>
          Effect.sync(() => {
            observer?.disconnect();
          }),
      );
      return yield* Effect.never;
    }),
  ),
);

type CardPointerMessage =
  | typeof MovedBuilderHover.Type
  | typeof ClearedBuilderHover.Type
  | typeof OpenedBuilderMenu.Type
  | typeof ActivatedBuilderTarget.Type;

/** Pointer chrome for pool tiles / deck rows / commander: hover preview, long-press + right-click menu, click activate. */
export const BindBuilderCardPointer = Mount.defineStream(
  "BindBuilderCardPointer",
  {
    cardId: S.String,
    kind: S.Union([S.Literal("pool"), S.Literal("deck"), S.Literal("commander")]),
  },
  MovedBuilderHover,
  ClearedBuilderHover,
  OpenedBuilderMenu,
  ActivatedBuilderTarget,
)(
  (args) => (element) =>
    Stream.callback<CardPointerMessage>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => {
            let pressTimer: ReturnType<typeof setTimeout> | undefined;
            let pressOrigin: { x: number; y: number } | null = null;
            let suppressClick = false;

            const clearPress = () => {
              if (pressTimer) clearTimeout(pressTimer);
              pressTimer = undefined;
              pressOrigin = null;
            };

            const openMenu = (x: number, y: number) => {
              Queue.offerUnsafe(queue, OpenedBuilderMenu({ cardId: args.cardId, kind: args.kind, x, y }));
            };

            const onPointerMove = (event: Event) => {
              if (!(event instanceof PointerEvent)) return;
              if (pressTimer && pressOrigin) {
                const dx = event.clientX - pressOrigin.x;
                const dy = event.clientY - pressOrigin.y;
                if (dx * dx + dy * dy > 100) clearPress();
              }
              Queue.offerUnsafe(queue, MovedBuilderHover({ id: args.cardId, x: event.clientX, y: event.clientY }));
            };

            const onPointerLeave = () => {
              clearPress();
              Queue.offerUnsafe(queue, ClearedBuilderHover());
            };

            const onPointerDown = (event: Event) => {
              if (!(event instanceof PointerEvent) || event.button !== 0) return;
              clearPress();
              const x = event.clientX;
              const y = event.clientY;
              pressOrigin = { x, y };
              pressTimer = setTimeout(() => {
                pressTimer = undefined;
                suppressClick = true;
                openMenu(x, y);
              }, CONTEXT_MENU_PRESS_MS);
            };

            const onPointerUp = () => {
              clearPress();
            };

            const onContextMenu = (event: Event) => {
              event.preventDefault();
              if (!(event instanceof MouseEvent)) return;
              clearPress();
              openMenu(event.clientX, event.clientY);
            };

            const onClick = (event: Event) => {
              if (suppressClick) {
                suppressClick = false;
                event.preventDefault();
                event.stopPropagation();
                return;
              }
              Queue.offerUnsafe(queue, ActivatedBuilderTarget({ cardId: args.cardId, kind: args.kind }));
            };

            element.addEventListener("pointermove", onPointerMove);
            element.addEventListener("pointerleave", onPointerLeave);
            element.addEventListener("pointerdown", onPointerDown);
            element.addEventListener("pointerup", onPointerUp);
            element.addEventListener("pointercancel", onPointerUp);
            element.addEventListener("contextmenu", onContextMenu);
            element.addEventListener("click", onClick);

            return () => {
              clearPress();
              element.removeEventListener("pointermove", onPointerMove);
              element.removeEventListener("pointerleave", onPointerLeave);
              element.removeEventListener("pointerdown", onPointerDown);
              element.removeEventListener("pointerup", onPointerUp);
              element.removeEventListener("pointercancel", onPointerUp);
              element.removeEventListener("contextmenu", onContextMenu);
              element.removeEventListener("click", onClick);
            };
          }),
          (teardown) => Effect.sync(teardown),
        );
        return yield* Effect.never;
      }),
    ),
);

// Size is required rather than defaulted: every art in the builder renders small, so the default
// `display` (672px) would be 3-5x oversampled everywhere. Pick from the rendered width — `grid`
// (488px) for the tile grids, `thumb` (146px) for the list rows.
function builderCardArt(
  print: string,
  alt: string,
  className: string,
  size: ImageSize,
  h: HtmlBuilder<ViewMessage>,
): Html {
  return cardArt(h, { print, alt, className, size });
}

function hoverPreview(model: DeckBuilderSubmodel, h: HtmlBuilder<ViewMessage>): Html | null {
  const hover = model.hover;
  if (hover == null) return null;
  return cardHoverPreviewView(h, {
    hover,
    card: model.known[hover.id],
    testId: "builder-hover-preview",
  });
}

function contextMenu(model: DeckBuilderSubmodel, h: HtmlBuilder<ViewMessage>): Html {
  const menu = model.menu;
  if (menu == null || model.printPicker != null) return null;

  const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
  const vh = typeof window !== "undefined" ? window.innerHeight : 720;
  const x = Math.min(menu.x, vw - 180);
  const y = Math.min(menu.y, vh - 140);

  return h.div(
    [h.DataAttribute("testid", "builder-context-menu-root")],
    [
      h.div(
        [
          h.Class("fixed inset-0 z-40"),
          h.DataAttribute("testid", "builder-context-menu-catcher"),
          h.OnClick(ClosedBuilderMenu()),
          h.OnContextMenu(ClosedBuilderMenu()),
          h.OnKeyDownPreventDefault((key) => (key === "Escape" ? Option.some(ClosedBuilderMenu()) : Option.none())),
        ],
        [],
      ),
      h.div(
        [
          h.DataAttribute("testid", "builder-context-menu"),
          h.Class(menuPanelClass("fixed top-(--y) left-(--x) z-41 min-w-[160px]")),
          h.Style({ "--x": `${x}px`, "--y": `${y}px` }),
        ],
        [
          h.div([h.Class("border-hud-edge border-b px-md pt-0.5 pb-1.5 text-label text-lichen")], [menu.title]),
          ...menu.items.map((item, index) =>
            h.button(
              [
                h.Type("button"),
                h.DataAttribute("testid", `builder-menu-item-${index}`),
                h.OnClick(RanBuilderMenuAction({ action: item.action })),
                h.Class(menuItemClass()),
              ],
              [item.label],
            ),
          ),
        ],
      ),
    ],
  );
}

function printTile(cardId: string, print: ScryfallPrint, h: HtmlBuilder<ViewMessage>): Html {
  return h.button(
    [
      h.Type("button"),
      h.Class(PRINT_TILE),
      h.DataAttribute("testid", `print-tile-${print.id}`),
      h.OnClick(PickedBuilderPrint({ cardId, print: print.id })),
    ],
    [
      builderCardArt(print.id, `${print.set_name} #${print.collector_number}`, CARD_ART, "grid", h),
      h.div(
        [h.Class(PRINT_BADGE_ROW)],
        [
          h.span([h.Class(PRINT_BADGE), h.Title(print.set_name)], [print.set.toUpperCase()]),
          h.span([h.Class(PRINT_BADGE)], [`#${print.collector_number}`]),
          h.span([h.Class(PRINT_BADGE)], [formatReleasedAt(print.released_at)]),
        ],
      ),
    ],
  );
}

function skeletonPrintTile(h: HtmlBuilder<ViewMessage>): Html {
  return h.div(
    [h.Class(cn(PRINT_SKELETON, "pointer-events-none")), h.DataAttribute("testid", "print-skeleton")],
    [
      h.div([h.Class(cn("aspect-[0.72] w-full animate-skeleton rounded-control bg-white/8"))], []),
      h.div([h.Class("h-2.5 w-[70%] animate-skeleton rounded-[3px] bg-white/8")], []),
    ],
  );
}

/** The picker's scrolling area: skeletons, a status line, or the prints themselves. A card can have
 *  hundreds of printings (basic lands especially), so the prints are windowed. */
function printPickerBody(model: DeckBuilderSubmodel, picker: BuilderPrintPicker, h: HtmlBuilder<ViewMessage>): Html {
  // Skeletons only until the first page lands; later pages append under the prints already shown.
  if (picker.prints.length === 0) {
    if (picker.error) {
      return h.div([h.Class("text-burn-red text-label")], ["Could not load printings. Close and try again."]);
    }
    if (picker.pendingPage !== null) {
      return h.div(
        [h.Class(PRINT_PICKER_GRID)],
        Array.from({ length: 4 }, () => skeletonPrintTile(h)),
      );
    }
    return h.div([h.Class("text-label text-lichen")], ["No printings found."]);
  }

  return windowedGrid(h, {
    model: model.printGrid,
    toGridMessage: (message) => GotPrintGridMessage({ message }),
    items: picker.prints,
    columns: PRINT_GRID_COLUMNS,
    itemToKey: (print) => print.id,
    itemToView: (print) => printTile(picker.cardId, print, h),
    rowClass: PRINT_PICKER_ROW,
    containerClass: "max-h-[min(60vh,720px)] w-fit",
    testId: PRINT_GRID_ID,
  });
}

function printPicker(model: DeckBuilderSubmodel, h: HtmlBuilder<ViewMessage>): Html {
  const picker = model.printPicker;

  return modalDialog(
    h,
    {
      model: model.printDialog,
      toDialogMessage: (message) => GotPrintDialogMessage({ message }),
      panel: "w-fit max-w-[90vw]",
      testId: PRINT_DIALOG_ID,
    },
    (render) =>
      picker == null
        ? []
        : [
            h.div(
              [h.Class("flex items-center justify-between gap-lg")],
              [
                h.div([...render.title, h.Class("font-semibold text-body")], ["Choose printing"]),
                button(h, { testId: "close-print-picker", variant: "ghost", attrs: [...render.closeButton] }, [
                  "Close",
                ]),
              ],
            ),
            printPickerBody(model, picker, h),
          ],
  );
}

function offIdentity(model: DeckBuilderSubmodel, card: DeckBuilderSubmodel["pool"][number]): boolean {
  if (!model.commander.id) return false;
  const identity = model.known[model.commander.id]?.color_identity ?? [];
  return card.color_identity.some((c) => !identity.includes(c));
}

function poolTile(
  model: DeckBuilderSubmodel,
  card: DeckBuilderSubmodel["pool"][number],
  h: HtmlBuilder<ViewMessage>,
): Html {
  const print = model.preferredPrint[card.id] ?? card.default_print;
  return h.button(
    [
      // Key by oracle id: BindBuilderCardPointer captures cardId at mount; without a
      // stable key, snabbdom reuses the node after list churn and clicks keep the old id.
      h.Key(card.id),
      h.Type("button"),
      h.DataAttribute("testid", `pool-card-${card.id}`),
      h.Class(cn(POOL_CARD, offIdentity(model, card) && "opacity-40")),
      h.OnMount(BindBuilderCardPointer({ cardId: card.id, kind: "pool" })),
    ],
    [
      builderCardArt(print, card.name, CARD_ART, "grid", h),
      // One line, ellipsised: the windowed grid needs every tile the same height, and a long card
      // name that wrapped would push its art out of the row. `w-full` because `items-center`
      // otherwise shrinks the span to its text and leaves nothing to truncate. No `title` — the
      // full name is what the hover preview is for.
      h.span([h.Class("w-full truncate text-center leading-[1.1]")], [`${card.legendary ? "★ " : ""}${card.name}`]),
    ],
  );
}

function skeletonTile(h: HtmlBuilder<ViewMessage>): Html {
  return h.div(
    [h.Class(cn(POOL_CARD, "pointer-events-none cursor-default"))],
    [
      h.div([h.Class(cn(CARD_ART, "animate-skeleton bg-white/8"))], []),
      h.div([h.Class("h-2.5 w-[70%] animate-skeleton rounded-[3px] bg-white/8")], []),
    ],
  );
}

/** Classes on one row of the windowed pool grid. The column count is measured, so it arrives as an
 *  inline `grid-template-columns` rather than a `grid-cols-*` class. */
const POOL_ROW = "gap-md";
const POOL_SKELETON_GRID = "grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] content-start gap-md";

/** The pool's scrolling area. Windowed, because the catalog runs to tens of thousands of cards and
 *  every tile that renders also fetches its art. */
function poolBody(model: DeckBuilderSubmodel, scrollLocked: boolean, h: HtmlBuilder<ViewMessage>): Html {
  if (model.pool.length === 0) {
    if (model.searching)
      return h.div(
        [h.Class(POOL_SKELETON_GRID)],
        Array.from({ length: 10 }, () => skeletonTile(h)),
      );
    return h.div([h.Class("text-label text-lichen")], ["No cards match."]);
  }

  return windowedGrid(h, {
    model: model.poolGrid,
    toGridMessage: (message) => GotPoolGridMessage({ message }),
    items: model.pool,
    columns: poolGridColumns(model.poolWidth),
    itemToKey: (card) => card.id,
    itemToView: (card) => poolTile(model, card, h),
    rowClass: POOL_ROW,
    rowStyle: { "grid-template-columns": `repeat(${poolGridColumns(model.poolWidth)},minmax(0,1fr))` },
    // VirtualList writes `overflow: auto` on the container as an inline style, so freezing the pool
    // behind the print picker needs `!important` to reach past it.
    containerClass: cn("min-h-0 flex-1", scrollLocked && "overflow-hidden!"),
    testId: POOL_GRID_ID,
  });
}

export type ViewInputs = {
  readonly chrome: AppChromeMeta;
  readonly username: string;
  readonly meGravatarHash: string | null;
  readonly accountMenu: Menu.Model;
};

export const view = Submodel.defineView<DeckBuilderSubmodel, ViewMessage, ViewInputs>((model, viewInputs, h) => {
  const rows = sortedDeckList(model.entries, model.known);
  const count = deckCount(model.entries);
  const backgroundScrollLocked = model.printPicker != null;

  return shellFrame(h, {
    atmosphere: "shell",
    title: model.editingId == null ? "New deck" : "Edit deck",
    chrome: viewInputs.chrome,
    lockStageScroll: true,
    leading: button(h, { testId: "builder-cancel", onClick: RequestedBuilderCancel(), variant: "ghost" }, ["Cancel"]),
    trailing: h.div(
      [h.Class("flex items-center gap-sm")],
      [
        button(
          h,
          {
            testId: "save-deck",
            disabled: model.saving,
            onClick: SubmittedDeckSave(),
            variant: "primary",
            class: "shrink-0",
          },
          [model.saving ? "Saving…" : "Save deck"],
        ),
        accountChrome(h, {
          username: viewInputs.username,
          gravatarHash: viewInputs.meGravatarHash,
          menu: viewInputs.accountMenu,
          toMenuMessage: (message) => GotAccountMenuMessage({ message }),
          showLeaderboardLink: true,
        }),
      ],
    ),
    stage: h.div(
      [
        // Fill the contained shell stage (not h-dvh): header + 100dvh overflowed the viewport and scrolled the page.
        h.Class(
          "grid h-full min-h-0 flex-1 grid-cols-[minmax(0,1fr)_minmax(220px,min(32vw,360px))] grid-rows-[minmax(0,1fr)] gap-5 overflow-hidden",
        ),
        h.DataAttribute("testid", "deck-builder-page"),
      ],
      [
        h.section(
          [h.Class("flex min-h-0 min-w-0 flex-col")],
          [
            h.h2([h.Class("m-0 font-display text-title tracking-display")], ["Card pool"]),
            h.div(
              [h.Class("text-label text-lichen"), h.DataAttribute("testid", "builder-pool-hint")],
              ["Click to add. Right-click or long-press for print and other options. Only basics may exceed one copy."],
            ),
            h.label([h.Class("sr-only"), h.For("pool-search")], ["Search card pool"]),
            input(h, {
              id: "pool-search",
              type: "search",
              value: model.query,
              placeholder: "Search name, type, subtype, color, set, tag…",
              onInput: (query) => ChangedBuilderQuery({ query }),
              class: "mt-2 w-full",
            }),
            h.div(
              [
                // The grid inside is what scrolls; this wrapper only bounds it and is what the
                // width observer measures, since the scrollport itself belongs to VirtualList.
                h.Class("mt-3 flex min-h-0 min-w-0 flex-1 flex-col"),
                h.DataAttribute("testid", "builder-pool-measure"),
                h.OnMount(ObservePoolWidth()),
              ],
              [poolBody(model, backgroundScrollLocked, h)],
            ),
          ],
        ),
        h.aside(
          [h.Class("flex min-h-0 min-w-0 flex-col gap-3")],
          [
            h.label([h.Class("sr-only"), h.For("deck-name")], ["Deck name"]),
            input(h, {
              id: "deck-name",
              testId: "deck-name",
              value: model.name,
              onInput: (name) => ChangedBuilderName({ name }),
              class: "w-full",
              attrs: [h.Disabled(model.loadingDeck)],
            }),
            model.loadingDeck
              ? h.div(
                  [
                    h.DataAttribute("testid", "builder-deck-loading"),
                    h.Class("flex flex-1 items-center justify-center text-label text-lichen"),
                  ],
                  ["Loading deck…"],
                )
              : null,
            model.loadingDeck ? null : h.div([h.Class("text-label text-lichen")], ["Commander"]),
            model.loadingDeck
              ? null
              : model.commander.id === ""
                ? h.div(
                    [h.Class("text-label text-lichen")],
                    ["Right-click or long-press a legendary creature to set commander or choose its art."],
                  )
                : h.button(
                    [
                      h.Key(model.commander.id),
                      h.Type("button"),
                      h.DataAttribute("testid", "builder-commander"),
                      h.Class(
                        "flex w-full cursor-pointer items-center gap-sm rounded-control border border-vine bg-glass-dim px-sm py-xs text-left",
                      ),
                      h.OnMount(BindBuilderCardPointer({ cardId: model.commander.id, kind: "commander" })),
                    ],
                    [
                      builderCardArt(
                        model.commander.print,
                        model.known[model.commander.id]?.name ?? model.commander.id,
                        "aspect-[0.72] w-10 rounded-focus object-cover",
                        "thumb",
                        h,
                      ),
                      h.span(
                        [h.Class("min-w-0 flex-1 truncate font-semibold")],
                        [`★ ${model.known[model.commander.id]?.name ?? model.commander.id}`],
                      ),
                    ],
                  ),
            model.loadingDeck
              ? null
              : h.div(
                  [h.Class("flex items-center justify-between gap-sm")],
                  [
                    h.b([], ["Cards"]),
                    h.span(
                      [h.Class(cn("shrink-0 text-caution-amber", count === DECK_SIZE && "text-vine"))],
                      [`${count}/${DECK_SIZE}${model.commander.id ? " + commander" : ""}`],
                    ),
                  ],
                ),
            model.loadingDeck
              ? null
              : h.div(
                  [
                    h.Class(
                      cn(
                        "flex max-h-[40vh] min-h-0 flex-1 flex-col gap-1 overscroll-contain",
                        backgroundScrollLocked ? "overflow-hidden" : "overflow-y-auto",
                      ),
                    ),
                    h.DataAttribute("testid", "builder-decklist-scroll"),
                  ],
                  [
                    rows.length === 0
                      ? h.div(
                          [
                            h.DataAttribute("testid", "builder-decklist-empty"),
                            h.Class("flex flex-1 items-center justify-center p-md text-center text-label text-lichen"),
                          ],
                          ["Cards you add appear here. Click a pool card to add it."],
                        )
                      : null,
                    ...rows.map((row) =>
                      h.button(
                        [
                          // Key by oracle id so removing a row remounts BindBuilderCardPointer
                          // for the next card (Mount args are captured once at insert).
                          h.Key(row.id),
                          h.Type("button"),
                          h.DataAttribute("testid", `deck-row-${row.id}`),
                          h.Class(DECK_ROW),
                          h.OnMount(BindBuilderCardPointer({ cardId: row.id, kind: "deck" })),
                        ],
                        [
                          builderCardArt(
                            row.print,
                            "",
                            "aspect-[0.72] w-7 shrink-0 rounded-[3px] object-cover",
                            "thumb",
                            h,
                          ),
                          h.span(
                            [h.Class("min-w-0 flex-1 truncate")],
                            [
                              `${row.legendary ? "★ " : ""}${row.name}`,
                              row.id === model.commander.id
                                ? h.span([h.Class("text-label text-lichen")], [" (commander)"])
                                : null,
                            ],
                          ),
                          h.span([h.Class("shrink-0 text-label text-lichen")], [`×${row.count}`]),
                        ],
                      ),
                    ),
                  ],
                ),
            // Always rendered: Dialog opens and closes the <dialog> element itself, so it has to
            // stay in the tree. `model.discardDialog.isOpen` is what makes the prompt visible.
            confirmDialog(h, {
              model: model.discardDialog,
              toDialogMessage: (message) => GotDiscardDialogMessage({ message }),
              title: "Discard changes?",
              body: "Everything you've edited since the deck loaded will be lost.",
              confirmLabel: "Discard",
              danger: true,
              onConfirm: ConfirmedBuilderDiscard(),
              testId: "builder-discard-confirm",
            }),
            model.problems.length === 0
              ? null
              : h.div(
                  [h.Role("alert"), h.DataAttribute("testid", "deck-problems"), h.Class(alertClass("text-burn-red"))],
                  [...model.problems.map((problem) => h.div([h.Class("text-caption")], [problem]))],
                ),
          ],
        ),
        hoverPreview(model, h),
        contextMenu(model, h),
        printPicker(model, h),
      ],
    ),
  });
});
