// CR 103.1 spotlight reveal: pure hop schedule, one-shot-per-table storage, and the
// two sleep commands that step it. No rAF — the board's frame clock stays untouched.
import { Duration, Effect, Schema as S } from "effect";
import { Command } from "foldkit";
import { FirstPlayerRevealFinished, FirstPlayerRevealStepped } from "./messages";

export type SpotlightStep = { slot: number; delayMs: number };

const LAPS = 3;
const FIRST_GAP_MS = 70;
const LAST_GAP_MS = 200;
export const REVEAL_HOLD_MS = 600;
export const REVEAL_HOLD_REDUCED_MS = 1200;

/** Screen slot for a seat, matching the board's viewer-relative 2×2 quadrants. */
export function revealSlot(seat: number, viewer: number, count: number): number {
  const seatCount = Math.max(1, count);
  const anchor = viewer < seatCount ? viewer : 0;
  return (seat - anchor + seatCount) % seatCount;
}

/** Hop schedule: laps around the table decelerating onto the winner's slot. */
export function spotlightSteps(winnerSlot: number, seatCount: number, reducedMotion: boolean): SpotlightStep[] {
  if (reducedMotion) return [{ slot: winnerSlot, delayMs: 0 }];

  const count = Math.max(1, seatCount);
  const total = LAPS * count + (winnerSlot % count) + 1;
  return Array.from({ length: total }, (_, i) => ({
    slot: i % count,
    delayMs: i === 0 ? 0 : Math.round(FIRST_GAP_MS + ((LAST_GAP_MS - FIRST_GAP_MS) * i) / (total - 1)),
  }));
}

function storageKey(tableId: string): string {
  return `mtgfr:first-player-reveal:${tableId}`;
}

export function revealSeen(tableId: string): boolean {
  if (typeof sessionStorage === "undefined") return false;
  try {
    return sessionStorage.getItem(storageKey(tableId)) != null;
  } catch {
    return false;
  }
}

export function markRevealSeen(tableId: string): void {
  if (typeof sessionStorage === "undefined") return;
  try {
    sessionStorage.setItem(storageKey(tableId), "1");
  } catch {
    // ponytail: privacy-mode storage denial just means the reveal may replay on reload.
  }
}

export function prefersReducedMotion(): boolean {
  if (typeof matchMedia === "undefined") return false;
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export const RevealStepTimer = Command.define(
  "RevealStepTimer",
  { ms: S.Number },
  FirstPlayerRevealStepped,
)(({ ms }) => Effect.sleep(Duration.millis(ms)).pipe(Effect.as(FirstPlayerRevealStepped())));

export const RevealHoldTimer = Command.define(
  "RevealHoldTimer",
  { ms: S.Number },
  FirstPlayerRevealFinished,
)(({ ms }) => Effect.sleep(Duration.millis(ms)).pipe(Effect.as(FirstPlayerRevealFinished())));
