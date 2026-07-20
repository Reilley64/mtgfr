import type { ActionView, ObjectView, VisibleState } from "~/wire/types";

/** Name an object id from the viewer's visible state. */
export function objectName(state: VisibleState, id: number): string {
  return state.objects.find((o) => o.id === id)?.name ?? `#${id}`;
}

/** Printing UUID for an object id, when the viewer's visible state carries one (accounts-decks-and-catalog spec).
 * Empty when the object isn't found or has no print — callers render a broken image, not a
 * name-keyed fallback (there is no name-based art source anymore). */
export function objectPrint(state: VisibleState, id: number): string {
  return state.objects.find((o) => o.id === id)?.print ?? "";
}

/** Printing UUID for a pending-choice item. Prefers `item.print` (library/scry picks never
 * appear in `objects`); falls back to joining against the visible object list. */
export function choiceItemPrint(state: VisibleState, item: { id: number; print?: string }): string {
  return item.print || objectPrint(state, item.id);
}

/** A short subtitle naming the source permanent when it differs from the effect label. */
export function sourceHint(state: VisibleState, sourceId: number, effectLabel: string): string | undefined {
  const name = objectName(state, sourceId);
  return name !== effectLabel ? name : undefined;
}

/** Title for a pending single-target choice — the effect, with the source card named when useful. */
export function pendingTargetTitle(state: VisibleState, label: string, sourceId: number): string {
  const hint = sourceHint(state, sourceId, label);
  return hint ? `${label} — ${hint}` : label;
}

/** Title for a multi-target spell's target choice (CR 601.2c). */
export function spellTargetsTitle(label: string, min: number, max: number): string {
  const count =
    min === max
      ? min === 1
        ? "Choose 1 target"
        : `Choose ${min} targets`
      : max === 255 || max >= 99
        ? `Choose ${min} or more targets`
        : `Choose ${min}–${max} targets`;
  return `${label}: ${count}`;
}

/** Title for an optional paid trigger — name the source, the cost, and what paying does. */
export function payCostTitle(sourceName: string, cost: string, effectLabel: string): string {
  return `${sourceName}: pay ${cost} to ${effectLabel}?`;
}

/** Title for an optional "you may" trigger — name the source and what accepting does. */
export function mayYesNoTitle(sourceName: string, effectLabel: string): string {
  return `${sourceName}: ${effectLabel}?`;
}

/** Title for a pay-or-the-spell-is-countered prompt. */
export function payOrCounterTitle(spellName: string, cost: string): string {
  return `Pay ${cost} or ${spellName} is countered?`;
}

/** Title for an echo pay-or-sacrifice prompt. */
export function payEchoTitle(sourceName: string, cost: string): string {
  return `${sourceName}: pay echo ${cost} or sacrifice it?`;
}

/** Title while the player is aiming a staged cast or activation before submitting. */
export function stagedTargetTitle(staged: { card: ObjectView; action: ActionView } | null | undefined): string {
  if (!staged) return "Choose a target";
  const { card, action } = staged;
  if (action.kind === "activate" && action.label !== card.name) {
    return `${action.label} — ${card.name}`;
  }
  return action.label;
}

/** Corner hint while aiming with the targeting arrow on the canvas. */
export function stagedTargetHint(staged: { card: ObjectView; action: ActionView } | null | undefined): string {
  const title = stagedTargetTitle(staged);
  return `Targeting for ${title}`;
}
