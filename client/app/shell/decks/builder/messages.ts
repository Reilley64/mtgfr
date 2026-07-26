import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { CatalogCardSchema } from "../../../domain/deck-builder/cards";
import { ScryfallPrintSchema } from "../../../domain/deck-builder/scryfall";
import { DeckDetail } from "../../../domain/wire/types";

export const BuilderMenuTargetKind = S.Union([S.Literal("pool"), S.Literal("deck"), S.Literal("commander")]);
export type BuilderMenuTargetKind = typeof BuilderMenuTargetKind.Type;

export const BuilderProxyArtTargetKind = S.Union([S.Literal("entry"), S.Literal("commander")]);
export type BuilderProxyArtTargetKind = typeof BuilderProxyArtTargetKind.Type;

export const BuilderMenuActionSchema = S.Union([
  S.Struct({ kind: S.Literal("add"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("remove"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("fill"), cardId: S.String, count: S.Number }),
  S.Struct({ kind: S.Literal("setCommander"), cardId: S.String }),
  S.Struct({ kind: S.Literal("choosePrint"), cardId: S.String, addOnPick: S.Boolean }),
  S.Struct({ kind: S.Literal("setProxyArt"), cardId: S.String, target: BuilderProxyArtTargetKind }),
]);
export type BuilderMenuActionSchema = typeof BuilderMenuActionSchema.Type;

export const BuilderMenuItemSchema = S.Struct({
  action: BuilderMenuActionSchema,
  label: S.String,
});
export type BuilderMenuItemSchema = typeof BuilderMenuItemSchema.Type;

export const ChangedBuilderName = m("ChangedBuilderName", { name: S.String });
export const ChangedBuilderQuery = m("ChangedBuilderQuery", { query: S.String });
export const ChangedBuilderRoute = m("ChangedBuilderRoute", { editingId: S.NullOr(S.String) });
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
export const OpenedBuilderProxyArtPicker = m("OpenedBuilderProxyArtPicker", {
  cardId: S.String,
  target: BuilderProxyArtTargetKind,
});
export const ChangedBuilderProxyArtUrl = m("ChangedBuilderProxyArtUrl", { url: S.String });
export const SubmittedBuilderProxyArt = m("SubmittedBuilderProxyArt");
export const ClearedBuilderProxyArt = m("ClearedBuilderProxyArt");
export const ClosedBuilderProxyArtPicker = m("ClosedBuilderProxyArtPicker");
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
  kind: BuilderMenuTargetKind,
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
  ChangedBuilderRoute,
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
  OpenedBuilderProxyArtPicker,
  ChangedBuilderProxyArtUrl,
  SubmittedBuilderProxyArt,
  ClearedBuilderProxyArt,
  ClosedBuilderProxyArtPicker,
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
