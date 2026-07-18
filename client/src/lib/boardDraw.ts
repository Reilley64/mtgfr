// Canvas board drawing: seat bands, cards, avatars, combat/target arrows.

import { AVATAR_R, CARD_H, CARD_W, type RenderCard, STEP, seatBand, seatColor, ZONE } from "~/layout";
import { type Camera, worldToScreen } from "~/lib/camera";
import { abilityGlyph, hiddenKeywordCount, keywordBadges, showsSummoningSick, TAP_GLYPH } from "~/lib/cardBadges";
import type { CardFlight } from "~/lib/cardFlight";
import type { ImageCache } from "~/lib/imageCache";
import { LETHAL_COMMANDER_DAMAGE, worstCommanderDamage } from "~/lib/outcome";
import { cardBackUrl, imageUrlByPrint } from "~/lib/scryfall";
import type { VisibleState, WireAttack, WireBlock } from "~/wire/types";

type Vec = { x: number; y: number };

// Stack overlay geometry — one source for the DOM overlay and the canvas entrance seed.
export const STACK_CARD_W = 180;
export const STACK_OVERLAY_RIGHT = 16;
/** Vertical peek of each lower card in the physical pile. */
export const STACK_PEEK = 34;
/** Screen-x inset from the right edge to the stack card centerline (glide anchor). */
export const STACK_ANCHOR_FROM_RIGHT = STACK_OVERLAY_RIGHT + STACK_CARD_W / 2;
/** Object count at which the magnifier appears even at full peek. */
export const STACK_EXPAND_COUNT = 6;
/** Tightest horizontal peek before escalating to full stack view. */
export const STACK_STRIP_MIN_PEEK = 20;
/** Default vertical chrome reserved under/above the centered pile (captions / hold). */
export const STACK_VERTICAL_RESERVED = 120;
/** Horizontal inset when measuring strip/full width. */
export const STACK_HORIZONTAL_MARGIN = 48;

export function stackCardH(cardW = STACK_CARD_W): number {
  return cardW / 0.716;
}

/**
 * Vertical peek so `cardH + (n-1)*peek` fits the usable viewport band.
 * Never exceeds {@link STACK_PEEK}; may compress to 0.
 */
export function stackPeekFor(count: number, viewportH: number, reserved = STACK_VERTICAL_RESERVED): number {
  const n = Math.max(1, count);
  if (n <= 1) return STACK_PEEK;
  const cardH = stackCardH();
  const maxPileH = Math.max(cardH, viewportH - reserved);
  const peek = (maxPileH - cardH) / (n - 1);
  return Math.min(STACK_PEEK, Math.max(0, peek));
}

/** Magnifier / expand control: reading threshold or compression has started. */
export function stackExpandAvailable(count: number, peek: number): boolean {
  if (count >= STACK_EXPAND_COUNT) return true;
  return peek < STACK_PEEK - 0.01;
}

/**
 * Auto-collapse expand/full when thresholds clear or the stack empties.
 * Keeps expand open while a local staged target is live so aim does not re-arm mid-read.
 */
export function shouldAutoCollapseStackExpand(opts: {
  expanded: boolean;
  count: number;
  peek: number;
  staged: boolean;
}): boolean {
  if (!opts.expanded) return false;
  if (opts.count <= 0) return true;
  if (opts.staged) return false;
  return !stackExpandAvailable(opts.count, opts.peek);
}

/** Whether a horizontal strip of `count` cards fits at {@link STACK_STRIP_MIN_PEEK}. */
export function stackStripFits(
  count: number,
  viewportW: number,
  cardW = STACK_CARD_W,
  minPeek = STACK_STRIP_MIN_PEEK,
  margin = STACK_HORIZONTAL_MARGIN,
): boolean {
  if (count <= 1) return true;
  return cardW + (count - 1) * minPeek <= viewportW - margin;
}

/** Horizontal peek for the expanded strip (compresses to fit; caller escalates if below min). */
export function stackStripPeek(
  count: number,
  viewportW: number,
  cardW = STACK_CARD_W,
  margin = STACK_HORIZONTAL_MARGIN,
): number {
  if (count <= 1) return STACK_PEEK;
  const budget = viewportW - margin - cardW;
  return Math.min(STACK_PEEK, Math.max(0, budget / (count - 1)));
}

