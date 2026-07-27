import { Canvas } from "foldkit";
import { colors } from "~/design-tokens.generated";
import type { PlayerView } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { type Camera, worldToScreen } from "../geometry/camera";
import { AVATAR_R, avatarPos, seatColor } from "../geometry/layout";

type Shape = Canvas.Shape;

/** Highest combat damage from any single commander source (21-damage clock). */
export function maxCommanderDamage(player: PlayerView): number {
  const rows = player.commander_damage;
  if (rows == null || rows.length === 0) return 0;
  let max = 0;
  for (const row of rows) {
    if (row.amount > max) max = row.amount;
  }
  return max;
}

/** The alternate lose-the-game / attrition clocks stacked under a life orb, in row order. */
export function clockChips(player: PlayerView): Array<{ label: string; fill: string }> {
  const chips: Array<{ label: string; fill: string }> = [];
  const cmd = maxCommanderDamage(player);
  if (cmd > 0) chips.push({ label: `Cmd ${cmd}`, fill: "#db8664" });
  const poison = player.poison ?? 0;
  // CR 704.5c — ten poison counters eliminate a player, so the last two tick over to red.
  if (poison > 0) chips.push({ label: `Poison ${poison}`, fill: poison >= 8 ? "#e0574f" : "#8fd14f" });
  const rad = player.rad ?? 0;
  if (rad > 0) chips.push({ label: `Rad ${rad}`, fill: "#e8a33d" });
  return chips;
}

export type AvatarScreenPositions = Record<number, { x: number; y: number }>;

export function avatarScreenPositions(
  players: ReadonlyArray<PlayerView>,
  viewer: number,
  count: number,
  camera: Camera,
): AvatarScreenPositions {
  const out: AvatarScreenPositions = {};
  for (const player of players) {
    const pos = avatarPos(player.player, viewer, count);
    out[player.player] = worldToScreen(camera, pos.x, pos.y);
  }
  return out;
}

export function avatarShapes(
  players: ReadonlyArray<PlayerView>,
  positions: AvatarScreenPositions,
  priority: number,
  zoom: number,
  targetPlayers: ReadonlySet<number> = new Set(),
): Shape[] {
  const radius = AVATAR_R * zoom;
  const shapes: Shape[] = [];

  for (const player of players) {
    const pos = positions[player.player];
    if (pos == null) continue;

    const stroke = priority === player.player ? colors.priorityGold : seatColor(player.player, 0.9);
    const targeted = targetPlayers.has(player.player);
    shapes.push(
      Canvas.Circle({
        x: pos.x,
        y: pos.y,
        radius,
        fill: player.lost ? "rgba(14,26,20,0.5)" : "rgba(14,26,20,0.95)",
        stroke,
        lineWidth: priority === player.player ? 4 : 2,
      }),
      Canvas.Text({
        x: pos.x,
        y: pos.y + 4 * zoom,
        content: `${player.life}`,
        font: `700 ${Math.max(1, Math.round(30 * zoom))}px system-ui, sans-serif`,
        fill: "#eff",
        align: "Center",
        baseline: "Middle",
      }),
      Canvas.Text({
        x: pos.x,
        y: pos.y + 27 * zoom,
        content: player.username?.trim() || `P${player.player}`,
        font: `${Math.max(1, Math.round(14 * zoom))}px system-ui, sans-serif`,
        fill: "#9cb",
        align: "Center",
        baseline: "Middle",
      }),
      Canvas.Text({
        x: pos.x,
        y: pos.y - 29 * zoom,
        content: `Hand ${player.hand_count}`,
        font: `${Math.max(1, Math.round(12 * zoom))}px system-ui, sans-serif`,
        fill: "#89a",
        align: "Center",
        baseline: "Middle",
      }),
    );

    for (const [row, chip] of clockChips(player).entries()) {
      shapes.push(
        Canvas.Text({
          x: pos.x,
          y: pos.y + (42 + row * 14) * zoom,
          content: chip.label,
          font: `${Math.max(1, Math.round(12 * zoom))}px system-ui, sans-serif`,
          fill: chip.fill,
          align: "Center",
          baseline: "Middle",
        }),
      );
    }

    if (targeted) {
      shapes.push(
        Canvas.Circle({
          x: pos.x,
          y: pos.y,
          radius: radius + 5,
          stroke: TARGET_COLOR,
          lineWidth: 3,
        }),
      );
    }
  }

  return shapes;
}
