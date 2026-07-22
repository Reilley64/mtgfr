import { Canvas } from "foldkit";
import type { PlayerView } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { type Camera, worldToScreen } from "../geometry/camera";
import { AVATAR_R, avatarPos, seatColor } from "../geometry/layout";

type Shape = Canvas.Shape;

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

    const stroke = priority === player.player ? "#ffd76a" : seatColor(player.player, 0.9);
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
