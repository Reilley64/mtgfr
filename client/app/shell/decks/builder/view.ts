import { Effect, Option, Queue, Schema as S, Stream } from "effect";
import { Submodel } from "foldkit";
import { type Html, html } from "foldkit/html";
import * as Mount from "foldkit/mount";
import { cn } from "../../../domain/cn";
import { cardHoverPreviewView } from "../../../domain/deck-builder/card-hover-preview";
import { DECK_SIZE, deckCount, sortedDeckList } from "../../../domain/deck-builder/cards";
import { formatReleasedAt } from "../../../domain/deck-builder/print";
import type { ScryfallPrint } from "../../../domain/deck-builder/scryfall";
import { type AppChromeMeta, appVersionBadge } from "../../../domain/ui/app-version";
import { buttonClass } from "../../../domain/ui/buttonClass";
import { cardArt } from "../../../domain/ui/card-art";
import { confirmDialog, OpenDialogAsModal } from "../../../domain/ui/confirmDialog";
import { feltClass, fieldClass } from "../../../domain/ui/surfaces";
import type { CardArtTick, ModalOpened } from "../../../messages";
import {
  ActivatedBuilderTarget,
  type BuilderProxyArtTargetKind,
  CancelledBuilderDiscard,
  ChangedBuilderName,
  ChangedBuilderProxyArtUrl,
  ChangedBuilderQuery,
  ClearedBuilderHover,
  ClearedBuilderProxyArt,
  ClosedBuilderMenu,
  ClosedBuilderPrintPicker,
  ClosedBuilderProxyArtPicker,
  ConfirmedBuilderDiscard,
  type Message,
  MovedBuilderHover,
  OpenedBuilderMenu,
  PickedBuilderPrint,
  RanBuilderMenuAction,
  RequestedBuilderCancel,
  RequestedNextBuilderPage,
  SubmittedBuilderProxyArt,
  SubmittedDeckSave,
} from "./messages";
import type { DeckBuilderSubmodel } from "./submodel";

export type ViewMessage = Message | typeof ModalOpened.Type | typeof CardArtTick.Type;

const h = html<ViewMessage>();

const CONTEXT_MENU_PRESS_MS = 500;

