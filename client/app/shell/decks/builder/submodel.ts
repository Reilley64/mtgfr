import * as Dialog from "@foldkit/ui/dialog";
import * as VirtualList from "@foldkit/ui/virtualList";
import { Schema as S } from "effect";
import { CatalogCardSchema } from "../../../domain/deck-builder/cards";
import { ScryfallPrintSchema } from "../../../domain/deck-builder/scryfall";
import { BuilderMenuItemSchema } from "./messages";

/** Document-unique id for the discard confirmation. Dialog keys its element, ARIA, and cleanup on it. */
export const DISCARD_DIALOG_ID = "builder-discard-confirm";

/** Document-unique id for the print picker. Doubles as its `data-testid`. */
export const PRINT_DIALOG_ID = "builder-print-picker";

/** Document-unique id for the print picker's scrolling grid. Doubles as its `data-testid`. */
export const PRINT_GRID_ID = "builder-print-picker-scroll";

/** Print tiles per row. Matches `grid-cols-2` on the picker's rows in `view.ts`. */
export const PRINT_GRID_COLUMNS = 2;

// The windowed grid needs one row height for the whole list, so a print tile's vertical pieces are
// spelled out here. They mirror `PRINT_TILE` / `CARD_ART` / `PRINT_BADGE_ROW` in `view.ts`; change
// one and change the other.
const TILE_PADDING_PX = 20; // p-md, top and bottom
const TILE_GAP_PX = 6; // gap-1.5 between the art and the badges
const BADGE_ROW_PX = 40; // h-10 — two badge lines, reserved so every tile is the same height
const ROW_GAP_PX = 10; // gap-md between rows, which the row's own height has to carry
const CARD_ASPECT = 0.72; // CARD_ART's aspect-[0.72]
const TILE_WIDTH_VW = 0.38; // PRINT_PICKER_COL's w-[min(38vw,200px)]
const TILE_WIDTH_MAX_PX = 200;

/** Height of one row of print tiles. The tile's width is a fraction of the viewport and its art is
 *  a fixed aspect ratio of that width, so the row height follows from the viewport alone. */
export function printGridRowHeightPx(viewportWidth: number): number {
  const tileWidth = Math.min(viewportWidth * TILE_WIDTH_VW, TILE_WIDTH_MAX_PX);
  const artHeight = Math.max(0, tileWidth - TILE_PADDING_PX) / CARD_ASPECT;
  return TILE_PADDING_PX + artHeight + TILE_GAP_PX + BADGE_ROW_PX + ROW_GAP_PX;
}

/** Viewport width in px, or a desktop-sized default where there is no window (SSR, tests). */
export function viewportWidthPx(): number {
  return typeof window === "undefined" ? 1024 : window.innerWidth;
}

/** Document-unique id for the card pool's scrolling grid. Doubles as its `data-testid`. */
export const POOL_GRID_ID = "builder-pool-scroll";

// The pool grid sizes itself to its column, so its geometry comes from the measured container width
// rather than the viewport. These mirror `POOL_CARD` / `CARD_ART` / `POOL_ROW` in `view.ts`.
const POOL_TILE_MIN_PX = 120; // the pool's minimum tile width, the floor the column count divides by
const POOL_GAP_PX = 10; // gap-md, between columns and between rows
const POOL_TILE_BORDER_PX = 2; // border, top + bottom
const POOL_TILE_PADDING_PX = 16; // p-sm, top + bottom
const POOL_TILE_GAP_PX = 4; // gap-1 between the art and the name
const POOL_NAME_PX = 14; // one line of text-caption at leading-[1.1], rounded up to a whole pixel

/** Tiles per pool row. The row sets this explicitly, so the tiles narrow rather than rewrapping when
 *  a scrollbar takes a few pixels the measurement did not see. */
export function poolGridColumns(containerWidth: number): number {
  return Math.max(1, Math.floor((containerWidth + POOL_GAP_PX) / (POOL_TILE_MIN_PX + POOL_GAP_PX)));
}

/** Height of one row of pool tiles. Rounds up rather than down: a row taller than its tiles just
 *  reads as more gap, while a row shorter than its tiles clips them. */