export type StackPresentation = "pile" | "expanded" | "full";

/** Active stack presentation given expand open-state and viewport. */
export function stackPresentation(opts: {
  count: number;
  expandedOpen: boolean;
  viewportW: number;
  viewportH: number;
}): StackPresentation {
  if (!opts.expandedOpen || opts.count <= 0) return "pile";
  if (!stackStripFits(opts.count, opts.viewportW)) return "full";
  return "expanded";
}

/** Shared target-arrow / staged-preview accent (canvas stroke + DOM ring). */
export const TARGET_COLOR = "#77CCFF";
/** Legend sample for a permanent that stays bright during stack response. */
export const RESPONSE_COLOR = "#EAFFF0";

/**
 * Screen-space center of the top card in a right-edge pile of `count` cards.
 * Assumes the pile box alone is viewport-centered (`top-1/2 -translate-y-1/2`); captions and
 * buttons must sit outside that box so they don't shift the pile.
 */
export function stackAimOrigin(
  viewportW: number,
  viewportH: number,
  count: number,
  peek: number = STACK_PEEK,
): { x: number; y: number } {
  const n = Math.max(1, count);
  const cardH = stackCardH();
  const pileH = cardH + (n - 1) * peek;
  return {
    x: viewportW - STACK_ANCHOR_FROM_RIGHT,
    // Simplifies to viewportH/2 - (n-1)*peek/2.
    y: viewportH / 2 + pileH / 2 - (n - 1) * peek - cardH / 2,
  };
}

/** Aim origin while a hand card is staged in arrow-target mode; `null` for pick/none/expand. */
export function stagingAimFrom(
  viewportW: number,
  viewportH: number,
  stackLen: number,
  arrowStaging: boolean,
  peek: number = STACK_PEEK,
): { x: number; y: number } | null {
  if (!arrowStaging) return null;
  return stackAimOrigin(viewportW, viewportH, stackLen + 1, peek);
}

/** Max cards per row in full view at min horizontal peek (wrap capacity). */
export function stackFullPerRow(
  viewportW: number,
  cardW = STACK_CARD_W,
  minPeek = STACK_STRIP_MIN_PEEK,
  margin = STACK_HORIZONTAL_MARGIN,
): number {
  if (minPeek <= 0) return Math.max(1, Math.floor((viewportW - margin) / cardW));
  return Math.max(1, Math.floor((viewportW - margin - cardW) / minPeek) + 1);
}

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

interface Stroke {
  color: string;
  dash: number[];
}
const ATTACK_STROKE: Stroke = { color: "#FF5555", dash: [] };
const BLOCK_STROKE: Stroke = { color: "#66FF99", dash: [10, 6] };
const TARGET_STROKE: Stroke = { color: TARGET_COLOR, dash: [2, 6] };
const SELECT_STROKE: Stroke = { color: "#FFD76A", dash: [] };
const CARD_OUTLINE = "#1a1a1a";
/** Black veil over non-activatable permanents — matches hand `opacity-55` (≈45% black). */
const DIM_CARD_VEIL = 0.45;

