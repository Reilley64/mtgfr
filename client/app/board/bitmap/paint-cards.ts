import { cardBackUrl, imageUrlByPrint } from "../../../lib/deck-builder/scryfall";
import type { ImageCache } from "../../../lib/image-cache";
import { CARD_RESTING_OUTLINE, COMMANDER_GOLD } from "../chrome";
import { type Camera, worldToScreen } from "../geometry/camera";
import { type RenderCard, seatColor, ZONE } from "../geometry/layout";

export interface Stroke {
  color: string;
  dash: number[];
}

export const CARD_OUTLINE = CARD_RESTING_OUTLINE;
export const DIM_CARD_VEIL = 0.45;
export const TAP_GLYPH = "\ue61a";
export const TARGET_STROKE: Stroke = { color: "#77CCFF", dash: [2, 6] };
export { TARGET_COLOR } from "../action/targeting";

const ABILITY_GLYPH: Record<string, string> = {
  deathtouch: "\ue94b",
  defender: "\ue94c",
  double_strike: "\ue94d",
  first_strike: "\ue950",
  flash: "\ue951",
  flying: "\ue952",
  goaded: "\ue9c9",
  haste: "\ue953",
  hexproof: "\ue954",
  indestructible: "\ue95a",
  lifelink: "\uea4b",
  menace: "\ue95d",
  "protection:black": "\uea7f",
  "protection:blue": "\uea80",
  "protection:green": "\uea81",
  "protection:red": "\uea82",
  "protection:white": "\uea83",
  prowess: "\ue982",
  reach: "\ue960",
  shroud: "\uea88",
  summoning_sick: "\ue96a",
  trample: "\ue964",
  unblockable: "\uea5c",
  vigilance: "\ue968",
  ward: "\ue992",
};

const BADGE_KEYWORDS = [
  "flying",
  "first_strike",
  "double_strike",
  "vigilance",
  "haste",
  "trample",
  "deathtouch",
  "lifelink",
  "menace",
  "reach",
  "defender",
  "unblockable",
  "indestructible",
  "hexproof",
  "shroud",
  "flash",
  "prowess",
] as const;

const MAX_KEYWORD_BADGES = 4;

export type CardPaintOptions = {
  outline?: Stroke | null;
  glow?: string | null;
  dim?: boolean;
  autoTapPreview?: boolean;
};

export type BitmapImageCache = Pick<ImageCache, "get">;

export function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
}

