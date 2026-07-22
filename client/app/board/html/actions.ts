// Pure helpers for the sectioned action bar. Bucket the viewer's action list by section.

import type { ActionView } from "~/wire/types";

export type Section = "hand" | "command" | "graveyard" | "exile" | "battlefield" | "combat";
export type BarZone = "hand" | "command" | "graveyard" | "exile";
export type GroupedActions = Record<Section, ActionView[]>;

/** Zone aura on bar faces — Arena gap + colour, no section captions. */
export function barZoneAura(zone: BarZone, playable = false): string {
  if (zone === "hand") {
    return playable ? "ring-2 ring-playable-border shadow-[0_0_12px_rgba(234,255,240,0.42)]" : "";
  }
  if (zone === "command") return "ring-2 ring-commander-gold shadow-[0_0_12px_rgba(233,184,74,0.45)]";
  if (zone === "graveyard") {
    return playable ? "ring-2 ring-graveyard-outline shadow-[0_0_12px_rgba(123,92,255,0.45)]" : "";
  }
  return playable ? "ring-2 ring-exile-outline shadow-[0_0_12px_rgba(61,220,151,0.45)]" : "";
}

export function bySection(actions: readonly ActionView[] | undefined): GroupedActions {
  const g: GroupedActions = { hand: [], command: [], graveyard: [], exile: [], battlefield: [], combat: [] };
  for (const a of actions ?? []) {
    const bucket = g[a.section as Section];
    if (bucket) bucket.push(a);
  }
  return g;
}

export function byObject(actions: readonly ActionView[]): Map<number, ActionView> {
  const m = new Map<number, ActionView>();
  for (const a of actions) {
    if (a.object == null) continue;
    const existing = m.get(a.object);
    if (!existing || actionPriority(a) > actionPriority(existing)) m.set(a.object, a);
  }
  return m;
}

function actionPriority(a: ActionView): number {
  if (a.kind === "cast" || a.kind === "play_land") return 2;
  if (a.kind === "cycle") return 1;
  return 0;
}

export function handExtras(actions: readonly ActionView[]): ActionView[] {
  const primary = byObject(actions);
  return actions.filter((a) => a.object != null && primary.get(a.object)?.id !== a.id);
}

/** Object ids to paint with the auto-tap preview glyph while hovering an action. */
export function autoTapPreviewIds(action: ActionView | null | undefined): ReadonlySet<number> {
  return new Set(action?.auto_tap ?? []);
}
