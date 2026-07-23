import { Canvas } from "foldkit";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { CARD_CORNER_RADIUS } from "../bitmap/paint-cards";
import { CARD_RESTING_OUTLINE, COMMANDER_GOLD, PLAYABLE_BORDER, playableBattlefieldObjectIds } from "../chrome";
import { type Camera, worldToScreen } from "../geometry/camera";
import { fitCamera } from "../geometry/interaction";
import { CARD_H, CARD_W, layout, type RenderCard, seatBand, seatColor } from "../geometry/layout";
import { BOARD_VIEWPORT } from "../submodel";
import { aimArrowShapes, arrowShapes, combatDragArrowShapes } from "./arrows";
import { avatarScreenPositions, avatarShapes } from "./avatars";
import { feltShapes } from "./felt";

type Shape = Canvas.Shape;

/** Axis-aligned rounded rect as a Foldkit Path (bitmap cards use the same corner radius). */
function roundedRectPath(
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
  fill: string | undefined,
  stroke: string | undefined,
  lineWidth: number,
): Shape {
  const r = Math.max(0, Math.min(radius, width / 2, height / 2));
  return Canvas.Path({
    instructions: [
      Canvas.MoveTo({ x: x + r, y }),
      Canvas.LineTo({ x: x + width - r, y }),
      Canvas.QuadTo({ cpx: x + width, cpy: y, x: x + width, y: y + r }),
      Canvas.LineTo({ x: x + width, y: y + height - r }),
      Canvas.QuadTo({ cpx: x + width, cpy: y + height, x: x + width - r, y: y + height }),
      Canvas.LineTo({ x: x + r, y: y + height }),
      Canvas.QuadTo({ cpx: x, cpy: y + height, x, y: y + height - r }),
      Canvas.LineTo({ x, y: y + r }),
      Canvas.QuadTo({ cpx: x, cpy: y, x: x + r, y }),
      Canvas.Close(),
    ],
    fill,
    stroke,
    lineWidth,
    lineJoin: "Round",
  });
}

export type StagedTargeting = {
  targetObjects: ReadonlySet<number>;
  targetPlayers: ReadonlySet<number>;
  aimFrom: { x: number; y: number };
  cursor: { x: number; y: number };
};

export type SceneShapesOptions = {
  width?: number;
  height?: number;
  camera?: Camera;
  selectedId?: number | null;
  stagedAttackers?: ReadonlyArray<WireAttack>;
  stagedBlocks?: ReadonlyArray<WireBlock>;
  stagedTargeting?: StagedTargeting | null;
  combatDrag?: { from: { x: number; y: number }; to: { x: number; y: number }; declaringBlock: boolean } | null;
};

function seatShapes(state: VisibleState, camera: Camera): Shape[] {
  const count = Math.max(1, state.players.length);
  return state.players.map((player) => {
    const band = seatBand(player.player, state.viewer, count);
    const topLeft = worldToScreen(camera, band.x, band.y);
    const active = player.player === state.active_player;
    return Canvas.Rect({
      x: topLeft.x,
      y: topLeft.y,
      width: band.w * camera.zoom,
      height: band.h * camera.zoom,
      fill: seatColor(player.player, active ? 0.12 : 0.06),
      stroke: seatColor(player.player, active ? 0.65 : 0.28),
      lineWidth: active ? 2.5 : 1.5,
    });
  });
}

function kindFill(card: RenderCard): string {
  switch (card.kind) {
    case "creature":
      return "#1b3329";
    case "land":
      return "#223820";
    case "artifact":
      return "#2d302f";
    case "enchantment":
      return "#2a2540";
    case "planeswalker":
      return "#3a2a20";
    case "instant":
    case "sorcery":
      return "#1d2b3a";
    default: {
      const exhaustive: never = card.kind;
      return exhaustive;
    }
  }
}

function cardRotation(card: RenderCard, viewer: number): number {
  const tapFrac = card.tapFrac ?? (card.tapped ? 1 : 0);
  let angle = card.controller !== viewer ? Math.PI : 0;
  angle += card.fanAngle ?? 0;
  angle += tapFrac * (Math.PI / 2);
  return angle;
}

