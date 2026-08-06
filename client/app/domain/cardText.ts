import type { CardTextView } from "~/wire/types";

/** Stable scalar encoding of the wire's `(oracle card id, printing id)` text identity. */
export function cardTextKey(cardId: string, print: string): string {
  return `${cardId}\u0000${print}`;
}

export function cardTextFor(
  book: ReadonlyMap<string, CardTextView>,
  cardId: string | null | undefined,
  print: string | null | undefined,
): CardTextView | undefined {
  if (cardId == null) return undefined;
  return book.get(cardTextKey(cardId, print ?? "")) ?? book.get(cardTextKey(cardId, ""));
}
