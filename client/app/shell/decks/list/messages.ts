import { Schema as S } from "effect";
import { m } from "foldkit/message";
import { CatalogCardSchema } from "../../../../lib/deck-builder/cards";
import { DeckSummary } from "../../../../lib/wire/types";

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
/** Player dismissed the confirmation dialog without deleting. */
export const CancelledDeckDelete = m("CancelledDeckDelete");
/** Player confirmed the deletion — fires the DeleteDeck command. */
export const RequestedDeckDelete = m("RequestedDeckDelete", { id: S.Number });
export const DeckDeleted = m("DeckDeleted");
export const DeckDeleteFailed = m("DeckDeleteFailed", { message: S.String });

export const Message = S.Union([
  RequestedDecksRefresh,
  ReceivedDecks,
  DecksLoadFailed,
  ReceivedDeckListCommanders,
  ChangedDeckListSearch,
  OpenedDeckListMenu,
  ClosedDeckListMenu,
  AskedDeckDelete,
  CancelledDeckDelete,
  RequestedDeckDelete,
  DeckDeleted,
  DeckDeleteFailed,
]);
export type Message = typeof Message.Type;
