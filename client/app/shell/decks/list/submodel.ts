import { Schema as S } from "effect";
import { CatalogCardSchema } from "../../../../lib/deck-builder/cards";
import { DeckSummary } from "../../../../lib/wire/types";
import { DeckListHover } from "./hover";

export const DeckListSubmodel = S.Struct({
  hover: S.NullOr(DeckListHover),
  searchQuery: S.String,
  contextMenu: S.NullOr(S.Struct({ deckId: S.Number, x: S.Number, y: S.Number })),
  knownCommanders: S.Record(S.String, CatalogCardSchema),
  decks: S.Array(DeckSummary),
  error: S.NullOr(S.String),
  loading: S.Boolean,
  /** Deck id whose delete confirmation dialog is open, or null. */
  confirmingDeleteId: S.NullOr(S.Number),
});
export type DeckListSubmodel = typeof DeckListSubmodel.Type;

export function initialDeckListSubmodel(): DeckListSubmodel {
  return {
    hover: null,
    searchQuery: "",
    contextMenu: null,
    knownCommanders: {},
    decks: [],
    error: null,
    loading: false,
    confirmingDeleteId: null,
  };
}
