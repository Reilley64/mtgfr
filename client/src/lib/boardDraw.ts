// Canvas board drawing: seat bands, cards, avatars, combat/target arrows.

import { AVATAR_R, type RenderCard, STEP, seatBand, seatColor } from "~/layout";
import {
  arrowBetween,
  arrowsNeedFrame,
  markArrowsAnimating,
  pruneArrows,
  resetArrowAnimFlag,
} from "~/lib/boardArrows";
import { drawAvatar, ringAvatar } from "~/lib/boardAvatarPaint";
import { drawCard, drawFlightCard } from "~/lib/boardCardPaint";
import { drawFelt } from "~/lib/boardFelt";
import {
  ATTACK_STROKE,
  BLOCK_STROKE,
  roundRect,
  SELECT_STROKE,
  TARGET_STROKE,
  type Vec,
} from "~/lib/boardPaintPrims";
import { type Camera, worldToScreen } from "~/lib/camera";
import type { CardFlight } from "~/lib/cardFlight";
import type { ImageCache } from "~/lib/imageCache";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";

export type { StackPresentation } from "~/lib/stackLayout";
export {
  STACK_ANCHOR_FROM_RIGHT,
  STACK_CARD_W,
  STACK_EXPAND_COUNT,
  STACK_HORIZONTAL_MARGIN,
  STACK_OVERLAY_RIGHT,
  STACK_PEEK,
  STACK_STRIP_MIN_PEEK,
  STACK_VERTICAL_RESERVED,
  shouldAutoCollapseStackExpand,
  stackAimOrigin,
  stackCardH,
  stackExpandAvailable,
  stackFullPerRow,
  stackPeekFor,
  stackPresentation,
  stackStripFits,
  stackStripPeek,
  stagingAimFrom,
} from "~/lib/stackLayout";

export { ARROW_DRAW_MS, arrowDrawProgress } from "~/lib/boardArrows";
export { RESPONSE_COLOR, TARGET_COLOR } from "~/lib/boardPaintPrims";

export interface DrawCtx {
  cam: Camera;
  cards: RenderCard[];
  cache: ImageCache;
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
  /** Selected permanent for the activation radial. */
  selectedId: number | null;
  /** Permanents that would auto-tap to pay the hovered action's mana. */
  paymentObjects: ReadonlySet<number>;
  /** True while this viewer holds instant-speed priority (dim non-usable permanents). */
  stackResponseFocus: boolean;
  /** Battlefield activates + mana sources to keep bright during instant priority. */
  responseObjects: ReadonlySet<number>;
  /** Screen-space origin for the targeting arrow while aiming (stack-card center). */
  aimFrom: Vec | null;
  /** In-flight cards (ADR 0035) — drawn in screen space above board cards. */
  flights?: readonly CardFlight[];
  /** Canvas card ids still owned by a flight — skip drawing the resting face. */
  hideCardIds?: ReadonlySet<number>;
}

export function draw(ctx: CanvasRenderingContext2D, d: DrawCtx): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = ctx.canvas.width / dpr;
  const h = ctx.canvas.height / dpr;
  ctx.clearRect(0, 0, w, h);
  drawFelt(ctx, w, h);
  resetArrowAnimFlag();

  const rad = AVATAR_R * d.cam.zoom;

  for (const p of d.players) {
    const seat = p.player;
    const band = seatBand(seat, d.viewer, d.count);
    const tl = worldToScreen(d.cam, band.x, band.y);
    ctx.save();
    // Soft seat wash + border for every seat; active reads louder so turn ownership stays clear.
    ctx.fillStyle = seatColor(seat, seat === d.active ? 0.12 : 0.06);
    roundRect(ctx, tl.x, tl.y, band.w * d.cam.zoom, band.h * d.cam.zoom, 12 * d.cam.zoom);
    ctx.fill();
    ctx.strokeStyle = seatColor(seat, seat === d.active ? 0.65 : 0.28);
    ctx.lineWidth = seat === d.active ? 2.5 : 1.5;
    roundRect(ctx, tl.x, tl.y, band.w * d.cam.zoom, band.h * d.cam.zoom, 12 * d.cam.zoom);
    ctx.stroke();
    ctx.restore();
  }

  const attackerSet = new Set(d.attackers.map((a) => a.attacker));
  const blockerSet = new Set(d.blocks.map((b) => b.blocker));
  const incoming = new Set(d.combat.attackers.map((a) => a.attacker));
  // Instant-speed priority: dim permanents that aren't usable now (mana + legal activates stay bright).
  // Skip during arrow targeting — legal targets already get their own highlight.
  const dimOthers = !d.aiming && d.stackResponseFocus;

  for (const card of d.cards) {
    if (d.hideCardIds?.has(card.id)) continue;
    let outline: (typeof ATTACK_STROKE) | null = null;
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
    const dim = dimOthers && !d.responseObjects.has(card.id) && d.selectedId !== card.id;
    drawCard(ctx, d.cam, card, d.cache, outline, d.viewer, glow, dim, d.paymentObjects.has(card.id));
  }

  if (d.flights?.length) {
    for (const f of d.flights) {
      if (f.phase === "settled") continue;
      drawFlightCard(ctx, d.cam, f, d.cache);
      markArrowsAnimating(); // keep the paint loop alive while flights ease
    }
  }

  for (const p of d.players) {
    const scr = d.avatarScreenPositions[p.player];
    if (!scr) continue;
    drawAvatar(ctx, scr, rad, p, d.priority === p.player);
    if (d.aiming && d.targetPlayers.has(p.player)) ringAvatar(ctx, scr, rad, TARGET_STROKE);
  }

  for (const a of d.attackers) {
    const c = d.cards.find((x) => x.id === a.attacker);
    const to = d.avatarScreenPositions[a.defender];
    if (c && to) arrowBetween(ctx, cardCenter(d.cam, c), to, ATTACK_STROKE, `stage-atk-${a.attacker}-${a.defender}`);
  }
  for (const a of d.combat.attackers) {
    const c = d.cards.find((x) => x.id === a.attacker);
    const to = d.avatarScreenPositions[a.defender];
    if (c && to) arrowBetween(ctx, cardCenter(d.cam, c), to, ATTACK_STROKE, `atk-${a.attacker}-${a.defender}`);
  }
  for (const b of d.blocks) {
    const from = d.cards.find((x) => x.id === b.blocker);
    const to = d.cards.find((x) => x.id === b.attacker);
    if (from && to)
      arrowBetween(ctx, cardCenter(d.cam, from), cardCenter(d.cam, to), BLOCK_STROKE, `blk-${b.blocker}-${b.attacker}`);
  }

  if (d.canvasDrag) {
    const declaringBlock = d.stepIdx === STEP.DeclareBlockers && d.active !== d.me;
    arrowBetween(
      ctx,
      cardCenter(d.cam, d.canvasDrag),
      d.cursor,
      declaringBlock ? BLOCK_STROKE : ATTACK_STROKE,
      `drag-${d.canvasDrag.id}`,
    );
  }

  if (d.aiming && d.aimFrom) {
    arrowBetween(ctx, d.aimFrom, d.cursor, TARGET_STROKE, "aim");
  }
  pruneArrows();
  // True while any arrow's draw-on is incomplete — caller must keep painting until this is false,
  // otherwise a one-shot redraw leaves the tip stuck on the source (e.g. staged attackers).
  return arrowsNeedFrame();
}

function cardCenter(cam: Camera, c: RenderCard): Vec {
  return worldToScreen(cam, c.x + c.w / 2, c.y + c.h / 2);
}
