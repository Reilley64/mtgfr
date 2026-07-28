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
  loading: S.Boolean,
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
