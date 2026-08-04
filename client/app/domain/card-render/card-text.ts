// The printed words a rendered face carries: type line and rules text, read off the catalog card.
// The wire's `kind` is what the engine needs, not what the card prints — see `typeLineOf`.

import type { CatalogCard, WireKind } from "~/wire/types";

/**
 * Type line, rules text and flavor for a face. Empty strings draw nothing.
 *
 * `flavorPrint` names the printing the flavor belongs to — the catalog only knows the card's
 * `default_print`, and flavor is per printing, so a card played from a deck that chose another
 * printing has to have its own flavor fetched (see `requestBarCardText`).
 */
export type CardText = { typeLine: string; oracle: string; flavor: string; flavorPrint: string };

const KIND_LABEL: Record<WireKind["kind"], string> = {
  artifact: "Artifact",
  battle: "Battle",
  creature: "Creature",
  enchantment: "Enchantment",
  instant: "Instant",
  land: "Land",
  planeswalker: "Planeswalker",
  sorcery: "Sorcery",
};

/**
 * The card's printed type line, as close as the wire allows.
 *
 * ponytail: `WireKind` names one card type, so a dual type line ("Artifact Creature") prints as its
 * primary type alone, and supertypes other than Legendary (Basic, Snow, World) are lost. Widening
 * the wire is the fix if a face ever has to read exactly.
 */
export function typeLineOf(card: CatalogCard): string {
  const types = card.legendary ? `Legendary ${KIND_LABEL[card.kind.kind]}` : KIND_LABEL[card.kind.kind];
  if (card.subtypes.length === 0) return types;
  return `${types} — ${card.subtypes.join(" ")}`;
}

export function cardTextOf(card: CatalogCard): CardText {
  return {
    typeLine: typeLineOf(card),
    oracle: card.oracle ?? "",
    flavor: card.flavor ?? "",
    flavorPrint: card.default_print,
  };
}
