import type { DeckSummary } from "../../../../lib/wire/types";

type KnownCommander = {
  readonly name: string;
};

const COLOR_PIP = ["W", "U", "B", "R", "G"] as const;

export function identityPipCodes(colorIdentity: readonly number[]): string[] {
  const out: string[] = [];
  for (const i of colorIdentity) {
    if (i < 0 || i > 4) continue;
    out.push(COLOR_PIP[i]);
  }
  return out;
}

export function deckListContextMenuAllowed(deckId: number): boolean {
  return deckId > 0;
}

export function visibleDecks(
  decks: readonly DeckSummary[],
  knownCommanders: Readonly<Record<string, KnownCommander | undefined>>,
  query: string,
): DeckSummary[] {
  const q = query.trim().toLowerCase();
  const matched =
    q === ""
      ? [...decks]
      : decks.filter((deck) => {
          if (deck.name.toLowerCase().includes(q)) return true;
          const commander = knownCommanders[deck.commander];
          const commanderLabel = (commander?.name ?? deck.commander).toLowerCase();
          return commanderLabel.includes(q);
        });

  const customs = matched.filter((d) => d.id > 0);
  const precons = matched.filter((d) => d.id < 0).sort((a, b) => a.id - b.id);
  return [...customs, ...precons];
}
