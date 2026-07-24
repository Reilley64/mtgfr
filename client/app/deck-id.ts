export function parseDeckIdParam(raw: string): number | null {
  if (raw.trim() === "") return null;
  const id = Number(raw);
  if (!Number.isInteger(id)) return null;
  return id;
}

export function deckCardViewTransitionName(deckId: number): string {
  return `deck-card-${deckId}`;
}

export function playDeckAccess(
  deckId: number | null,
  decks: ReadonlyArray<{ id: number }>,
  loading: boolean,
  error: string | null = null,
): "loading" | "ok" | "missing" | "error" {
  if (deckId == null) return "missing";
  if (error != null) return "error";
  if (loading && decks.length === 0) return "loading";
  if (decks.some((deck) => deck.id === deckId)) return "ok";
  if (loading) return "loading";
  return "missing";
}
