import { Canvas } from "foldkit";
import type { WireAttack, WireBlock } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { type Camera, worldToScreen } from "../geometry/camera";
import type { RenderCard } from "../geometry/layout";
import type { AvatarScreenPositions } from "./avatars";

type Shape = Canvas.Shape;

const ATTACK_STROKE = "#ff6b6b";
const BLOCK_STROKE = "#66ff99";

type Vec = { x: number; y: number };

function cardCenter(camera: Camera, card: RenderCard): Vec {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

function arrowPath(from: Vec, to: Vec, stroke: string): Shape[] {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const len = Math.hypot(dx, dy) || 1;
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
  const control = {
    x: mid.x - (dy / len) * Math.min(48, len * 0.22),
    y: mid.y + (dx / len) * Math.min(48, len * 0.22),
  };
  const angle = Math.atan2(to.y - control.y, to.x - control.x);

  return [
    Canvas.Path({
      instructions: [
        Canvas.MoveTo({ x: from.x, y: from.y }),
        Canvas.QuadTo({ cpx: control.x, cpy: control.y, x: to.x, y: to.y }),
      ],
      stroke,
      lineWidth: 3,
      lineCap: "Round",
      lineJoin: "Round",
    }),
    Canvas.Path({
      instructions: [
        Canvas.MoveTo({ x: to.x, y: to.y }),
        Canvas.LineTo({ x: to.x - 13 * Math.cos(angle - 0.4), y: to.y - 13 * Math.sin(angle - 0.4) }),
        Canvas.LineTo({ x: to.x - 13 * Math.cos(angle + 0.4), y: to.y - 13 * Math.sin(angle + 0.4) }),
        Canvas.Close(),
      ],
      fill: stroke,
    }),
  ];
}

export function combatDragArrowShapes(input: { from: Vec; to: Vec; declaringBlock: boolean }): Shape[] {
  const stroke = input.declaringBlock ? BLOCK_STROKE : ATTACK_STROKE;
  return arrowPath(input.from, input.to, stroke);
}

export function aimArrowShapes(input: { from: Vec; to: Vec }): Shape[] {
  return arrowPath(input.from, input.to, TARGET_COLOR);
}

export function arrowShapes(input: {
  camera: Camera;
  cards: ReadonlyArray<RenderCard>;
  avatars: AvatarScreenPositions;
  attackers: ReadonlyArray<WireAttack>;
  blocks: ReadonlyArray<WireBlock>;
}): Shape[] {
  const byId = new Map(input.cards.map((card) => [card.id, card]));
  const shapes: Shape[] = [];

  for (const attack of input.attackers) {
    const from = byId.get(attack.attacker);
    const to = input.avatars[attack.defender];
    if (from == null || to == null) continue;
    shapes.push(...arrowPath(cardCenter(input.camera, from), to, ATTACK_STROKE));
  }

  for (const block of input.blocks) {
    const from = byId.get(block.blocker);
    const to = byId.get(block.attacker);
    if (from == null || to == null) continue;
    shapes.push(...arrowPath(cardCenter(input.camera, from), cardCenter(input.camera, to), BLOCK_STROKE));
  }

  return shapes;
}
