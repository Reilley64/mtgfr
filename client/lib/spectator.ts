// Wire spectator seat — matches schema::SPECTATOR_VIEWER and Solid `store.ts`.
import { outcome } from "./outcome";
import type { PlayerView } from "./wire/types";

/** `VisibleState.viewer` for a spectator — watcher with no seat (server: u8::MAX). */
export const SPECTATOR_VIEWER = 255;

/** Seated and still in the game (not eliminated, not spectating). */
export function isActivePlayer(players: ReadonlyArray<PlayerView>, viewer: number): boolean {
  if (viewer === SPECTATOR_VIEWER) return false;
  return outcome(players, viewer).kind === "playing";
}
