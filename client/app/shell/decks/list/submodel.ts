import * as Dialog from "@foldkit/ui/dialog";
import { Schema as S } from "effect";
import { CatalogCardSchema } from "../../../domain/deck-builder/cards";
import { DeckSummary } from "../../../domain/wire/types";

export const DeckListSubmodel = S.Struct({
  searchQuery: S.String,
  contextMenu: S.NullOr(S.Struct({ deckId: S.Number, x: S.Number, y: S.Number })),
  knownCommanders: S.Record(S.String, CatalogCardSchema),
  decks: S.Array(DeckSummary),
  error: S.NullOr(S.String),
  loading: S.Boolean,
  /** Deck id the open delete confirmation is asking about, or null. */
  confirmingDeleteId: S.NullOr(S.Number),
  confirmDialog: Dialog.Model,
});

/** Document-unique id for the delete confirmation. Dialog keys its element, ARIA, and cleanup on it. */
export const DELETE_DIALOG_ID = "deck-delete-confirm";
export type DeckListSubmodel = typeof DeckListSubmodel.Type;

export function initialDeckListSubmodel(): DeckListSubmodel {
  return {
    searchQuery: "",
    contextMenu: null,
    knownCommanders: {},
    decks: [],
    error: null,
    loading: false,
    confirmingDeleteId: null,
    confirmDialog: Dialog.init({ id: DELETE_DIALOG_ID }),
  };
}