const LIST_ROW = "border border-vine-dim bg-glass-dim text-snow hover:bg-white/8";
const POOL_CARD = cn(
  LIST_ROW,
  "flex cursor-pointer flex-col items-center gap-1 rounded-hud p-sm text-caption focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const DECK_ROW = cn(
  LIST_ROW,
  "flex w-full cursor-pointer items-center gap-xs rounded-control px-sm py-1 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const MENU_ITEM =
  "cursor-pointer rounded-control border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine";
const PRINT_PICKER_COL = "w-[min(38vw,200px)]";
const PRINT_TILE = cn(
  PRINT_PICKER_COL,
  "flex cursor-pointer flex-col items-center gap-1.5 rounded-hud p-md text-label hover:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const PRINT_PICKER_GRID = "grid w-fit grid-cols-2 gap-md";
const PRINT_BADGE =
  "rounded-full border border-vine-dim bg-glass-dim px-[7px] py-px font-semibold text-chip text-lichen";
const PROXY_CHIP =
  "rounded-full border border-vine/50 bg-vine/12 px-[7px] py-px font-semibold text-chip text-vine";
const CARD_ART = cn("aspect-[0.72] w-full rounded-control object-cover");
const PRINT_SKELETON = cn(PRINT_PICKER_COL, "flex cursor-default flex-col items-center gap-1.5 p-md");

export const ObserveBuilderSentinel = Mount.defineStream(
  "ObserveBuilderSentinel",
  RequestedNextBuilderPage,
)((element) =>
  Stream.callback<typeof RequestedNextBuilderPage.Type>((queue) =>
    Effect.gen(function* () {
      yield* Effect.acquireRelease(
        Effect.sync(() => {
          if (typeof IntersectionObserver === "undefined") {
            Queue.offerUnsafe(queue, RequestedNextBuilderPage());
            return null;
          }

          const observer = new IntersectionObserver(
            (entries) => {
              if (entries[0]?.isIntersecting) Queue.offerUnsafe(queue, RequestedNextBuilderPage());
            },
            { rootMargin: "300px" },
          );
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
              Queue.offerUnsafe(
                queue,
                MovedBuilderHover({ id: args.cardId, kind: args.kind, x: event.clientX, y: event.clientY }),
              );
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

function proxyArtFor(model: DeckBuilderSubmodel, cardId: string): string | undefined {
  const entryProxyArt = model.entries[cardId]?.proxyArtUrl;
  if (entryProxyArt) return entryProxyArt;
  if (model.commander.id === cardId) return model.commander.proxyArtUrl;
  return undefined;
}

function proxyArtForTarget(
  model: DeckBuilderSubmodel,
  cardId: string,
  target: BuilderProxyArtTargetKind,
): string | undefined {
  if (target === "entry") return model.entries[cardId]?.proxyArtUrl;
  if (model.commander.id === cardId) return model.commander.proxyArtUrl;
  return undefined;
}

function builderCardArt(print: string, proxyArtUrl: string | undefined, alt: string, className: string): Html {
  return cardArt(h, { print, proxyArtUrl, alt, className });
}

function hoverPreview(model: DeckBuilderSubmodel): Html | null {
  const hover = model.hover;
  if (hover == null) return null;
  return cardHoverPreviewView(h, {
    hover,
    card: model.known[hover.id],
    testId: "builder-hover-preview",
  });
}

function contextMenu(model: DeckBuilderSubmodel): Html {
  const menu = model.menu;
  if (menu == null || model.printPicker != null || model.proxyArtPicker != null) return null;

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
          h.Class(
            "fixed top-(--y) left-(--x) z-41 flex min-w-[160px] flex-col rounded-hud border border-vine bg-forest-surface p-xs shadow-table",
          ),
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
                h.Class(MENU_ITEM),
              ],
              [item.label],
            ),
          ),
        ],
      ),
    ],
  );
}

function printTile(cardId: string, print: ScryfallPrint): Html {
  return h.button(
    [
      h.Type("button"),
      h.Class(PRINT_TILE),
      h.DataAttribute("testid", `print-tile-${print.id}`),
      h.OnClick(PickedBuilderPrint({ cardId, print: print.id })),
    ],
    [
      builderCardArt(print.id, undefined, `${print.set_name} #${print.collector_number}`, CARD_ART),
      h.div(
        [h.Class("flex w-full flex-wrap items-center justify-center gap-1")],
        [
          h.span([h.Class(PRINT_BADGE), h.Title(print.set_name)], [print.set.toUpperCase()]),
          h.span([h.Class(PRINT_BADGE)], [`#${print.collector_number}`]),
          h.span([h.Class(PRINT_BADGE)], [formatReleasedAt(print.released_at)]),
        ],
      ),
    ],
  );
}

function skeletonPrintTile(): Html {
  return h.div(
    [h.Class(cn(PRINT_SKELETON, "pointer-events-none"))],
    [
      h.div([h.Class(cn("aspect-[0.72] w-full animate-skeleton rounded-control bg-white/8"))], []),
      h.div([h.Class("h-2.5 w-[70%] animate-skeleton rounded-[3px] bg-white/8")], []),
    ],
  );
}

function printPicker(model: DeckBuilderSubmodel): Html {
  const picker = model.printPicker;
  if (picker == null) return null;

  return h.dialog(
    [
      h.DataAttribute("testid", "builder-print-picker"),
      h.Class(
        "m-auto w-fit max-w-[90vw] rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table backdrop:bg-black/60",
      ),
      h.OnMount(OpenDialogAsModal()),
      h.OnCancel(ClosedBuilderPrintPicker()),
    ],
    [
      h.div(
        [h.Class("flex w-fit max-w-full flex-col gap-md")],
        [
          h.div(
            [h.Class("flex items-center justify-between gap-lg")],
            [
              h.div([h.Class("font-semibold text-body")], ["Choose printing"]),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "close-print-picker"),
                  h.OnClick(ClosedBuilderPrintPicker()),
                  h.Class(buttonClass("ghost")),
                ],
                ["Close"],
              ),
            ],
          ),
          h.div(
            [
              h.Class(cn(PRINT_PICKER_GRID, "max-h-[min(60vh,720px)] overflow-y-auto overscroll-contain")),
              h.DataAttribute("testid", "builder-print-picker-scroll"),
            ],
            [
              !picker.loading && picker.error
                ? h.div(
                    [h.Class("col-span-2 text-burn-red text-label")],
                    ["Could not load printings. Close and try again."],
                  )
                : null,
              !picker.loading && !picker.error && picker.prints.length === 0
                ? h.div([h.Class("col-span-2 text-label text-lichen")], ["No printings found."])
                : null,
              ...(picker.loading
                ? Array.from({ length: 4 }, () => skeletonPrintTile())
                : picker.prints.map((p) => printTile(picker.cardId, p))),
            ],
          ),
        ],
      ),
    ],
  );
}

