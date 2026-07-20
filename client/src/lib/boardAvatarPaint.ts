// Player avatar discs and target rings on the battlefield canvas.

import { AVATAR_R, seatColor } from "~/layout";
import type { Stroke, Vec } from "~/lib/boardPaintPrims";
import { LETHAL_COMMANDER_DAMAGE, worstCommanderDamage } from "~/lib/outcome";
import type { VisibleState } from "~/wire/types";

export function drawAvatar(
  ctx: CanvasRenderingContext2D,
  pos: Vec,
  radius: number,
  player: VisibleState["players"][number],
  priority: boolean,
) {
  const scale = radius / AVATAR_R;
  ctx.save();
  if (player.lost) {
    ctx.globalAlpha = 0.5;
  }
  ctx.beginPath();
  ctx.arc(pos.x, pos.y, radius, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(14,26,20,0.95)";
  ctx.fill();
  ctx.lineWidth = (priority ? 4 : 2) * scale;
  ctx.strokeStyle = priority ? "#ffd76a" : seatColor(player.player, 0.9);
  if (priority) {
    ctx.shadowColor = "rgba(255,215,106,0.6)";
    ctx.shadowBlur = 22 * scale;
  }
  ctx.stroke();
  ctx.shadowBlur = 0;
  ctx.fillStyle = "#eff";
  ctx.textAlign = "center";
  // Floor at 1px so pure zoom never paints a 0px font (circle would outlive the labels).
  ctx.font = `700 ${Math.max(1, Math.round(30 * scale))}px system-ui, sans-serif`;
  ctx.fillText(`${player.life}`, pos.x, pos.y + 4 * scale);
  ctx.font = `${Math.max(1, Math.round(14 * scale))}px system-ui, sans-serif`;
  ctx.fillStyle = "#9cb";
  const label = player.username?.trim() || `P${player.player}`;
  ctx.fillText(`${label}${player.lost ? " ✕" : ""}`, pos.x, pos.y + 26 * scale);
  ctx.fillStyle = "#89a";
  ctx.fillText(`🖐${player.hand_count}`, pos.x, pos.y - 30 * scale);
  const worst = worstCommanderDamage(player.commander_damage);
  if (worst > 0) {
    const share = worst / LETHAL_COMMANDER_DAMAGE;
    ctx.fillStyle = share >= 1 ? "#ff6b6b" : share >= 0.66 ? "#e8b24a" : "#89a";
    ctx.fillText(`⚔ ${worst}/${LETHAL_COMMANDER_DAMAGE}`, pos.x, pos.y + 40 * scale);
  }
  ctx.textAlign = "left";
  ctx.restore();
}

export function ringAvatar(ctx: CanvasRenderingContext2D, pos: Vec, radius: number, stroke: Stroke) {
  ctx.save();
  ctx.beginPath();
  ctx.arc(pos.x, pos.y, radius + 5, 0, Math.PI * 2);
  ctx.strokeStyle = stroke.color;
  ctx.lineWidth = 3;
  ctx.setLineDash(stroke.dash);
  ctx.stroke();
  ctx.restore();
}
