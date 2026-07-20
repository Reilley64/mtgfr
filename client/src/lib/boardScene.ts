// Retained board scene: pure build decisions, dumb pixel painter.

import { AVATAR_R, type RenderCard, STEP, seatBand, seatColor } from "~/layout";
import { type ArrowAnimState, arrowBetweenWithProgress, arrowProgressFor, pruneArrowBorn } from "~/lib/boardArrows";
import { drawAvatar, ringAvatar } from "~/lib/boardAvatarPaint";
import { drawCard, drawFlightCard } from "~/lib/boardCardPaint";
import { drawFelt } from "~/lib/boardFelt";
import {
  ATTACK_STROKE,
  BLOCK_STROKE,
  roundRect,
  SELECT_STROKE,
  type Stroke,
  TARGET_STROKE,
  type Vec,
} from "~/lib/boardPaintPrims";
import { type Camera, worldToScreen } from "~/lib/camera";
import type { CardFlight } from "~/lib/cardFlight";
import type { ImageCache } from "~/lib/imageCache";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";

export type SeatBandPaint = {
  seat: number;
  x: number;
  y: number;
  w: number;
  h: number;
  active: boolean;
};

export type CardPaint = {
  card: RenderCard;
  outline: Stroke | null;
  glow: string | null;
  dim: boolean;
  autoTapPreview: boolean;
};

export type AvatarPaint = {
  player: VisibleState["players"][number];
  pos: Vec;
  radius: number;
  priority: boolean;
  targetRing: boolean;
};

export type ArrowPaint = {
  from: Vec;
  to: Vec;
  stroke: Stroke;
  key: string;
};

export type BoardScene = {
  viewport: { w: number; h: number };
  cam: Camera;
  viewer: number;
  seats: SeatBandPaint[];
  cards: CardPaint[];
  flights: CardFlight[];
  avatars: AvatarPaint[];
  arrows: ArrowPaint[];
  nowMs: number;
  reducedMotion: boolean;
};

/** Inputs for {@link buildBoardScene} — same facts as the former `DrawCtx`, plus clock. */
export type BuildBoardSceneInput = {
  cam: Camera;
  cards: RenderCard[];
  me: number;
  active: number;
  priority: number;
  viewer: number;
  count: number;
  players: VisibleState["players"];
  combat: VisibleState["combat"];
  attackers: WireAttack[];
  blocks: WireBlock[];
  aiming: boolean;
  targetObjects: ReadonlySet<number>;
  targetPlayers: ReadonlySet<number>;
  canvasDrag: RenderCard | null;
  cursor: Vec;
  avatarScreenPositions: Record<number, Vec>;
  stepIdx: number;
  selectedId: number | null;
  paymentObjects: ReadonlySet<number>;
  stackResponseFocus: boolean;
  responseObjects: ReadonlySet<number>;
  aimFrom: Vec | null;
  flights?: readonly CardFlight[];
  hideCardIds?: ReadonlySet<number>;
  viewportW: number;
  viewportH: number;
  nowMs: number;
  reducedMotion: boolean;
};

function cardCenter(cam: Camera, c: RenderCard): Vec {
  return worldToScreen(cam, c.x + c.w / 2, c.y + c.h / 2);
}

