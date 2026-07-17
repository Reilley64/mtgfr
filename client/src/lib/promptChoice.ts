// Pure pending-choice helpers (no Solid / DOM) so unit tests never pull CardPreview.

import type { PendingChoiceView, VisibleState } from "~/wire/types";

/** The viewer's pending choice, if any. */
export function myChoice(state: VisibleState, me: number): PendingChoiceView | null {
  const pc = state.pending_choice;
  return pc && pc.player === me ? pc : null;
}

/**
 * Identity for a pending choice surface: kind + answering seat. Useful when remounting a form
 * only when the choice *type* changes (not on every same-kind delta).
 */
export function choiceShowKey(state: VisibleState, me: number): string | false {
  const c = myChoice(state, me);
  return c ? `${c.kind}:${c.player}` : false;
}

/**
 * Whether a choose_target prompt can use the card-image picker. Player seats (Bojuka Bog's
 * "exile target player's graveyard") have no art — those need the life-orb PickDialog instead.
 * `choose_target` uses fullscreen chrome (`isFullscreenPrompt`), so a player-only form that skips
 * PickDialog is invisible.
 */
export function chooseTargetIsCardPick(
  items: ReadonlyArray<{ id?: number; label?: string; player?: number | null }>,
): boolean {
  return items.length > 0 && items.every((it) => it.player == null);
}
