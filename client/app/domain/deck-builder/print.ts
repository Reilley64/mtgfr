import type { DeckCardEntry, DeckDetail } from "../wire/types";
import { imageUrlByPrint } from "./scryfall";

/** Scryfall `released_at` is YYYY-MM-DD; show the release year only. */
export function formatReleasedAt(iso: string | undefined): string {
  if (!iso) return "—";
  const year = iso.slice(0, 4);
  return /^\d{4}$/.test(year) ? year : "—";
}

/** Turn a loaded decklist into the builder's Card id -> row record. */
export function reconcileEntries(
  cards: ReadonlyArray<DeckCardEntry>,
): Record<string, { count: number; print: string }> {
  const out: Record<string, { count: number; print: string }> = {};
  for (const c of cards) out[c.id] = { count: c.count, print: c.print };
  return out;
}

/** Every art URL the board will paint for this deck, at the size the board asks for. */
export function deckArtUrls(deck: DeckDetail): string[] {
  return [deck.commander_print, ...deck.cards.map((card) => card.print)].map((print) => imageUrlByPrint(print));
}

/** When a deck row's print changes, commander art should stay in sync if it is the same Card id. */
export function commanderPrintForRow(commanderId: string, rowId: string, printId: string): string | null {
  if (!commanderId || commanderId !== rowId) return null;
  return printId;
}