function proxyArtPicker(model: DeckBuilderSubmodel): Html {
  const picker = model.proxyArtPicker;
  if (picker == null) return null;

  const cardName = model.known[picker.cardId]?.name ?? picker.cardId;
  const hasSavedProxyArt = proxyArtForTarget(model, picker.cardId, picker.target) != null;
  const canSave = picker.error == null && picker.url.trim() !== "";

  return h.dialog(
    [
      h.DataAttribute("testid", "builder-proxy-art-picker"),
      h.Class(
        "m-auto w-[min(92vw,640px)] rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table backdrop:bg-black/60",
      ),
      h.OnMount(OpenDialogAsModal()),
      h.OnCancel(ClosedBuilderProxyArtPicker()),
    ],
    [
      h.div(
        [h.Class("flex flex-col gap-md")],
        [
          h.div(
            [h.Class("flex items-center justify-between gap-lg")],
            [
              h.div([h.Class("min-w-0")], [
                h.div([h.Class("font-semibold text-body")], ["Set proxy art"]),
                h.div([h.Class("truncate text-label text-lichen")], [cardName]),
              ]),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "close-proxy-art-picker"),
                  h.OnClick(ClosedBuilderProxyArtPicker()),
                  h.Class(buttonClass("ghost")),
                ],
                ["Close"],
              ),
            ],
          ),
          h.label([h.Class("sr-only"), h.For("builder-proxy-art-url")], ["Proxy art URL"]),
          h.input([
            h.Id("builder-proxy-art-url"),
            h.Type("url"),
            h.Value(picker.url),
            h.Autocomplete("off"),
            h.Spellcheck(false),
            h.Placeholder("https://example.com/card.png"),
            h.DataAttribute("testid", "builder-proxy-art-url"),
            h.OnInput((url) => ChangedBuilderProxyArtUrl({ url })),
            h.Class(fieldClass("w-full")),
          ]),
          picker.error == null
            ? null
            : h.div(
                [h.Role("alert"), h.DataAttribute("testid", "builder-proxy-art-error"), h.Class("text-burn-red text-caption")],
                [picker.error],
              ),
          h.div(
            [h.Class("flex flex-wrap justify-end gap-sm")],
            [
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "builder-proxy-art-clear"),
                  h.Disabled(!hasSavedProxyArt),
                  h.OnClick(ClearedBuilderProxyArt()),
                  h.Class(buttonClass("ghost")),
                ],
                ["Clear"],
              ),
              h.button(
                [
                  h.Type("button"),
                  h.DataAttribute("testid", "builder-proxy-art-save"),
                  h.Disabled(!canSave),
                  h.OnClick(SubmittedBuilderProxyArt()),
                  h.Class(buttonClass("primary")),
                ],
                ["Save"],
              ),
            ],
          ),
        ],
      ),
    ],
  );
}