export function poolGridRowHeightPx(containerWidth: number): number {
  const columns = poolGridColumns(containerWidth);
  const tileWidth = (containerWidth - (columns - 1) * POOL_GAP_PX) / columns;
  const artWidth = Math.max(0, tileWidth - POOL_TILE_PADDING_PX - POOL_TILE_BORDER_PX);
  const tileHeight =
    POOL_TILE_BORDER_PX + POOL_TILE_PADDING_PX + artWidth / CARD_ASPECT + POOL_TILE_GAP_PX + POOL_NAME_PX;
  return Math.ceil(tileHeight) + POOL_GAP_PX;
}

export const DeckEntry = S.Struct({
  count: S.Number,
  print: S.String,
});
export type DeckEntry = typeof DeckEntry.Type;

export const BuilderCommander = S.Struct({
  id: S.String,
  print: S.String,
});
export type BuilderCommander = typeof BuilderCommander.Type;

export const BuilderPrintPicker = S.Struct({
  addOnPick: S.Boolean,
  cardId: S.String,
  error: S.Boolean,
  /** URL of the printings page in flight, or null when every page has landed. Prints arrive a page
   *  at a time and append, so this doubles as the "still loading" flag and as the token that tells
   *  this picker's pages apart from a previous picker's still-in-flight chain. */
  pendingPage: S.NullOr(S.String),
  prints: S.Array(ScryfallPrintSchema),
});
export type BuilderPrintPicker = typeof BuilderPrintPicker.Type;

export const BuilderHover = S.Struct({
  id: S.String,
  print: S.String,
  x: S.Number,
  y: S.Number,
});
export type BuilderHover = typeof BuilderHover.Type;

export const BuilderContextMenu = S.Struct({
  items: S.Array(BuilderMenuItemSchema),
  title: S.String,
  x: S.Number,
  y: S.Number,
});
export type BuilderContextMenu = typeof BuilderContextMenu.Type;

export const DeckBuilderSubmodel = S.Struct({
  atEnd: S.Boolean,
  commander: BuilderCommander,
  discardDialog: Dialog.Model,
  dirty: S.Boolean,
  editingId: S.NullOr(S.String),
  entries: S.Record(S.String, DeckEntry),
  hover: S.NullOr(BuilderHover),
  known: S.Record(S.String, CatalogCardSchema),
  loadingDeck: S.Boolean,
  menu: S.NullOr(BuilderContextMenu),
  name: S.String,
  offset: S.Number,
  pool: S.Array(CatalogCardSchema),
  poolGrid: VirtualList.Model,
  /** Measured width of the pool column, or 0 before its ResizeObserver reports. Both the column
   *  count and the row height follow from it, so the grid waits for it. */
  poolWidth: S.Number,
  preferredPrint: S.Record(S.String, S.String),
  printDialog: Dialog.Model,
  printGrid: VirtualList.Model,
  printPicker: S.NullOr(BuilderPrintPicker),
  problems: S.Array(S.String),
  query: S.String,
  saving: S.Boolean,
  searching: S.Boolean,
});
export type DeckBuilderSubmodel = typeof DeckBuilderSubmodel.Type;

export function initialDeckBuilderSubmodel(editingId: string | null = null): DeckBuilderSubmodel {
  return {
    atEnd: false,
    commander: { id: "", print: "" },
    discardDialog: Dialog.init({ id: DISCARD_DIALOG_ID }),
    dirty: false,
    editingId,
    entries: {},
    hover: null,
    known: {},
    loadingDeck: editingId !== null,
    menu: null,
    name: "New deck",
    offset: 0,
    pool: [],
    poolGrid: VirtualList.init({ id: POOL_GRID_ID, rowHeightPx: poolGridRowHeightPx(0) }),
    poolWidth: 0,
    preferredPrint: {},
    printDialog: Dialog.init({ id: PRINT_DIALOG_ID }),
    printGrid: VirtualList.init({ id: PRINT_GRID_ID, rowHeightPx: printGridRowHeightPx(viewportWidthPx()) }),
    printPicker: null,
    problems: [],
    query: "",
    saving: false,
    searching: true,
  };
}
