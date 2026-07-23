import { Schema as S } from "effect";

export const PAGE = 100;
export const DECK_SIZE = 99;
export const BASICS = new Set(["Plains", "Island", "Swamp", "Mountain", "Forest"]);

const WireCost = S.Struct({
  colored: S.Array(S.Number),
  generic: S.Number,
  has_x: S.optional(S.Boolean),
  x_symbols: S.optional(S.Number),
});

const WireKind = S.Union([
  S.Struct({ kind: S.Literal("creature"), power: S.Number, toughness: S.Number }),
  S.Struct({ kind: S.Literal("instant") }),
  S.Struct({ kind: S.Literal("sorcery") }),
  S.Struct({ kind: S.Literal("enchantment") }),
  S.Struct({ kind: S.Literal("artifact") }),
  S.Struct({ kind: S.Literal("planeswalker"), loyalty: S.Number }),
  S.Struct({ kind: S.Literal("land"), colors: S.Array(S.Number) }),
]);

export const CatalogCardSchema = S.Struct({
  approximates: S.optional(S.NullOr(S.String)),
  back: S.optional(
    S.NullOr(
      S.Struct({
        approximates: S.optional(S.NullOr(S.String)),
        name: S.String,
        oracle: S.optional(S.NullOr(S.String)),
      }),
    ),
  ),
  color_identity: S.Array(S.Number),
  cost: WireCost,
  default_print: S.String,
  id: S.String,
  keywords: S.Array(S.String),
  kind: WireKind,
  legendary: S.Boolean,
  name: S.String,
  oracle: S.optional(S.NullOr(S.String)),
  otags: S.Array(S.String),
  set: S.String,
  subtypes: S.Array(S.String),
  summary: S.String,
});

export type BuilderCatalogCard = typeof CatalogCardSchema.Type;

export function canBeCommander(card: Pick<BuilderCatalogCard, "kind" | "legendary">): boolean {
  return card.legendary && card.kind.kind === "creature";
}

export function deckCount(entries: Record<string, { count: number }>): number {
  return Object.values(entries).reduce((sum, entry) => sum + entry.count, 0);
}

export function sortedDeckList(
  entries: Record<string, { count: number; print: string }>,
  known: Record<string, BuilderCatalogCard>,
): Array<{ id: string; count: number; print: string; name: string; legendary: boolean }> {
  return Object.entries(entries)
    .map(([id, entry]) => ({
      id,
      count: entry.count,
      print: entry.print,
      name: known[id]?.name ?? id,
      legendary: known[id]?.legendary ?? false,
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
}
