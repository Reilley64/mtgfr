import type { ChoiceItem, PendingChoiceView } from "./wire/types";

/** Floor for the searchable card strip so flex shrink cannot collapse the grid. */
export const PICK_CARD_SCROLL_MIN_CLASS = "min-h-[min(40vh,280px)]";

/** Card-pick kinds that show a name filter (and pick-one face dedupe). */
export function cardPickIsSearchable(kind: PendingChoiceView["kind"]): boolean {
  return kind === "search_library";
}

/** Keep the first item per face label (library tutors show one Forest, not every copy). */
export function dedupeChoiceItems(items: ReadonlyArray<ChoiceItem>): ChoiceItem[] {
  const seen = new Set<string>();
  const out: ChoiceItem[] = [];
  for (const it of items) {
    if (seen.has(it.label)) continue;
    seen.add(it.label);
    out.push(it);
  }
  return out;
}

/** Case-insensitive substring filter on the card name (label). Empty query keeps all items. */
export function filterChoiceItems(items: ReadonlyArray<ChoiceItem>, query: string): ChoiceItem[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...items];
  return items.filter((it) => it.label.toLowerCase().includes(q));
}

/** Deduped then filtered candidates for a pick-one library search. */
export function searchableChoiceItems(items: ReadonlyArray<ChoiceItem>, query: string): ChoiceItem[] {
  return filterChoiceItems(dedupeChoiceItems(items), query);
}