function offIdentity(model: DeckBuilderSubmodel, card: DeckBuilderSubmodel["pool"][number]): boolean {
  if (!model.commander.id) return false;
  const identity = model.known[model.commander.id]?.color_identity ?? [];
  return card.color_identity.some((c) => !identity.includes(c));
}

function poolTile(model: DeckBuilderSubmodel, card: DeckBuilderSubmodel["pool"][number]): Html {
  const print = model.preferredPrint[card.id] ?? card.default_print;
  const proxyArtUrl = proxyArtFor(model, card.id);
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
      builderCardArt(print, proxyArtUrl, card.name, CARD_ART),
      h.span([h.Class("text-center leading-[1.1]")], [`${card.legendary ? "★ " : ""}${card.name}`]),
    ],
  );
}

function skeletonTile(): Html {
  return h.div(
    [h.Class(cn(POOL_CARD, "pointer-events-none cursor-default"))],
    [
      h.div([h.Class(cn(CARD_ART, "animate-skeleton bg-white/8"))], []),
      h.div([h.Class("h-2.5 w-[70%] animate-skeleton rounded-[3px] bg-white/8")], []),
    ],
  );
}

export type ViewInputs = {
  readonly chrome: AppChromeMeta;
};

export const view = Submodel.defineView<DeckBuilderSubmodel, ViewMessage, ViewInputs>((model, viewInputs) => {
  const rows = sortedDeckList(model.entries, model.known);
  const count = deckCount(model.entries);
  const backgroundScrollLocked = model.printPicker != null || model.proxyArtPicker != null;

  return h.main(
    [
      h.Class(
        feltClass(
          // h-dvh (not min-h-screen): shell must stay viewport-tall so the catalog/decklist overflow-y-auto hosts actually overflow.
          "grid h-dvh grid-cols-[minmax(0,1fr)_minmax(220px,min(32vw,360px))] grid-rows-[minmax(0,1fr)] gap-5 overflow-hidden p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]",
        ),
      ),
      h.DataAttribute("testid", "deck-builder-page"),
    ],
    [
      h.section(
        [h.Class("flex min-h-0 min-w-0 flex-col")],
        [
          h.h1([h.Class("m-0 text-title")], ["Card pool"]),
          h.div(
            [h.Class("text-label text-lichen"), h.DataAttribute("testid", "builder-pool-hint")],
            ["Click to add. Right-click or long-press for print and other options. Only basics may exceed one copy."],
          ),
          h.label([h.Class("sr-only"), h.For("pool-search")], ["Search card pool"]),
          h.input([
            h.Id("pool-search"),
            h.Type("search"),
            h.Value(model.query),
            h.Placeholder("Search name, type, subtype, color, set, tag…"),
            h.OnInput((query) => ChangedBuilderQuery({ query })),
            h.Class(fieldClass("mt-2 w-full")),
          ]),
          h.div(
            [
              h.Class(
                cn(
                  "mt-3 grid min-h-0 flex-1 grid-cols-[repeat(auto-fill,minmax(120px,1fr))] content-start gap-md overscroll-contain",
                  backgroundScrollLocked ? "overflow-hidden" : "overflow-y-auto",
                ),
              ),
              h.DataAttribute("testid", "builder-pool-scroll"),
            ],
            [
              ...model.pool.map((card) => poolTile(model, card)),
              ...(model.searching ? Array.from({ length: 10 }, () => skeletonTile()) : []),
              !model.searching && model.pool.length === 0
                ? h.div([h.Class("col-span-full text-label text-lichen")], ["No cards match."])
                : null,
              model.atEnd
                ? null
                : h.div(
                    [
                      h.Class("col-span-full h-px"),
                      h.DataAttribute("testid", "builder-scroll-sentinel"),
                      h.OnMount(ObserveBuilderSentinel()),
                    ],
                    [],
                  ),
            ],
          ),
        ],
      ),
      h.aside(
        [h.Class("flex min-h-0 min-w-0 flex-col gap-3")],
        [
          h.h2([h.Class("m-0 text-title")], [model.editingId == null ? "New deck" : "Edit deck"]),
          h.label([h.Class("sr-only"), h.For("deck-name")], ["Deck name"]),
          h.input([
            h.Id("deck-name"),
            h.DataAttribute("testid", "deck-name"),
            h.Value(model.name),
            h.OnInput((name) => ChangedBuilderName({ name })),
            h.Class(fieldClass("w-full")),
          ]),
          h.div([h.Class("text-label text-lichen")], ["Commander"]),
          model.commander.id === ""
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
                    model.commander.proxyArtUrl,
                    model.known[model.commander.id]?.name ?? model.commander.id,
                    "aspect-[0.72] w-10 rounded-focus object-cover",
                  ),
                  h.div(
                    [h.Class("flex min-w-0 flex-1 items-center gap-xs")],
                    [
                      h.span(
                        [h.Class("min-w-0 flex-1 truncate font-semibold")],
                        [`★ ${model.known[model.commander.id]?.name ?? model.commander.id}`],
                      ),
                      model.commander.proxyArtUrl
                        ? h.span([h.Class(PROXY_CHIP), h.DataAttribute("testid", "builder-commander-proxy-chip")], ["Proxy"])
                        : null,
                    ],
                  ),
                ],
              ),
          h.div(
            [h.Class("flex items-center justify-between gap-sm")],
            [
              h.b([], ["Cards"]),
              h.span(
                [h.Class(cn("shrink-0 text-caution-amber", count === DECK_SIZE && "text-vine"))],
                [`${count}/${DECK_SIZE}${model.commander.id ? " + commander" : ""}`],
              ),
            ],
          ),
          h.div(
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
                      model.entries[row.id]?.proxyArtUrl,
                      "",
                      "aspect-[0.72] w-7 shrink-0 rounded-[3px] object-cover",
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
                    model.entries[row.id]?.proxyArtUrl
                      ? h.span([h.Class(PROXY_CHIP), h.DataAttribute("testid", `builder-deck-proxy-chip-${row.id}`)], ["Proxy"])
                      : null,
                    h.span([h.Class("shrink-0 text-label text-lichen")], [`×${row.count}`]),
                  ],
                ),
              ),
            ],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "save-deck"),
              h.Disabled(model.saving),
              h.OnClick(SubmittedDeckSave()),
              h.Class(buttonClass("primary")),
            ],
            [model.saving ? "Saving…" : "Save deck"],
          ),
          h.button(
            [
              h.Type("button"),
              h.DataAttribute("testid", "builder-cancel"),
              h.OnClick(RequestedBuilderCancel()),
              h.Class(buttonClass("ghost")),
            ],
            ["Cancel"],
          ),
          model.confirmingDiscard
            ? confirmDialog(h, {
                title: "Discard changes?",
                body: "Everything you've edited since the deck loaded will be lost.",
                confirmLabel: "Discard",
                danger: true,
                onConfirm: ConfirmedBuilderDiscard(),
                onCancel: CancelledBuilderDiscard(),
                testId: "builder-discard-confirm",
              })
            : null,
          model.problems.length === 0
            ? null
            : h.div(
                [h.Role("alert"), h.DataAttribute("testid", "deck-problems"), h.Class("flex flex-col gap-[3px]")],
                [...model.problems.map((problem) => h.div([h.Class("text-burn-red text-caption")], [problem]))],
              ),
        ],
      ),
      hoverPreview(model),
      contextMenu(model),
      printPicker(model),
      proxyArtPicker(model),
      appVersionBadge(h, viewInputs.chrome),
    ],
  );
});
