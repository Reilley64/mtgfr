// Card faces, flight cards, and status badges on the battlefield canvas.

import { CARD_H, CARD_W, type RenderCard, seatColor, ZONE } from "~/layout";
import { CARD_OUTLINE, DIM_CARD_VEIL, roundRect, type Stroke } from "~/lib/boardPaintPrims";
import { type Camera, worldToScreen } from "~/lib/camera";
import {
  abilityGlyph,
  foreignOwnerSeat,
  hiddenKeywordCount,
  keywordBadges,
  showsSummoningSick,
  TAP_GLYPH,
} from "~/lib/cardBadges";
import type { CardFlight } from "~/lib/cardFlight";
import type { ImageCache } from "~/lib/imageCache";
import { cardBackUrl, imageUrlByPrint } from "~/lib/scryfall";

export function drawFlightCard(ctx: CanvasRenderingContext2D, cam: Camera, flight: CardFlight, cache: ImageCache) {
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

export function drawCard(
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

  // Owner badge: a permanent controlled by someone other than its owner (donated / stolen /
  // exchanged, CR 108.3) renders in its controller's row — a bar down its left edge in the owner's
  // seat colour marks whose card it really is. ponytail: encodes the owner by seat colour only, no
  // player name; upgrade to a labelled chip if colour alone proves ambiguous at a crowded table.
  if (card.zone === ZONE.Battlefield) {
    const ownerSeat = foreignOwnerSeat(card.owner, card.controller);
    if (ownerSeat != null) {
      ctx.fillStyle = seatColor(ownerSeat, 0.95);
      roundRect(ctx, x + zoom, y + pad, 4 * zoom, card.h * zoom - 2 * pad, 2 * zoom);
      ctx.fill();
    }
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
