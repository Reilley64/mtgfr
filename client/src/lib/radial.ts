// Legal activates for a selected permanent, including synthesized tap-for-mana.

import type { ActionView } from "~/api/generated";
import { CARD_H } from "~/layout";

export type RadialOption =
  | { kind: "tap_for_mana"; label: string }
  | { kind: "action"; action: ActionView; label: string };

/**
 * Screen-px radius from card center to option centers. Scales with camera zoom so the ring
 * tracks the on-screen card instead of drifting away when zoomed out or sitting on the art
 * when zoomed in. `+12` is a small gap past the card's half-height.
 */
export function activationRadialRadius(zoom: number): number {
  return Math.max(40, (CARD_H / 2) * zoom + 12);
}

/** Options for the activation radial around a selected permanent. */
export function radialOptions(
  objectId: number,
  actions: ActionView[] | undefined,
  tapsForMana: boolean,
  tapped: boolean,
  canAct: boolean,
): RadialOption[] {
  const out: RadialOption[] = [];
  if (canAct && tapsForMana && !tapped) out.push({ kind: "tap_for_mana", label: "Tap for mana" });
  for (const a of actions ?? []) {
    if (a.section !== "battlefield" || a.object !== objectId) continue;
    out.push({ kind: "action", action: a, label: a.label });
  }
  return out;
}
