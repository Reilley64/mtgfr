// Pure helpers for the sectioned action bar. Bucket the viewer's action list by section.

import type { ActionView } from "~/wire/types";

export type Section = "hand" | "command" | "graveyard" | "exile" | "battlefield" | "combat";
export type BarZone = "hand" | "command" | "graveyard" | "exile";
export type GroupedActions = Record<Section, ActionView[]>;

/**
 * Zone aura on bar faces — Arena gap + colour, no section captions.
 *
 * Playable dual chrome uses `ring` (mint) + `outline` (zone colour). Do not put the zone
 * colour in `box-shadow` at the same radius as `ring-2`: Tailwind paints ring and shadow into
 * one `box-shadow` list, so a 2px zone shadow is fully covered by the mint ring.
 */
export function barZoneAura(zone: BarZone, playable = false): string {
  if (zone === "hand") {
    return playable ? "ring-2 ring-playable-border shadow-[0_0_12px_rgba(234,255,240,0.42)]" : "";
  }
  if (zone === "command") {
    if (playable) {
      return "ring-2 ring-playable-border outline-2 outline-commander-gold outline-offset-2 shadow-[0_0_12px_rgba(233,184,74,0.45),0_0_12px_rgba(234,255,240,0.35)]";
    }
    return "ring-2 ring-commander-gold shadow-[0_0_12px_rgba(233,184,74,0.45)]";
  }
  if (zone === "graveyard") {
    if (playable) {
      return "ring-2 ring-playable-border outline-2 outline-graveyard-outline outline-offset-2 shadow-[0_0_12px_rgba(123,92,255,0.45),0_0_12px_rgba(234,255,240,0.35)]";
    }
    return "ring-2 ring-graveyard-outline shadow-[0_0_12px_rgba(123,92,255,0.45)]";
  }
  if (playable) {
    return "ring-2 ring-playable-border outline-2 outline-exile-outline outline-offset-2 shadow-[0_0_12px_rgba(61,220,151,0.45),0_0_12px_rgba(234,255,240,0.35)]";
  }
  return "ring-2 ring-exile-outline shadow-[0_0_12px_rgba(61,220,151,0.45)]";
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

type PaymentPreviewBoard = {
  hoverActionId: number | null;
  staged: { action: ActionView } | null;
  xPrompt: { action: ActionView } | null;
  modalCast: { action: ActionView } | null;
  sacrificePick: { action: ActionView } | null;
  discardPick: { action: ActionView } | null;
  gyExilePick: { action: ActionView } | null;
};

/**
 * Action whose `auto_tap` should paint while aiming or paying.
 * Session actions win over hover so the preview survives HandActionActivated clearing hover.
 */
export function paymentPreviewAction(
  board: PaymentPreviewBoard,
  actions: ReadonlyArray<ActionView> | null | undefined,
): ActionView | null {
  if (board.staged != null) return board.staged.action;
  if (board.xPrompt != null) return board.xPrompt.action;
  if (board.modalCast != null) return board.modalCast.action;
  if (board.sacrificePick != null) return board.sacrificePick.action;
  if (board.discardPick != null) return board.discardPick.action;
  if (board.gyExilePick != null) return board.gyExilePick.action;
  return actions?.find((action) => action.id === board.hoverActionId) ?? null;
}
