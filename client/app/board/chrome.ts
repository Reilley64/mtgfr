import type { ActionView } from "~/wire/types";

export const CARD_RESTING_OUTLINE = "#1a1a1a";
export const PLAYABLE_BORDER = "#EAFFF0";
export const COMMANDER_GOLD = "#E9B84A";
export const GRAVEYARD_OUTLINE = "#7B5CFF";
export const EXILE_OUTLINE = "#3DDC97";

export function playableBattlefieldObjectIds(actions: readonly ActionView[] | undefined): ReadonlySet<number> {
  const ids = new Set<number>();
  for (const action of actions ?? []) {
    if (action.section !== "battlefield") continue;
    if (action.object == null) continue;
    ids.add(action.object);
  }
  return ids;
}
