import { Canvas } from "foldkit";
import { colors } from "~/design-tokens.generated";
import type { StackObjectView, WireAttack, WireBlock } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { type Camera, worldToScreen } from "../geometry/camera";
import type { RenderCard } from "../geometry/layout";
import { STACK_PEEK, type StackPresentation, stackFaceScreenOrigin } from "../geometry/stackLayout";
import type { AvatarScreenPositions } from "./avatars";

type Shape = Canvas.Shape;

// Deliberately not colors.mountainRed — attack arrow paint (#ff6b6b) differs from the combat Mountain Red token (#FF5555).
const ATTACK_STROKE = "#ff6b6b";
const BLOCK_STROKE = colors.wallGreen;

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

/** Screen-space center of stack pile face at `row` (0 = bottom, last = top). */
export function stackPileFaceOrigin(
  viewportW: number,
  viewportH: number,
  count: number,
  row: number,
  peek = STACK_PEEK,
): Vec {
  return stackFaceScreenOrigin({
    presentation: "pile",
    viewportW,
    viewportH,
    count,
    row,
    peek,
  });
}

/** Declared stack targets → Island Blue arrows (presentation-aware origins). */
export function stackTargetArrowShapes(input: {
  viewport: { width: number; height: number };
  stack: ReadonlyArray<StackObjectView>;
  cards: ReadonlyArray<RenderCard>;
  avatars: AvatarScreenPositions;
  camera: Camera;
  presentation?: StackPresentation;
}): Shape[] {
  const count = input.stack.length;
  if (count === 0) return [];
  const presentation = input.presentation ?? "pile";
  const byId = new Map(input.cards.map((card) => [card.id, card]));
  const shapes: Shape[] = [];
  for (let row = 0; row < count; row++) {
    const entry = input.stack[row];
    if (entry?.target == null) continue;
    const from = stackFaceScreenOrigin({
      presentation,
      viewportW: input.viewport.width,
      viewportH: input.viewport.height,
      count,
      row,
    });
    let to: Vec | null = null;
    if (entry.target.kind === "player") {
      to = input.avatars[entry.target.player] ?? null;
    } else {
      const card = byId.get(entry.target.id);
      if (card != null) to = cardCenter(input.camera, card);
    }
    if (to == null) continue;
    shapes.push(...arrowPath(from, to, TARGET_COLOR));
  }
  return shapes;
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
