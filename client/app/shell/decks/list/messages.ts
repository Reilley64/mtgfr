import * as Dialog from "@foldkit/ui/dialog";
import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { CatalogCardSchema } from "../../../domain/deck-builder/cards";
import { DeckSummary } from "../../../domain/wire/types";

export const ChangedDeckListRoute = m("ChangedDeckListRoute");
export const RequestedDecksRefresh = m("RequestedDecksRefresh");
export const ReceivedDecks = m("ReceivedDecks", { decks: S.Array(DeckSummary) });
export const DecksLoadFailed = m("DecksLoadFailed", { message: S.String });
export const ReceivedDeckListCommanders = m("ReceivedDeckListCommanders", { cards: S.Array(CatalogCardSchema) });
export const ChangedDeckListSearch = m("ChangedDeckListSearch", { query: S.String });
export const OpenedDeckListMenu = m("OpenedDeckListMenu", {
  deckId: S.Number,
  x: S.Number,
  y: S.Number,
});
export const ClosedDeckListMenu = m("ClosedDeckListMenu");
/** Player clicked Delete on a deck row — open the confirmation dialog. */
export const AskedDeckDelete = m("AskedDeckDelete", { id: S.Number });
/** Delegation envelope for the delete confirmation's Dialog submodel. */
export const GotConfirmDialogMessage = m("GotConfirmDialogMessage", { message: Dialog.Message });
/** Player confirmed the deletion — fires the DeleteDeck command for `confirmingDeleteId`. */
export const RequestedDeckDelete = m("RequestedDeckDelete");
export const DeckDeleted = m("DeckDeleted");
export const DeckDeleteFailed = m("DeckDeleteFailed", { message: S.String });

export const Message = S.Union([
  ChangedDeckListRoute,
  RequestedDecksRefresh,
  ReceivedDecks,
  DecksLoadFailed,
  ReceivedDeckListCommanders,
  ChangedDeckListSearch,
  OpenedDeckListMenu,
  ClosedDeckListMenu,
  AskedDeckDelete,
  GotConfirmDialogMessage,
  RequestedDeckDelete,
  DeckDeleted,
  DeckDeleteFailed,
]);
export type Message = typeof Message.Type;
