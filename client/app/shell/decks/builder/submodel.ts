import { Schema as S } from "effect";
import { CatalogCardSchema } from "../../../domain/deck-builder/cards";
import { ScryfallPrintSchema } from "../../../domain/deck-builder/scryfall";
import { BuilderMenuItemSchema } from "./messages";

export const DeckEntry = S.Struct({
  count: S.Number,
  print: S.String,
  proxyArtUrl: S.optional(S.String),
});
export type DeckEntry = typeof DeckEntry.Type;

export const BuilderCommander = S.Struct({
  id: S.String,
  print: S.String,
  proxyArtUrl: S.optional(S.String),
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
  proxyArtUrl: S.optional(S.String),
  x: S.Number,
  y: S.Number,
});
export type BuilderHover = typeof BuilderHover.Type;

export const BuilderProxyArtPicker = S.Struct({
  cardId: S.String,
  error: S.NullOr(S.String),
  url: S.String,
});
export type BuilderProxyArtPicker = typeof BuilderProxyArtPicker.Type;

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
  confirmingDiscard: S.Boolean,
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
  printPicker: S.NullOr(BuilderPrintPicker),
  proxyArtPicker: S.NullOr(BuilderProxyArtPicker),
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
    confirmingDiscard: false,
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
    printPicker: null,
    proxyArtPicker: null,
    problems: [],
    query: "",
    saving: false,
    searching: true,
  };
}