export function draw(ctx: CanvasRenderingContext2D, d: DrawCtx): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = ctx.canvas.width / dpr;
  const h = ctx.canvas.height / dpr;
  ctx.clearRect(0, 0, w, h);
  drawFelt(ctx, w, h);
  arrowsAnimating = false;

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
    const dim = dimOthers && !d.responseObjects.has(card.id) && d.selectedId !== card.id;
    drawCard(ctx, d.cam, card, d.cache, outline, d.viewer, glow, dim, d.paymentObjects.has(card.id));
  }

  if (d.flights?.length) {
    for (const f of d.flights) {
      if (f.phase === "settled") continue;
      drawFlightCard(ctx, d.cam, f, d.cache);
      arrowsAnimating = true; // keep the paint loop alive while flights ease
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
  return arrowsAnimating;
}

function cardCenter(cam: Camera, c: RenderCard): Vec {
  return worldToScreen(cam, c.x + c.w / 2, c.y + c.h / 2);
}

function drawFelt(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.save();
  ctx.fillStyle = "#0B1310";
  ctx.fillRect(0, 0, w, h);
  // Deterministic speckles so the felt doesn't shimmer every frame.
  ctx.globalAlpha = 0.04;
  ctx.fillStyle = "#1a2a22";
  for (let i = 0; i < 120; i++) {
    const x = ((i * 73) % 97) / 97;
    const y = ((i * 41) % 89) / 89;
    ctx.fillRect(x * w, y * h, 2, 2);
  }
  ctx.globalAlpha = 1;
  const g = ctx.createRadialGradient(w / 2, h / 2, Math.min(w, h) * 0.2, w / 2, h / 2, Math.max(w, h) * 0.72);
  g.addColorStop(0, "rgba(0,0,0,0)");
  g.addColorStop(1, "rgba(0,0,0,0.45)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  ctx.restore();
}

function drawAvatar(
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

function ringAvatar(ctx: CanvasRenderingContext2D, pos: Vec, radius: number, stroke: Stroke) {
  ctx.save();
  ctx.beginPath();
  ctx.arc(pos.x, pos.y, radius + 5, 0, Math.PI * 2);
  ctx.strokeStyle = stroke.color;
  ctx.lineWidth = 3;
  ctx.setLineDash(stroke.dash);
  ctx.stroke();
  ctx.restore();
}

const ARROW_DRAW_MS = 180;

export { ARROW_DRAW_MS };

const arrowBorn = new Map<string, number>();
let arrowsSeenThisFrame = new Set<string>();
let arrowsAnimating = false;

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Fraction of an arrow's draw-on (0 → 1) given birth time and now. */
export function arrowDrawProgress(bornAtMs: number, nowMs: number): number {
  return Math.min(1, Math.max(0, (nowMs - bornAtMs) / ARROW_DRAW_MS));
}

function arrowProgress(key: string): number {
  arrowsSeenThisFrame.add(key);
  if (prefersReducedMotion()) return 1;
  if (!arrowBorn.has(key)) arrowBorn.set(key, performance.now());
  const t = arrowDrawProgress(arrowBorn.get(key) ?? 0, performance.now());
  if (t < 1) arrowsAnimating = true;
  return t;
}

function pruneArrows() {
  for (const k of [...arrowBorn.keys()]) {
    if (!arrowsSeenThisFrame.has(k)) arrowBorn.delete(k);
  }
  arrowsSeenThisFrame = new Set();
}

function arrowBetween(ctx: CanvasRenderingContext2D, a: Vec, b: Vec, stroke: Stroke, key: string) {
  const t = arrowProgress(key);
  const mx = (a.x + b.x) / 2;
  const my = (a.y + b.y) / 2;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  const bulge = Math.min(48, len * 0.22);
  const cx = mx - (dy / len) * bulge;
  const cy = my + (dx / len) * bulge;

  ctx.save();
  ctx.strokeStyle = stroke.color;
  ctx.fillStyle = stroke.color;
  ctx.lineWidth = 3;
  ctx.setLineDash(stroke.dash);
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  const steps = Math.max(2, Math.ceil(24 * Math.max(t, 0.05)));
  let endX = a.x;
  let endY = a.y;
  for (let i = 1; i <= steps; i++) {
    const u = (i / steps) * t;
    endX = (1 - u) * (1 - u) * a.x + 2 * (1 - u) * u * cx + u * u * b.x;
    endY = (1 - u) * (1 - u) * a.y + 2 * (1 - u) * u * cy + u * u * b.y;
    ctx.lineTo(endX, endY);
  }
  const uTip = Math.max(0, t - 0.02);
  const tx0 = (1 - uTip) * (1 - uTip) * a.x + 2 * (1 - uTip) * uTip * cx + uTip * uTip * b.x;
  const ty0 = (1 - uTip) * (1 - uTip) * a.y + 2 * (1 - uTip) * uTip * cy + uTip * uTip * b.y;
  ctx.stroke();
  ctx.setLineDash([]);
  const ang = Math.atan2(endY - ty0, endX - tx0);
  ctx.beginPath();
  ctx.moveTo(endX, endY);
  ctx.lineTo(endX - 13 * Math.cos(ang - 0.4), endY - 13 * Math.sin(ang - 0.4));
  ctx.lineTo(endX - 13 * Math.cos(ang + 0.4), endY - 13 * Math.sin(ang + 0.4));
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

function drawFlightCard(ctx: CanvasRenderingContext2D, cam: Camera, flight: CardFlight, cache: ImageCache) {
  const w = CARD_W * cam.zoom * flight.scale;
  const h = CARD_H * cam.zoom * flight.scale;
  const x = flight.x - w / 2;
  const y = flight.y - h / 2;
  const r = 6 * cam.zoom * Math.max(flight.scale, 0.5);
  ctx.save();
  ctx.shadowColor = "rgba(0,0,0,0.45)";
  ctx.shadowBlur = 16;
  roundRect(ctx, x, y, w, h, r);
  ctx.fillStyle = "#e8e4d8";
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, 2 * cam.zoom);
  ctx.stroke();
  const img = flight.print ? cache.get(imageUrlByPrint(flight.print)) : null;
  if (img) {
    ctx.save();
    roundRect(ctx, x, y, w, h, r);
    ctx.clip();
    ctx.drawImage(img, x, y, w, h);
    ctx.restore();
  } else {
    ctx.fillStyle = "#1a1a1a";
    ctx.font = `bold ${Math.max(10, 12 * cam.zoom * flight.scale)}px system-ui,sans-serif`;
    ctx.textAlign = "center";
    ctx.fillText(flight.name, flight.x, flight.y, w - 8);
  }
  ctx.restore();
}

function drawCard(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  cache: ImageCache,
  outline: Stroke | null,
  viewer: number,
  glow: string | null = null,
  dim = false,
  autoTapPreview = false,
) {
  const tl = worldToScreen(cam, card.x, card.y);
  const w = card.w * cam.zoom;
  const h = card.h * cam.zoom;
  const r = 6 * cam.zoom;

  ctx.save();
  const tapFrac = card.tapFrac ?? (card.tapped ? 1 : 0);
  let angle = card.controller !== viewer ? Math.PI : 0;
  angle += card.fanAngle ?? 0;
  angle += tapFrac * (Math.PI / 2);
  if (angle !== 0) {
    const cx = tl.x + w / 2;
    const cy = tl.y + h / 2;
    ctx.translate(cx, cy);
    ctx.rotate(angle);
    ctx.translate(-cx, -cy);
  }

  if (glow) {
    ctx.shadowColor = glow;
    ctx.shadowBlur = 18 * cam.zoom;
  }
  roundRect(ctx, tl.x, tl.y, w, h, r);
  ctx.fillStyle = card.faceDown ? "#2a3742" : "#e8e4d8";
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.strokeStyle = outline?.color ?? (card.isCommander ? "#e9b84a" : CARD_OUTLINE);
  ctx.lineWidth = Math.max(1, (outline || card.isCommander ? 3 : 2) * cam.zoom);
  ctx.setLineDash(outline?.dash ?? []);
  ctx.stroke();
  ctx.setLineDash([]);

  if (card.faceDown) {
    // Library piles (and any future face-down permanents) share one back image; morph-slate fill
    // above is the placeholder until ImageCache finishes loading it.
    const back = cache.get(cardBackUrl());
    if (back) {
      ctx.save();
      roundRect(ctx, tl.x, tl.y, w, h, r);
      ctx.clip();
      ctx.drawImage(back, tl.x, tl.y, w, h);
      ctx.restore();
    }
    if (card.pile > 0) {
      badge(
        ctx,
        tl.x + w / 2 - 14 * cam.zoom,
        tl.y + h / 2 - 9 * cam.zoom,
        28 * cam.zoom,
        18 * cam.zoom,
        `${card.pile}`,
        cam.zoom,
        CARD_OUTLINE,
        "#eff",
      );
    }
  } else {
    const img = cache.get(imageUrlByPrint(card.print));
    if (img) {
      ctx.save();
      roundRect(ctx, tl.x, tl.y, w, h, r);
      ctx.clip();
      ctx.drawImage(img, tl.x, tl.y, w, h);
      ctx.restore();
    } else {
      ctx.fillStyle = CARD_OUTLINE;
      ctx.font = `${Math.round(9 * cam.zoom)}px system-ui, sans-serif`;
      wrapText(ctx, card.name, tl.x + 6 * cam.zoom, tl.y + 16 * cam.zoom, w - 12 * cam.zoom, 11 * cam.zoom);
    }
    if (card.pile > 0)
      badge(
        ctx,
        tl.x + w / 2 - 14 * cam.zoom,
        tl.y + h / 2 - 9 * cam.zoom,
        28 * cam.zoom,
        18 * cam.zoom,
        `×${card.pile}`,
        cam.zoom,
        CARD_OUTLINE,
        "#eff",
      );
    if (card.cluster > 1)
      badge(
        ctx,
        tl.x + w - 28 * cam.zoom,
        tl.y + 4 * cam.zoom,
        24 * cam.zoom,
        16 * cam.zoom,
        `${card.cluster}`,
        cam.zoom,
        "#1a1a1a",
        "#f4efe2",
      );
    drawStatusBadges(ctx, tl.x, tl.y, w, cam.zoom, card);
    if (card.pt)
      badge(
        ctx,
        tl.x + w - 30 * cam.zoom,
        tl.y + h - 20 * cam.zoom,
        26 * cam.zoom,
        15 * cam.zoom,
        card.pt,
        cam.zoom,
        "#f4efe2",
        "#111",
      );
    if (card.counters > 0)
      badge(
        ctx,
        tl.x + 4 * cam.zoom,
        tl.y + h - 20 * cam.zoom,
        24 * cam.zoom,
        15 * cam.zoom,
        `+${card.counters}`,
        cam.zoom,
        "#2f7d46",
        "#eafff0",
      );
    if (card.markedDamage > 0)
      badge(
        ctx,
        tl.x + w / 2 - 12 * cam.zoom,
        tl.y + h - 20 * cam.zoom,
        24 * cam.zoom,
        15 * cam.zoom,
        `${card.markedDamage}`,
        cam.zoom,
        "#8f2f2f",
        "#ffecec",
      );
  }
  // Dim toward black (hand uses opacity over a dark bar) — not translucent white wash.
  if (dim) {
    roundRect(ctx, tl.x, tl.y, w, h, r);
    ctx.fillStyle = `rgba(0,0,0,${DIM_CARD_VEIL})`;
    ctx.fill();
  }
  if (autoTapPreview) drawAutoTapGlyph(ctx, tl.x, tl.y, w, h, cam.zoom);
  ctx.restore();
}

/** Large centered mana-font tap glyph over a permanent that would auto-tap for the hovered action. */
function drawAutoTapGlyph(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, zoom: number) {
  const size = Math.min(w, h) * 0.55;
  const cx = x + w / 2;
  const cy = y + h / 2;
  ctx.save();
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.font = `${size}px Mana`;
  ctx.lineWidth = Math.max(2, 3 * zoom);
  ctx.strokeStyle = "rgba(10,16,14,0.85)";
  ctx.strokeText(TAP_GLYPH, cx, cy);
  ctx.fillStyle = "#f4efe2";
  ctx.fillText(TAP_GLYPH, cx, cy);
  ctx.restore();
}

function wrapText(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, maxW: number, lineH: number) {
  const words = text.split(" ");
  let line = "";
  let yy = y;
  for (const word of words) {
    const test = line ? `${line} ${word}` : word;
    if (ctx.measureText(test).width > maxW && line) {
      ctx.fillText(line, x, yy);
      line = word;
      yy += lineH;
    } else line = test;
  }
  if (line) ctx.fillText(line, x, yy);
}

function badge(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  text: string,
  zoom: number,
  bg: string,
  fg: string,
) {
  roundRect(ctx, x, y, w, h, 4 * zoom);
  ctx.fillStyle = bg;
  ctx.fill();
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, zoom);
  ctx.stroke();
  ctx.fillStyle = fg;
  ctx.font = `${Math.round(10 * zoom)}px system-ui, sans-serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, x + w / 2, y + h / 2);
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
}

/** Arena-style chrome: summoning-sick icon, commander pip, prepared chip, left-rail keyword icons. */
function drawStatusBadges(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  zoom: number,
  card: RenderCard,
) {
  const pad = 5 * zoom;
  const iconR = 7 * zoom;
  let railY = y + pad + iconR;

  if (showsSummoningSick(card.summoningSick, card.hasHaste)) {
    // Amber chip + Arena summoning-sickness glyph (Mana font).
    drawAbilityChip(ctx, x + pad + iconR, y + pad + iconR, iconR, "summoning_sick", "#e8b24a", "#1a1208");
    railY = y + pad + iconR * 2 + 3 * zoom + iconR;
  }
  if (card.goaded) {
    // Warm rust chip — distinct from dark keyword badges (and from sick amber).
    drawAbilityChip(ctx, x + pad + iconR, railY, iconR, "goaded", "#7a3b13", "#ffecec");
    railY += iconR * 2 + 3 * zoom;
  }
  if (card.prepared) {
    const bw = 14 * zoom;
    const bh = 12 * zoom;
    // Phase-mint — distinct from +1/+1 counter green (#2f7d46). No Arena prepare glyph.
    badge(ctx, x + w - pad - bw, y + pad, bw, bh, "P", zoom, "#55cc99", "#0c1412");
  }
  if (card.isCommander) {
    const cx = x + w - pad - 4 * zoom;
    const cy = y + pad + 4 * zoom;
    // Nudge down if prepared chip occupies the top-right.
    const dy = card.prepared ? 14 * zoom : 0;
    dot(ctx, cx, cy + dy, 4 * zoom, "#e9b84a");
  }

  // Keyword icons along the left rail (below sick badge), Arena-style — battlefield only.
  if (card.zone !== ZONE.Battlefield) return;
  const { shown, overflow } = keywordBadges(card.keywords);
  // Leave room for bottom P/T / counter badges (~22 world units).
  const railFloor = y + card.h * zoom - 22 * zoom;
  const step = iconR * 2 + 2 * zoom;
  let painted = 0;
  for (let i = 0; i < shown.length; i++) {
    if (railY + iconR > railFloor) break;
    const stillHiddenAfter = shown.length - i - 1 + overflow;
    // If more keywords remain after this slot, reserve space for a +N chip instead of
    // painting an icon we'd immediately have to clip with no overflow indicator.
    if (stillHiddenAfter > 0 && railY + step + iconR > railFloor) {
      const hidden = hiddenKeywordCount(shown.length, painted, overflow);
      badge(ctx, x + pad, railY - iconR, 16 * zoom, 12 * zoom, `+${hidden}`, zoom, "#1a2420", "#ccddee");
      return;
    }
    drawAbilityChip(ctx, x + pad + iconR, railY, iconR, shown[i]);
    railY += step;
    painted += 1;
  }
  // Include icons dropped by the rail floor, not only those past MAX_KEYWORD_BADGES.
  const hidden = hiddenKeywordCount(shown.length, painted, overflow);
  if (hidden > 0 && railY + iconR <= railFloor) {
    badge(ctx, x + pad, railY - iconR, 16 * zoom, 12 * zoom, `+${hidden}`, zoom, "#1a2420", "#ccddee");
  }
}

/** Dark circular chip with a Mana-font Arena ability glyph. */
function drawAbilityChip(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  id: string,
  bg = "#0c1412eb",
  ink = "#eafff0",
) {
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fillStyle = bg;
  ctx.fill();
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, r * 0.12);
  ctx.stroke();
  ctx.clip();

  const glyph = abilityGlyph(id);
  const wardN = id.startsWith("ward:") ? id.slice("ward:".length) : null;
  ctx.fillStyle = ink;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  if (glyph) {
    // Slightly smaller when a ward cost digit shares the chip.
    const size = Math.round(r * (wardN ? 1.05 : 1.35));
    ctx.font = `${size}px Mana`;
    ctx.fillText(glyph, cx, wardN ? cy - r * 0.15 : cy);
    if (wardN) {
      ctx.font = `bold ${Math.round(r * 0.7)}px system-ui, sans-serif`;
      ctx.fillText(wardN, cx, cy + r * 0.45);
    }
  } else {
    ctx.font = `bold ${Math.round(r * 1.1)}px system-ui, sans-serif`;
    ctx.fillText("?", cx, cy);
  }
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.restore();
}

function dot(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, color: string) {
  ctx.save();
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fillStyle = color;
  ctx.fill();
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = 1;
  ctx.stroke();
  ctx.restore();
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
}
