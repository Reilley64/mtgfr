export function parseDeckIdParam(raw: string): number | null {
  if (raw.trim() === "") return null;
  const id = Number(raw);
  if (!Number.isInteger(id)) return null;
  return id;
}

export function deckCardViewTransitionName(deckId: number): string {
  return `deck-card-${deckId}`;
}
