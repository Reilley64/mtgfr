// Context-menu item lists for the deck builder (Solid parity).
// Labels are user-facing; `action` is the TEA payload `RanBuilderMenuAction` runs.

import { BASICS, type BuilderCatalogCard, canBeCommander, DECK_SIZE } from "./cards";

export type BuilderMenuAction =
  | { kind: "add"; cardId: string; count: number }
  | { kind: "remove"; cardId: string; count: number }
  | { kind: "fill"; cardId: string; count: number }
  | { kind: "setCommander"; cardId: string }
  | { kind: "choosePrint"; cardId: string; addOnPick: boolean };

export type BuilderMenuItem = { label: string; action: BuilderMenuAction };

export function poolMenuItems(args: {
  card: Pick<BuilderCatalogCard, "id" | "kind" | "legendary" | "name">;
  inDeck: boolean;
  total: number;
}): BuilderMenuItem[] {
  const { card, inDeck, total } = args;
  const items: BuilderMenuItem[] = BASICS.has(card.name)
    ? [
        { label: "Add One", action: { kind: "add", cardId: card.id, count: 1 } },
        { label: "Add Two", action: { kind: "add", cardId: card.id, count: 2 } },
        { label: "Add Five", action: { kind: "add", cardId: card.id, count: 5 } },
        {
          label: "Fill deck",
          action: { kind: "fill", cardId: card.id, count: Math.max(0, DECK_SIZE - total) },
        },
      ]
    : canBeCommander(card)
      ? [
          { label: "Add One", action: { kind: "add", cardId: card.id, count: 1 } },
          { label: "Set As Commander", action: { kind: "setCommander", cardId: card.id } },
        ]
      : [{ label: "Add One", action: { kind: "add", cardId: card.id, count: 1 } }];

  if (!inDeck) {
    items.push({
      label: "Choose print",
      action: { kind: "choosePrint", cardId: card.id, addOnPick: true },
    });
  }
  return items;
}

export function rowMenuItems(args: {
  card: Pick<BuilderCatalogCard, "id" | "name"> | undefined;
  total: number;
}): BuilderMenuItem[] {
  const { card, total } = args;
  const items: BuilderMenuItem[] = [];
  if (card && BASICS.has(card.name)) {
    items.push(
      {
        label: "Fill deck",
        action: { kind: "fill", cardId: card.id, count: Math.max(0, DECK_SIZE - total) },
      },
      { label: "Remove 1", action: { kind: "remove", cardId: card.id, count: 1 } },
      { label: "Remove 2", action: { kind: "remove", cardId: card.id, count: 2 } },
      { label: "Remove 5", action: { kind: "remove", cardId: card.id, count: 5 } },
    );
  }
  if (card) {
    items.push({
      label: "Choose print",
      action: { kind: "choosePrint", cardId: card.id, addOnPick: false },
    });
  }
  return items;
}

export function commanderMenuItems(args: { cardId: string }): BuilderMenuItem[] {
  return [
    {
      label: "Choose print",
      action: { kind: "choosePrint", cardId: args.cardId, addOnPick: false },
    },
  ];
}
