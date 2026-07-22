// Project per-seat mana pools to screen-anchored tray seats (camera + layout).

import { type ManaTrayChip, manaTrayChips } from "~/manaPips";
import type { PlayerView } from "~/wire/types";
import { type Camera, worldToScreen } from "./camera";
import { manaTrayPos } from "./layout";

export type ManaTraySeat = {
  seat: number;
  x: number;
  y: number;
  zoom: number;
  chips: ManaTrayChip[];
};

/** Non-empty pools → screen positions for the DOM mana tray overlay. */
export function projectManaTrays(
  players: readonly Pick<PlayerView, "player" | "mana_pool">[],
  viewer: number,
  playerCount: number,
  cam: Camera,
): ManaTraySeat[] {
  const out: ManaTraySeat[] = [];
  for (const p of players) {
    const chips = manaTrayChips(p.mana_pool);
    if (chips.length === 0) continue;
    const world = manaTrayPos(p.player, viewer, playerCount);
    const scr = worldToScreen(cam, world.x, world.y);
    out.push({ seat: p.player, x: scr.x, y: scr.y, zoom: cam.zoom, chips });
  }
  return out;
}
