// Pure helpers for the sectioned action bar. The engine ships the viewer their full legal-action
// list (`VisibleState.actions`); these functions bucket that flat list into the bar's sections.
// Kept pure (no Solid, no DOM) so both are unit-testable headlessly, in the style of layout.ts /
// store.ts.

import type { ActionView } from "~/wire/types";

/** The action-bar sections, matching `ActionView.section` (engine truth). "combat" actions inform
 * the existing combat UI rather than rendering as cards. */
export type Section = "hand" | "command" | "graveyard" | "exile" | "battlefield" | "combat";

/** Zones that can own tiles in the bottom action bar (not battlefield/combat). */
export type BarZone = "hand" | "command" | "graveyard" | "exile";

/**
 * Arena-style zone aura on bar tiles: hand is unmarked; command / graveyard / exile each get a
 * distinct ring so the gap between groups (not a caption) carries the zone cue.
 */
export function barZoneAura(zone: BarZone): string | null {
  if (zone === "hand") return null;
  if (zone === "command") {
    return "ring-2 ring-commander-gold shadow-[0_0_12px_rgba(233,184,74,0.45)]";
  }
  if (zone === "graveyard") {
    return "ring-2 ring-note-gold shadow-[0_0_12px_rgba(240,198,116,0.4)]";
  }
  return "ring-2 ring-island-blue shadow-[0_0_12px_rgba(119,204,255,0.45)]";
}

export type GroupedActions = Record<Section, ActionView[]>;

/** Bucket the viewer's actions by their `section`. Unknown sections are dropped (forward-compat:
 * a new engine section the client doesn't render yet simply doesn't appear). */
export function bySection(actions: ActionView[] | undefined): GroupedActions {
  const g: GroupedActions = { hand: [], command: [], graveyard: [], exile: [], battlefield: [], combat: [] };
  for (const a of actions ?? []) {
    const bucket = g[a.section as Section];
    if (bucket) bucket.push(a);
  }
  return g;
}

/** Index actions by their `object` id — how the Hand section matches each hand card to its play
 * action (a card with no entry renders dimmed and undraggable). Prefer cast/play_land over cycle
 * when both exist for the same object; a separate Hand tile surfaces the cycle action. */
export function byObject(actions: ActionView[]): Map<number, ActionView> {
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

/** Secondary hand actions (cycle / suspend / discard-ability) that share an object with a
 * higher-priority play action — rendered as extra Hand tiles so the alternative stays reachable
 * when a cast or land drop is also legal. Any overshadowed hand action qualifies: `byObject` keeps
 * only the top-priority action per object, so every other one on that object needs its own tile. */
export function handExtras(actions: ActionView[]): ActionView[] {
  const primary = byObject(actions);
  return actions.filter((a) => a.object != null && primary.get(a.object)?.id !== a.id);
}

/** Object ids to paint with the auto-tap preview glyph while hovering an action.
 * Pass the *live* `ActionView` from the current snapshot (look up by id), not a stale copy. */
export function autoTapPreviewIds(action: ActionView | null | undefined): ReadonlySet<number> {
  return new Set(action?.auto_tap ?? []);
}
