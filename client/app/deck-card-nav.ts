import { Effect, Schema as S } from "effect";
import { m } from "foldkit/message";
import * as Mount from "foldkit/mount";
import { parseDeckIdParam } from "./deck-id";

export type DeckCardFlipSnapshot = {
  deckId: number;
  top: number;
  left: number;
  width: number;
  height: number;
};

let pendingFlip: DeckCardFlipSnapshot | null = null;

/** No-op message so BindDeckCardFlip can settle like other Mounts. */
export const DeckCardFlipTick = m("DeckCardFlipTick");

function pathnameOnly(path: string): string {
  try {
    return new URL(path, "http://localhost").pathname;
  } catch {
    return path.split(/[?#]/, 1)[0] ?? "";
  }
}

function isHome(path: string): boolean {
  return pathnameOnly(path) === "/";
}

function isPlayDeckEntry(path: string): boolean {
  return /^\/play\/[^/]+$/.test(pathnameOnly(path));
}

function playDeckIdFromPath(path: string): number | null {
  const pathname = pathnameOnly(path);
  const match = pathname.match(/^\/play\/([^/]+)$/);
  if (match == null) return null;
  return parseDeckIdParam(match[1] ?? "");
}

export function shouldAnimateDeckCardNav(fromPathname: string, toPathname: string): boolean {
  if (isHome(fromPathname) && isPlayDeckEntry(toPathname)) return true;
  return isPlayDeckEntry(fromPathname) && isHome(toPathname);
}

function prefersReducedMotion(): boolean {
  return globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

function snapshotElement(deckId: number, el: Element): DeckCardFlipSnapshot | null {
  if (!(el instanceof HTMLElement)) return null;
  const rect = el.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return null;
  return {
    deckId,
    top: rect.top,
    left: rect.left,
    width: rect.width,
    height: rect.height,
  };
}

/** Capture the outgoing deck card before SPA navigation (FLIP first-half). */
export function captureDeckCardFlipForNav(fromPathname: string, toUrl: string): void {
  pendingFlip = null;
  if (prefersReducedMotion()) return;
  if (!shouldAnimateDeckCardNav(fromPathname, pathnameOnly(toUrl))) return;

  const from = pathnameOnly(fromPathname);
  const to = pathnameOnly(toUrl);
  const deckId = playDeckIdFromPath(isPlayDeckEntry(from) ? from : to);
  if (deckId == null) return;

  // Capture the same chrome node that BindDeckCardFlip animates (inner body),
  // not the outer a/div root — Foldkit allows only one OnMount per element.
  const el = globalThis.document?.querySelector(`[data-deck-card-flip="${deckId}"]`);
  if (el == null) return;
  pendingFlip = snapshotElement(deckId, el);
}

export function takePendingDeckCardFlip(deckId: number): DeckCardFlipSnapshot | null {
  if (pendingFlip == null || pendingFlip.deckId !== deckId) return null;
  const snap = pendingFlip;
  pendingFlip = null;
  return snap;
}

/** Test/reset helper — not used in product paths. */
export function resetDeckCardFlipForTests(): void {
  pendingFlip = null;
}

export function runDeckCardFlip(
  el: HTMLElement,
  from: DeckCardFlipSnapshot,
  animate: typeof Element.prototype.animate = HTMLElement.prototype.animate,
): void {
  const to = el.getBoundingClientRect();
  if (to.width <= 0 || to.height <= 0) return;

  const dx = from.left - to.left;
  const dy = from.top - to.top;
  const sx = from.width / to.width;
  const sy = from.height / to.height;
  if (Math.abs(dx) < 1 && Math.abs(dy) < 1 && Math.abs(sx - 1) < 0.02 && Math.abs(sy - 1) < 0.02) {
    return;
  }

  animate.call(
    el,
    [
      { transform: `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})`, transformOrigin: "top left" },
      { transform: "translate(0px, 0px) scale(1, 1)", transformOrigin: "top left" },
    ],
    { duration: 280, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)", fill: "both" },
  );
}

export const BindDeckCardFlip = Mount.define(
  "BindDeckCardFlip",
  { deckId: S.Number },
  DeckCardFlipTick,
)(
  ({ deckId }) =>
    (element) =>
      Effect.sync(() => {
        if (!(element instanceof HTMLElement)) return DeckCardFlipTick();
        const from = takePendingDeckCardFlip(deckId);
        if (from != null) runDeckCardFlip(element, from);
        return DeckCardFlipTick();
      }),
);
