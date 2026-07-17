import type { DeckCardEntry } from "~/wire/types";

/** Scryfall `released_at` is YYYY-MM-DD; show the release year only. */
export function formatReleasedAt(iso: string | undefined): string {
  if (!iso) return "—";
  const year = iso.slice(0, 4);
  return /^\d{4}$/.test(year) ? year : "—";
}

/** Turn a loaded decklist into the store's id → { count, print } record. */
export function reconcileEntries(cards: DeckCardEntry[]): Record<string, { count: number; print: string }> {
  const out: Record<string, { count: number; print: string }> = {};
  for (const c of cards) out[c.id] = { count: c.count, print: c.print };
  return out;
}

/** When a deck row's print changes, commander art should stay in sync if it is the same Card id. */
export function commanderPrintForRow(commanderId: string, rowId: string, printId: string): string | null {
  if (!commanderId || commanderId !== rowId) return null;
  return printId;
}
