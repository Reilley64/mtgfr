import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { CatalogCardSchema } from "../../../../lib/deck-builder/cards";
import { ScryfallPrintSchema } from "../../../../lib/deck-builder/scryfall";
import { DeckDetail } from "../../../../lib/wire/types";

export const BuilderMenuTargetKind = S.Union([S.Literal("pool"), S.Literal("deck"), S.Literal("commander")]);
export type BuilderMenuTargetKind = typeof BuilderMenuTargetKind.Type;

export const BuilderMenuActionSchema = S.Union([
  S.Struct({ kind: S.Literal("add"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("remove"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("fill"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("setCommander"), cardId: S.String }),
  S.Struct({ kind: S.Literal("choosePrint"), cardId: S.String, addOnPick: S.Boolean }),
]);
export type BuilderMenuActionSchema = typeof BuilderMenuActionSchema.Type;

export const BuilderMenuItemSchema = S.Struct({
  action: BuilderMenuActionSchema,
  label: S.String,
});
export type BuilderMenuItemSchema = typeof BuilderMenuItemSchema.Type;

export const ChangedBuilderName = m("ChangedBuilderName", { name: S.String });
export const ChangedBuilderQuery = m("ChangedBuilderQuery", { query: S.String });
export const RequestedNextBuilderPage = m("RequestedNextBuilderPage");
export const ReceivedBuilderSearchPage = m("ReceivedBuilderSearchPage", {
  cards: S.Array(CatalogCardSchema),
  offset: S.Number,
  query: S.String,
});
export const BuilderSearchFailed = m("BuilderSearchFailed");
export const ReceivedDeckForBuilder = m("ReceivedDeckForBuilder", { deck: DeckDetail });
export const DeckBuilderLoadFailed = m("DeckBuilderLoadFailed", { message: S.String });
export const HydratedBuilderCards = m("HydratedBuilderCards", { cards: S.Array(CatalogCardSchema) });
export const AddedBuilderCard = m("AddedBuilderCard", { card: CatalogCardSchema });
export const RemovedBuilderCard = m("RemovedBuilderCard", { id: S.String });
export const SetBuilderCommander = m("SetBuilderCommander", { card: S.NullOr(CatalogCardSchema) });
export const OpenedBuilderPrintPicker = m("OpenedBuilderPrintPicker", { addOnPick: S.Boolean, cardId: S.String });
export const ReceivedBuilderPrints = m("ReceivedBuilderPrints", {
  cardId: S.String,
  prints: S.Array(ScryfallPrintSchema),
});
export const BuilderPrintSearchFailed = m("BuilderPrintSearchFailed", { cardId: S.String });
export const PickedBuilderPrint = m("PickedBuilderPrint", { cardId: S.String, print: S.String });
export const ClosedBuilderPrintPicker = m("ClosedBuilderPrintPicker");
export const SubmittedDeckSave = m("SubmittedDeckSave");
export const DeckSaved = m("DeckSaved");
export const DeckSaveFailed = m("DeckSaveFailed", { problems: S.Array(S.String) });

/** Player clicked Cancel — if dirty open a discard confirm, otherwise navigate home. */
export const RequestedBuilderCancel = m("RequestedBuilderCancel");
/** Player confirmed discarding unsaved changes. */
export const ConfirmedBuilderDiscard = m("ConfirmedBuilderDiscard");
/** Player dismissed the discard confirmation without discarding. */
export const CancelledBuilderDiscard = m("CancelledBuilderDiscard");
/** Navigation away from the builder completed — handled as a no-op. */
export const NavigatedAwayFromBuilder = m("NavigatedAwayFromBuilder");

/** Cursor-follow card preview (Solid HoverPreview). Print resolved in update. */
export const MovedBuilderHover = m("MovedBuilderHover", {
  id: S.String,
  x: S.Number,
  y: S.Number,
});
export const ClearedBuilderHover = m("ClearedBuilderHover");

/** Right-click / long-press menu — items are built in update from the live model. */
export const OpenedBuilderMenu = m("OpenedBuilderMenu", {
  cardId: S.String,
  kind: BuilderMenuTargetKind,
  x: S.Number,
  y: S.Number,
});
export const ClosedBuilderMenu = m("ClosedBuilderMenu");
export const RanBuilderMenuAction = m("RanBuilderMenuAction", { action: BuilderMenuActionSchema });

/** Click on a pool tile / deck row / commander chip (Solid click-to-add / remove / clear). */
export const ActivatedBuilderTarget = m("ActivatedBuilderTarget", {
  cardId: S.String,
  kind: BuilderMenuTargetKind,
});

export const Message = S.Union([
  ChangedBuilderName,
  ChangedBuilderQuery,
  RequestedNextBuilderPage,
  ReceivedBuilderSearchPage,
  BuilderSearchFailed,
  ReceivedDeckForBuilder,
  DeckBuilderLoadFailed,
  HydratedBuilderCards,
  AddedBuilderCard,
  RemovedBuilderCard,
  SetBuilderCommander,
  OpenedBuilderPrintPicker,
  ReceivedBuilderPrints,
  BuilderPrintSearchFailed,
  PickedBuilderPrint,
  ClosedBuilderPrintPicker,
  SubmittedDeckSave,
  DeckSaved,
  DeckSaveFailed,
  MovedBuilderHover,
  ClearedBuilderHover,
  OpenedBuilderMenu,
  ClosedBuilderMenu,
  RanBuilderMenuAction,
  ActivatedBuilderTarget,
  RequestedBuilderCancel,
  ConfirmedBuilderDiscard,
  CancelledBuilderDiscard,
  NavigatedAwayFromBuilder,
]);
export type Message = typeof Message.Type;
