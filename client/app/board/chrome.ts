import type { ActionView } from "~/wire/types";

export const CARD_RESTING_OUTLINE = "#1a1a1a";
export const PLAYABLE_BORDER = "#EAFFF0";
export const COMMANDER_GOLD = "#E9B84A";
export const GRAVEYARD_OUTLINE = "#7B5CFF";
export const EXILE_OUTLINE = "#3DDC97";

export type PlayableCardGate = {
  id: number;
  summoningSick: boolean;
  hasHaste: boolean;
};

/** Battlefield object ids that should show Arena playable chrome right now. */
export function playableBattlefieldObjectIds(
  actions: readonly ActionView[] | undefined,
  cards: readonly PlayableCardGate[] = [],
): ReadonlySet<number> {
  const byId = new Map(cards.map((card) => [card.id, card]));
  const ids = new Set<number>();
  for (const action of actions ?? []) {
    if (action.section !== "battlefield") continue;
    if (action.object == null) continue;
    const card = byId.get(action.object);
    // CR 302.6: summoning-sick creatures can't pay {T}/{Q}. Don't advertise those activates.
    if (
      card != null &&
      card.summoningSick &&
      !card.hasHaste &&
      action.taps_self === true
    ) {
      continue;
    }
    ids.add(action.object);
  }
  return ids;
}
