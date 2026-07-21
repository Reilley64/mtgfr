// Legal activates for a selected permanent, including synthesized tap-for-mana.

import { CARD_H, CARD_W } from "~/layout";
import type { ActionView } from "~/wire/types";

export type RadialOption =
  | { kind: "tap_for_mana"; label: string }
  | { kind: "action"; action: ActionView; label: string };

const INNER_GAP_PX = 4;
const MIN_RING_PX = 36;

/**
 * Screen-px radius from card center to option centers. Scales with camera zoom so the ring
 * tracks the on-screen card instead of drifting away when zoomed out or sitting on the art
 * when zoomed in. `+12` is a small gap past the card's half-height.
 */
export function activationRadialRadius(zoom: number): number {
  return Math.max(40, (CARD_H / 2) * zoom + 12);
}

export function activationRadialInnerRadius(zoom: number): number {
  return Math.hypot(CARD_W / 2, CARD_H / 2) * zoom + INNER_GAP_PX;
}

export function activationRadialOuterRadius(zoom: number): number {
  const inner = activationRadialInnerRadius(zoom);
  return Math.max(activationRadialRadius(zoom), inner + MIN_RING_PX);
}

export function radialOptionKey(opt: RadialOption): string {
  if (opt.kind === "tap_for_mana") return "tap_for_mana";
  return `action:${opt.action.id}`;
}

/** Normalize atan2 angle so 0 is the start of wedge 0 (top-centered). */
export function wedgeIndex(angleRad: number, count: number): number {
  if (count <= 1) return 0;
  const slice = (2 * Math.PI) / count;
  // Shift so wedge 0 is centered on -π/2 (top).
  let a = angleRad + Math.PI / 2 + slice / 2;
  a = ((a % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
  return Math.min(count - 1, Math.floor(a / slice));
}

export function wedgePath(i: number, count: number, inner: number, outer: number): string {
  const slice = (2 * Math.PI) / count;
  const a0 = -Math.PI / 2 - slice / 2 + i * slice;
  const a1 = a0 + slice;
  const large = slice > Math.PI ? 1 : 0;
  const x = (r: number, a: number) => Math.cos(a) * r;
  const y = (r: number, a: number) => Math.sin(a) * r;
  return [
    `M ${x(outer, a0)} ${y(outer, a0)}`,
    `A ${outer} ${outer} 0 ${large} 1 ${x(outer, a1)} ${y(outer, a1)}`,
    `L ${x(inner, a1)} ${y(inner, a1)}`,
    `A ${inner} ${inner} 0 ${large} 0 ${x(inner, a0)} ${y(inner, a0)}`,
    "Z",
  ].join(" ");
}

export function wedgeLabelPoint(
  i: number,
  count: number,
  inner: number,
  outer: number,
): { x: number; y: number } {
  const slice = (2 * Math.PI) / count;
  const mid = -Math.PI / 2 + i * slice;
  const r = (inner + outer) / 2;
  return { x: Math.cos(mid) * r, y: Math.sin(mid) * r };
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