export function paintCard(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  cache: BitmapImageCache,
  viewer: number,
  options: CardPaintOptions = {},
): void {
  const tl = worldToScreen(cam, card.x, card.y);
  const w = card.w * cam.zoom;
  const h = card.h * cam.zoom;
  const r = 6 * cam.zoom;
  const outline = options.outline ?? null;

  ctx.save();
  rotateCard(ctx, card, viewer, tl.x, tl.y, w, h);

  if (options.glow) {
    ctx.shadowColor = options.glow;
    ctx.shadowBlur = 18 * cam.zoom;
  }

  roundRect(ctx, tl.x, tl.y, w, h, r);
  ctx.fillStyle = card.faceDown ? "#2a3742" : "#e8e4d8";
  ctx.fill();
  ctx.shadowBlur = 0;
  const baseStroke = card.isCommander ? COMMANDER_GOLD : (outline?.color ?? CARD_OUTLINE);
  ctx.strokeStyle = baseStroke;
  ctx.lineWidth = Math.max(1, (outline || card.isCommander ? 3 : 2) * cam.zoom);
  ctx.setLineDash(card.isCommander ? [] : (outline?.dash ?? []));
  ctx.stroke();
  ctx.setLineDash([]);

  if (card.faceDown) {
    paintFaceDown(ctx, cam, card, cache, tl.x, tl.y, w, h, r);
  } else {
    paintFaceUp(ctx, cam, card, cache, tl.x, tl.y, w, h, r);
  }

  if (card.isCommander && outline != null) {
    roundRect(ctx, tl.x, tl.y, w, h, r);
    ctx.strokeStyle = outline.color;
    ctx.lineWidth = Math.max(1, 2 * cam.zoom);
    ctx.setLineDash(outline.dash);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  if (options.dim) {
    roundRect(ctx, tl.x, tl.y, w, h, r);
    ctx.fillStyle = `rgba(0,0,0,${DIM_CARD_VEIL})`;
    ctx.fill();
  }

  if (options.autoTapPreview) drawAutoTapGlyph(ctx, tl.x, tl.y, w, h, cam.zoom);
  ctx.restore();
}

export function paintCardArt(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  cache: BitmapImageCache,
  viewer: number,
): void {
  const tl = worldToScreen(cam, card.x, card.y);
  const w = card.w * cam.zoom;
  const h = card.h * cam.zoom;
  const r = 6 * cam.zoom;

  ctx.save();
  rotateCard(ctx, card, viewer, tl.x, tl.y, w, h);

  const url = card.faceDown ? cardBackUrl() : imageUrlByPrint(card.print);
  const image = cache.get(url);
  if (image) {
    roundRect(ctx, tl.x, tl.y, w, h, r);
    ctx.clip();
    ctx.drawImage(image, tl.x, tl.y, w, h);
    ctx.restore();
    return;
  }

  roundRect(ctx, tl.x, tl.y, w, h, r);
  ctx.fillStyle = card.faceDown ? "rgba(42,55,66,0.78)" : "rgba(232,228,216,0.72)";
  ctx.fill();
  ctx.fillStyle = card.faceDown ? "#eff" : CARD_OUTLINE;
  ctx.font = `${Math.round(9 * cam.zoom)}px system-ui, sans-serif`;
  wrapText(ctx, card.name, tl.x + 6 * cam.zoom, tl.y + 16 * cam.zoom, w - 12 * cam.zoom, 11 * cam.zoom);
  ctx.restore();
}

export function paintAutoTapPreview(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  viewer: number,
): void {
  const tl = worldToScreen(cam, card.x, card.y);
  const w = card.w * cam.zoom;
  const h = card.h * cam.zoom;
  ctx.save();
  rotateCard(ctx, card, viewer, tl.x, tl.y, w, h);
  drawAutoTapGlyph(ctx, tl.x, tl.y, w, h, cam.zoom);
  ctx.restore();
}

export function paintCardTargetHighlight(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  viewer: number,
): void {
  const tl = worldToScreen(cam, card.x, card.y);
  const w = card.w * cam.zoom;
  const h = card.h * cam.zoom;
  const r = 6 * cam.zoom;

  ctx.save();
  rotateCard(ctx, card, viewer, tl.x, tl.y, w, h);

  ctx.shadowColor = TARGET_STROKE.color;
  ctx.shadowBlur = 18 * cam.zoom;
  roundRect(ctx, tl.x, tl.y, w, h, r);
  ctx.strokeStyle = TARGET_STROKE.color;
  ctx.lineWidth = Math.max(1, 3 * cam.zoom);
  ctx.setLineDash(TARGET_STROKE.dash.map((n) => n * cam.zoom));
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.shadowBlur = 0;

  ctx.restore();
}

function rotateCard(
  ctx: CanvasRenderingContext2D,
  card: RenderCard,
  viewer: number,
  x: number,
  y: number,
  w: number,
  h: number,
): void {
  const tapFrac = card.tapFrac ?? (card.tapped ? 1 : 0);
  let angle = card.controller !== viewer ? Math.PI : 0;
  angle += card.fanAngle ?? 0;
  angle += tapFrac * (Math.PI / 2);
  if (angle === 0) return;

  ctx.translate(x + w / 2, y + h / 2);
  ctx.rotate(angle);
  ctx.translate(-(x + w / 2), -(y + h / 2));
}

function paintFaceDown(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  cache: BitmapImageCache,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  const back = cache.get(cardBackUrl());
  if (back) {
    ctx.save();
    roundRect(ctx, x, y, w, h, r);
    ctx.clip();
    ctx.drawImage(back, x, y, w, h);
    ctx.restore();
  }

  if (card.pile <= 0) return;
  badge(
    ctx,
    x + w / 2 - 14 * cam.zoom,
    y + h / 2 - 9 * cam.zoom,
    28 * cam.zoom,
    18 * cam.zoom,
    `${card.pile}`,
    cam.zoom,
    CARD_OUTLINE,
    "#eff",
  );
}

function paintFaceUp(
  ctx: CanvasRenderingContext2D,
  cam: Camera,
  card: RenderCard,
  cache: BitmapImageCache,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  const img = cache.get(imageUrlByPrint(card.print));
  if (img) {
    ctx.save();
    roundRect(ctx, x, y, w, h, r);
    ctx.clip();
    ctx.drawImage(img, x, y, w, h);
    ctx.restore();
  } else {
    ctx.fillStyle = CARD_OUTLINE;
    ctx.font = `${Math.round(9 * cam.zoom)}px system-ui, sans-serif`;
    wrapText(ctx, card.name, x + 6 * cam.zoom, y + 16 * cam.zoom, w - 12 * cam.zoom, 11 * cam.zoom);
  }

  if (card.pile > 0) {
    badge(
      ctx,
      x + w / 2 - 14 * cam.zoom,
      y + h / 2 - 9 * cam.zoom,
      28 * cam.zoom,
      18 * cam.zoom,
      `x${card.pile}`,
      cam.zoom,
      CARD_OUTLINE,
      "#eff",
    );
  }
  if (card.cluster > 1) {
    badge(
      ctx,
      x + w - 28 * cam.zoom,
      y + 4 * cam.zoom,
      24 * cam.zoom,
      16 * cam.zoom,
      `${card.cluster}`,
      cam.zoom,
      "#1a1a1a",
      "#f4efe2",
    );
  }

  drawStatusBadges(ctx, x, y, w, cam.zoom, card);
  if (card.pt) {
    badge(
      ctx,
      x + w - 30 * cam.zoom,
      y + h - 20 * cam.zoom,
      26 * cam.zoom,
      15 * cam.zoom,
      card.pt,
      cam.zoom,
      "#f4efe2",
      "#111",
    );
  }
  if (card.counters > 0) {
    badge(
      ctx,
      x + 4 * cam.zoom,
      y + h - 20 * cam.zoom,
      24 * cam.zoom,
      15 * cam.zoom,
      `+${card.counters}`,
      cam.zoom,
      "#2f7d46",
      "#eafff0",
    );
  }
  if (card.markedDamage > 0) {
    badge(
      ctx,
      x + w / 2 - 12 * cam.zoom,
      y + h - 20 * cam.zoom,
      24 * cam.zoom,
      15 * cam.zoom,
      `${card.markedDamage}`,
      cam.zoom,
      "#8f2f2f",
      "#ffecec",
    );
  }
}

function drawAutoTapGlyph(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  zoom: number,
): void {
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

function wrapText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxW: number,
  lineH: number,
): void {
  const words = text.split(" ");
  let line = "";
  let yy = y;
  for (const word of words) {
    const test = line ? `${line} ${word}` : word;
    if (ctx.measureText(test).width > maxW && line) {
      ctx.fillText(line, x, yy);
      line = word;
      yy += lineH;
      continue;
    }
    line = test;
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
): void {
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

function drawStatusBadges(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  zoom: number,
  card: RenderCard,
): void {
  const pad = 5 * zoom;
  const iconR = 7 * zoom;
  let railY = y + pad + iconR;

  if (card.summoningSick && !card.hasHaste) {
    drawAbilityChip(ctx, x + pad + iconR, y + pad + iconR, iconR, "summoning_sick", "#e8b24a", "#1a1208");
    railY = y + pad + iconR * 2 + 3 * zoom + iconR;
  }
  if (card.goaded) {
    drawAbilityChip(ctx, x + pad + iconR, railY, iconR, "goaded", "#7a3b13", "#ffecec");
    railY += iconR * 2 + 3 * zoom;
  }
  if (card.prepared) {
    badge(ctx, x + w - pad - 14 * zoom, y + pad, 14 * zoom, 12 * zoom, "P", zoom, "#55cc99", "#0c1412");
  }
  if (card.isCommander) {
    dot(ctx, x + w - pad - 4 * zoom, y + pad + 4 * zoom + (card.prepared ? 14 * zoom : 0), 4 * zoom, COMMANDER_GOLD);
  }
  if (card.zone === ZONE.Battlefield && card.owner !== card.controller) {
    ctx.fillStyle = seatColor(card.owner, 0.95);
    roundRect(ctx, x + zoom, y + pad, 4 * zoom, card.h * zoom - 2 * pad, 2 * zoom);
    ctx.fill();
  }
  if (card.zone !== ZONE.Battlefield) return;

  const { shown, overflow } = keywordBadges(card.keywords);
  const railFloor = y + card.h * zoom - 22 * zoom;
  const step = iconR * 2 + 2 * zoom;
  let painted = 0;
  for (let i = 0; i < shown.length; i++) {
    if (railY + iconR > railFloor) break;
    const stillHiddenAfter = shown.length - i - 1 + overflow;
    if (stillHiddenAfter > 0 && railY + step + iconR > railFloor) {
      badge(
        ctx,
        x + pad,
        railY - iconR,
        16 * zoom,
        12 * zoom,
        `+${hiddenKeywordCount(shown.length, painted, overflow)}`,
        zoom,
        "#1a2420",
        "#ccddee",
      );
      return;
    }
    drawAbilityChip(ctx, x + pad + iconR, railY, iconR, shown[i]);
    railY += step;
    painted += 1;
  }
  const hidden = hiddenKeywordCount(shown.length, painted, overflow);
  if (hidden > 0 && railY + iconR <= railFloor) {
    badge(ctx, x + pad, railY - iconR, 16 * zoom, 12 * zoom, `+${hidden}`, zoom, "#1a2420", "#ccddee");
  }
}

function keywordBadges(keywords: readonly string[]): { shown: string[]; overflow: number } {
  const present = new Set(keywords);
  const ordered: string[] = [];
  for (const id of BADGE_KEYWORDS) {
    if (!present.has(id)) continue;
    ordered.push(id);
  }
  for (const raw of keywords) {
    if (ordered.includes(raw)) continue;
    if (raw.startsWith("ward:") || raw.startsWith("protection:")) ordered.push(raw);
  }
  if (ordered.length <= MAX_KEYWORD_BADGES) return { shown: ordered, overflow: 0 };
  return { shown: ordered.slice(0, MAX_KEYWORD_BADGES), overflow: ordered.length - MAX_KEYWORD_BADGES };
}

function hiddenKeywordCount(shownLen: number, painted: number, overflow: number): number {
  return overflow + Math.max(0, shownLen - painted);
}

function drawAbilityChip(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  id: string,
  bg = "#0c1412eb",
  ink = "#eafff0",
): void {
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fillStyle = bg;
  ctx.fill();
  ctx.strokeStyle = CARD_OUTLINE;
  ctx.lineWidth = Math.max(1, r * 0.12);
  ctx.stroke();
  ctx.clip();

  const glyph = id.startsWith("ward:") ? ABILITY_GLYPH.ward : (ABILITY_GLYPH[id] ?? null);
  const wardN = id.startsWith("ward:") ? id.slice("ward:".length) : null;
  ctx.fillStyle = ink;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  if (glyph) {
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

function dot(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, color: string): void {
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