/** Pure: resolve outlines, dimming, arrow endpoints — no canvas I/O. */
export function buildBoardScene(d: BuildBoardSceneInput): BoardScene {
  const seats: SeatBandPaint[] = d.players.map((p) => {
    const band = seatBand(p.player, d.viewer, d.count);
    const tl = worldToScreen(d.cam, band.x, band.y);
    return {
      seat: p.player,
      x: tl.x,
      y: tl.y,
      w: band.w * d.cam.zoom,
      h: band.h * d.cam.zoom,
      active: p.player === d.active,
    };
  });

  const attackerSet = new Set(d.attackers.map((a) => a.attacker));
  const blockerSet = new Set(d.blocks.map((b) => b.blocker));
  const incoming = new Set(d.combat.attackers.map((a) => a.attacker));
  const dimOthers = !d.aiming && d.stackResponseFocus;

  const cards: CardPaint[] = [];
  for (const card of d.cards) {
    if (d.hideCardIds?.has(card.id)) continue;
    let outline: Stroke | null = null;
    let glow: string | null = null;
    if (attackerSet.has(card.id) || (incoming.has(card.id) && card.controller !== d.me)) {
      outline = ATTACK_STROKE;
      glow = "rgba(255,85,85,0.45)";
    } else if (blockerSet.has(card.id)) {
      outline = BLOCK_STROKE;
      glow = "rgba(102,255,153,0.4)";
    } else if (d.aiming && d.targetObjects.has(card.id)) {
      outline = TARGET_STROKE;
      glow = "rgba(119,204,255,0.5)";
    } else if (d.selectedId === card.id) {
      outline = SELECT_STROKE;
      glow = "rgba(255,215,106,0.55)";
    }
    cards.push({
      card,
      outline,
      glow,
      dim: dimOthers && !d.responseObjects.has(card.id) && d.selectedId !== card.id,
      autoTapPreview: d.paymentObjects.has(card.id),
    });
  }

  const flights = (d.flights ?? []).filter((f) => f.phase !== "settled");

  const rad = AVATAR_R * d.cam.zoom;
  const avatars: AvatarPaint[] = [];
  for (const p of d.players) {
    const scr = d.avatarScreenPositions[p.player];
    if (!scr) continue;
    avatars.push({
      player: p,
      pos: scr,
      radius: rad,
      priority: d.priority === p.player,
      targetRing: d.aiming && d.targetPlayers.has(p.player),
    });
  }

  const arrows: ArrowPaint[] = [];
  for (const a of d.attackers) {
    const c = d.cards.find((x) => x.id === a.attacker);
    const to = d.avatarScreenPositions[a.defender];
    if (c && to) {
      arrows.push({
        from: cardCenter(d.cam, c),
        to,
        stroke: ATTACK_STROKE,
        key: `stage-atk-${a.attacker}-${a.defender}`,
      });
    }
  }
  for (const a of d.combat.attackers) {
    const c = d.cards.find((x) => x.id === a.attacker);
    const to = d.avatarScreenPositions[a.defender];
    if (c && to) {
      arrows.push({
        from: cardCenter(d.cam, c),
        to,
        stroke: ATTACK_STROKE,
        key: `atk-${a.attacker}-${a.defender}`,
      });
    }
  }
  for (const b of d.blocks) {
    const from = d.cards.find((x) => x.id === b.blocker);
    const to = d.cards.find((x) => x.id === b.attacker);
    if (from && to) {
      arrows.push({
        from: cardCenter(d.cam, from),
        to: cardCenter(d.cam, to),
        stroke: BLOCK_STROKE,
        key: `blk-${b.blocker}-${b.attacker}`,
      });
    }
  }
  if (d.canvasDrag) {
    const declaringBlock = d.stepIdx === STEP.DeclareBlockers && d.active !== d.me;
    arrows.push({
      from: cardCenter(d.cam, d.canvasDrag),
      to: d.cursor,
      stroke: declaringBlock ? BLOCK_STROKE : ATTACK_STROKE,
      key: `drag-${d.canvasDrag.id}`,
    });
  }
  if (d.aiming && d.aimFrom) {
    arrows.push({ from: d.aimFrom, to: d.cursor, stroke: TARGET_STROKE, key: "aim" });
  }

  return {
    viewport: { w: d.viewportW, h: d.viewportH },
    cam: d.cam,
    viewer: d.viewer,
    seats,
    cards,
    flights,
    avatars,
    arrows,
    nowMs: d.nowMs,
    reducedMotion: d.reducedMotion,
  };
}

export type PaintBoardResult = {
  stillAnimating: boolean;
  arrowState: ArrowAnimState;
};

/** Dumb painter: strokes the scene; updates injectable arrow birth state. */
export function paintBoardScene(
  ctx: CanvasRenderingContext2D,
  scene: BoardScene,
  cache: ImageCache,
  arrowState: ArrowAnimState,
): PaintBoardResult {
  const { w, h } = scene.viewport;
  ctx.clearRect(0, 0, w, h);
  drawFelt(ctx, w, h);

  for (const seat of scene.seats) {
    ctx.save();
    ctx.fillStyle = seatColor(seat.seat, seat.active ? 0.12 : 0.06);
    roundRect(ctx, seat.x, seat.y, seat.w, seat.h, 12 * scene.cam.zoom);
    ctx.fill();
    ctx.strokeStyle = seatColor(seat.seat, seat.active ? 0.65 : 0.28);
    ctx.lineWidth = seat.active ? 2.5 : 1.5;
    roundRect(ctx, seat.x, seat.y, seat.w, seat.h, 12 * scene.cam.zoom);
    ctx.stroke();
    ctx.restore();
  }

  for (const c of scene.cards) {
    drawCard(ctx, scene.cam, c.card, cache, c.outline, scene.viewer, c.glow, c.dim, c.autoTapPreview);
  }

  let stillAnimating = scene.flights.length > 0;
  for (const f of scene.flights) {
    drawFlightCard(ctx, scene.cam, f, cache);
  }

  for (const a of scene.avatars) {
    drawAvatar(ctx, a.pos, a.radius, a.player, a.priority);
    if (a.targetRing) ringAvatar(ctx, a.pos, a.radius, TARGET_STROKE);
  }

  const born = arrowState.born;
  const seen = new Set<string>();
  for (const arrow of scene.arrows) {
    seen.add(arrow.key);
    const { progress, animating } = arrowProgressFor(born, arrow.key, scene.nowMs, scene.reducedMotion);
    if (animating) stillAnimating = true;
    arrowBetweenWithProgress(ctx, arrow.from, arrow.to, arrow.stroke, progress);
  }
  pruneArrowBorn(born, seen);

  return { stillAnimating, arrowState: { born } };
}
