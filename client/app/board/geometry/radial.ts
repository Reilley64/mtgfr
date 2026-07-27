// Legal activates for a selected permanent, including synthesized tap-for-mana.

import type { ActionView, WireCost } from "~/wire/types";
import { formatMessage } from "../../domain/i18n/message";
import { type Camera, worldToScreen } from "./camera";
import type { RenderCard } from "./layout";

export type RadialOption =
  | { kind: "tap_for_mana"; label: string; disabled: boolean }
  | { kind: "action"; action: ActionView; label: string; disabled: boolean };

export type ActivationCostChip =
  | { kind: "tap" }
  | { kind: "mana"; cost: WireCost }
  | { kind: "tap_and_mana"; cost: WireCost };

export function activationCostChip(opt: RadialOption): ActivationCostChip | null {
  if (opt.kind === "tap_for_mana") return { kind: "tap" };
  const taps = opt.action.taps_self === true;
  const mana = opt.action.x_cost ?? null;
  if (mana != null && taps) return { kind: "tap_and_mana", cost: mana };
  if (mana != null) return { kind: "mana", cost: mana };
  if (taps) return { kind: "tap" };
  return null;
}

export function radialScreenCenter(
  camera: Camera,
  card: Pick<RenderCard, "x" | "y" | "w" | "h">,
): { x: number; y: number } {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

export const ACTIVATION_MENU_WIDTH_PX = 240;
export const ACTIVATION_MENU_MAX_HEIGHT_PX = 280;
export const ACTIVATION_MENU_GAP_PX = 8;
const ACTIVATION_MENU_ROW_PX = 36;
const ACTIVATION_MENU_PAD_PX = 16;

export function activationMenuEstimatedHeight(optionCount: number, rowPx = ACTIVATION_MENU_ROW_PX): number {
  const n = Math.max(0, optionCount);
  return Math.min(ACTIVATION_MENU_MAX_HEIGHT_PX, n * rowPx + ACTIVATION_MENU_PAD_PX);
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

/**
 * Card-anchored menu box in % of the board viewport (CSS-stretch safe).
 * Prefer right → left → above → below; then clamp fully on-screen.
 */
export function activationMenuPlacement(
  center: { x: number; y: number },
  cardScreen: { w: number; h: number },
  menu: { width: number; height: number },
  viewport: { width: number; height: number },
  gap = ACTIVATION_MENU_GAP_PX,
): { left: string; top: string; width: string; maxHeight: string } {
  if (viewport.width <= 0 || viewport.height <= 0) {
    return { left: "0%", top: "0%", width: "0%", maxHeight: "0%" };
  }
  const halfW = cardScreen.w / 2;
  const halfH = cardScreen.h / 2;
  const candidates = [
    { x: center.x + halfW + gap, y: center.y - menu.height / 2 },
    { x: center.x - halfW - gap - menu.width, y: center.y - menu.height / 2 },
    { x: center.x - menu.width / 2, y: center.y - halfH - gap - menu.height },
    { x: center.x - menu.width / 2, y: center.y + halfH + gap },
  ];
  const fits = (p: { x: number; y: number }) =>
    p.x >= 0 && p.y >= 0 && p.x + menu.width <= viewport.width && p.y + menu.height <= viewport.height;
  const firstCandidate = candidates[0];
  if (firstCandidate == null) {
    return { left: "0%", top: "0%", width: "0%", maxHeight: "0%" };
  }
  const raw = candidates.find(fits) ?? firstCandidate;
  const x = clamp(raw.x, 0, Math.max(0, viewport.width - menu.width));
  const y = clamp(raw.y, 0, Math.max(0, viewport.height - menu.height));
  return {
    left: `${(x / viewport.width) * 100}%`,
    top: `${(y / viewport.height) * 100}%`,
    width: `${(menu.width / viewport.width) * 100}%`,
    maxHeight: `${(Math.min(menu.height, ACTIVATION_MENU_MAX_HEIGHT_PX) / viewport.height) * 100}%`,
  };
}

export function radialOptionKey(opt: RadialOption): string {
  if (opt.kind === "tap_for_mana") return "tap_for_mana";
  return `action:${opt.action.id}`;
}

export type RadialPress = { armed: number | null };

export function radialPressDown(_state: RadialPress, wedgeIndex: number): RadialPress {
  return { armed: wedgeIndex };
}

/** Resolve wedge index from a menu row or other `[data-wedge]` element. */
export function radialWedgeFromElement(el: EventTarget | null): number | null {
  if (!(el instanceof Element)) return null;
  const node = el.closest("[data-wedge]");
  if (!node) return null;
  const v = node.getAttribute("data-wedge");
  if (v == null) return null;
  const i = Number(v);
  return Number.isFinite(i) ? i : null;
}

/** Wedge under the pointer at release — not event target, which follows capture. */
export function radialWedgeAtPoint(
  clientX: number,
  clientY: number,
  elementFromPoint: (x: number, y: number) => Element | null,
): number | null {
  return radialWedgeFromElement(elementFromPoint(clientX, clientY));
}

export function radialPressUp(
  state: RadialPress,
  wedgeIndex: number | null,
): { state: RadialPress; commit: number | null; dismiss: boolean } {
  const clear = { armed: null as number | null };
  if (state.armed != null) {
    const commit = wedgeIndex === state.armed ? state.armed : null;
    return { state: clear, commit, dismiss: false };
  }
  if (wedgeIndex == null) return { state: clear, commit: null, dismiss: true };
  return { state: clear, commit: wedgeIndex, dismiss: false };
}

/** Options for the activation radial around a selected permanent. */
export function radialOptions(
  objectId: number,
  actions: ActionView[] | undefined,
  tapsForMana: boolean,
  tapped: boolean,
  canAct: boolean,
  summoningSick = false,
  hasHaste = false,
): RadialOption[] {
  const out: RadialOption[] = [];
  if (tapsForMana) {
    const sickBlocksTap = summoningSick && !hasHaste;
    out.push({
      kind: "tap_for_mana",
      label: "Tap for mana",
      disabled: !canAct || tapped || sickBlocksTap,
    });
  }
  for (const a of actions ?? []) {
    if (a.section !== "battlefield" || a.object !== objectId) continue;
    out.push({ kind: "action", action: a, label: formatMessage(a.label), disabled: false });
  }
  return out;
}