function cardShapes(
  cards: ReadonlyArray<RenderCard>,
  camera: Camera,
  selectedId: number | null,
  viewer: number,
  targetObjects: ReadonlySet<number>,
  playableObjects: ReadonlySet<number>,
): Shape[] {
  const shapes: Shape[] = [];
  for (const card of cards) {
    const topLeft = worldToScreen(camera, card.x, card.y);
    const width = card.w * camera.zoom;
    const height = card.h * camera.zoom;
    const left = -width / 2;
    const top = -height / 2;
    const selected = card.id === selectedId;
    const targeted = targetObjects.has(card.id);
    const playable = playableObjects.has(card.id);
    const cardParts: Shape[] = [];
    const cardStroke = targeted
      ? TARGET_COLOR
      : selected
        ? "#ffd76a"
        : card.isCommander
          ? COMMANDER_GOLD
          : playable
            ? PLAYABLE_BORDER
            : CARD_RESTING_OUTLINE;

    const corner = CARD_CORNER_RADIUS * camera.zoom;
    cardParts.push(
      roundedRectPath(
        left,
        top,
        width,
        height,
        corner,
        card.faceDown ? "#1a1623" : kindFill(card),
        cardStroke,
        targeted || selected || card.isCommander || playable ? 3 : 1.5,
      ),
    );

    if (card.isCommander && playable) {
      cardParts.push(
        roundedRectPath(left, top, width, height, corner, undefined, PLAYABLE_BORDER, 2),
      );
    }

    if (card.pt !== "") {
      cardParts.push(
        Canvas.Text({
          x: width / 2 - 8 * camera.zoom,
          y: height / 2 - 10 * camera.zoom,
          content: card.pt,
          font: `700 ${Math.max(1, Math.round(11 * camera.zoom))}px system-ui, sans-serif`,
          fill: "#eaf7ef",
          align: "Right",
          baseline: "Middle",
        }),
      );
    }

    if (card.pile > 1 || card.cluster > 1) {
      cardParts.push(
        Canvas.Text({
          x: width / 2 - 8 * camera.zoom,
          y: top + 10 * camera.zoom,
          content: `x${card.pile || card.cluster}`,
          font: `700 ${Math.max(1, Math.round(11 * camera.zoom))}px system-ui, sans-serif`,
          fill: "#ffd76a",
          align: "Right",
          baseline: "Middle",
        }),
      );
    }

    shapes.push(
      Canvas.Group({
        translate: { x: topLeft.x + width / 2, y: topLeft.y + height / 2 },
        rotate: cardRotation(card, viewer),
        shapes: cardParts,
      }),
    );
  }
  return shapes;
}

export function sceneShapes(state: VisibleState, options: SceneShapesOptions = {}): Shape[] {
  const width = options.width ?? BOARD_VIEWPORT.width;
  const height = options.height ?? BOARD_VIEWPORT.height;
  const count = Math.max(1, state.players.length);
  const camera = options.camera ?? fitCamera({ x: width, y: height }, count, 0);
  const cards = layout(state, state.viewer);
  const avatars = avatarScreenPositions(state.players, state.viewer, count, camera);
  const targeting = options.stagedTargeting ?? null;
  const targetObjects = targeting?.targetObjects ?? new Set<number>();
  const playableObjects = playableBattlefieldObjectIds(
    state.actions,
    cards.map((card) => ({
      id: card.id,
      summoningSick: card.summoningSick,
      hasHaste: card.hasHaste,
    })),
  );

  return [
    ...feltShapes(width, height),
    ...seatShapes(state, camera),
    ...cardShapes(cards, camera, options.selectedId ?? null, state.viewer, targetObjects, playableObjects),
    ...avatarShapes(state.players, avatars, state.priority, camera.zoom, targeting?.targetPlayers ?? new Set()),
    ...arrowShapes({
      camera,
      cards,
      avatars,
      attackers: [...(options.stagedAttackers ?? []), ...state.combat.attackers],
      blocks: [...(options.stagedBlocks ?? []), ...state.combat.blocks],
    }),
    ...(targeting == null ? [] : aimArrowShapes({ from: targeting.aimFrom, to: targeting.cursor })),
    ...(options.combatDrag == null
      ? []
      : combatDragArrowShapes({
          from: options.combatDrag.from,
          to: options.combatDrag.to,
          declaringBlock: options.combatDrag.declaringBlock,
        })),
  ];
}

export { CARD_H, CARD_W };
