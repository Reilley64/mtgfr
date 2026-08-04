import { BODY_FONT, frameAssetUrl, TITLE_FONT } from "./assets";
import { type Blit, type FaceData, type FaceVariant, frameKey, type Rect, slotRects } from "./frame";
import { drawManaSymbol } from "./symbols";
import { fitOracleSize, LINE_HEIGHT, type Measure, SYMBOL_EM, wrapOracle } from "./text";

/*
 * Printed type sizes, as a fraction of the slot each sits in. A real M15 card sets its name in about
 * 41px, its type line in 36px and its rules text in 35px at this asset's 750x1050 — those over the
 * slot heights in `frame.ts` are the numbers below. Rules text is a ceiling: a wordy card shrinks to
 * fit its box, the way a printed one does.
 */
const TITLE_SCALE = 0.62;
const TYPE_SCALE = 0.58;
const RULES_SCALE = 0.116;
const PT_SCALE = 0.62;

/** Printed ink — the near-black a card's text is set in, not pure black. */
const INK = "#17130d";
/**
 * Text-box padding, as a fraction of the box. Measured off a printed card: its rules text sits
 * about 12px in from a 628px-wide box, and centres vertically in the paper — so the vertical
 * number is only a floor for text too tall to centre, not a printed margin.
 */
const TEXT_PAD_X = 0.02;
const TEXT_PAD_Y = 0.03;

export type FaceInput = {
  face: FaceData;
  variant: FaceVariant;
  /** The printing's art crop from the card CDN; null until it loads — the frame still draws. */
  art: CanvasImageSource | null;
  frameImage: CanvasImageSource | null;
  ptImage: CanvasImageSource | null;
  crownImage: CanvasImageSource | null;
};

/** Which vendored assets a face needs, so the caller can warm them before drawing. */
export function faceAssetUrls(face: FaceData): { frame: string; pt: string | null; crown: string | null } {
  const key = frameKey(face);
  const creature = face.power !== "" || face.toughness !== "";
  return {
    frame: frameAssetUrl(`m15/${key}`),
    // ponytail: only creatures get a plate. The vendored set has no planeswalker loyalty badge, so
    // a planeswalker's number draws bare on the frame until one is vendored.
    // No land has a printed P/T box; a legendary land does get a crown (Gaea's Cradle).
    pt: creature && key !== "land" ? frameAssetUrl(`m15/pt/${key}`) : null,
    crown: face.legendary ? frameAssetUrl(`m15/crown/${key}`) : null,
  };
}

/**
 * Draws one card face at canonical size into `ctx`. Art first, frame pieces over it (every frame
 * asset leaves the art window transparent), then text. The mana cost is never drawn: the pip tray
 * under the card owns it.
 */
export function drawFace(ctx: CanvasRenderingContext2D, input: FaceInput): void {
  const { face, variant } = input;
  const slots = slotRects(variant, face);
  const measure: Measure = (piece, fontPx) => {
    if (piece.kind === "symbol") return fontPx * SYMBOL_EM;
    ctx.font = bodyFont(fontPx, piece.reminder);
    return ctx.measureText(piece.value).width;
  };

  ctx.save();
  if (input.art != null) drawArt(ctx, input.art, slots.art);
  if (input.frameImage != null) {
    for (const piece of slots.frame) blit(ctx, input.frameImage, piece);
  }
  if (input.crownImage != null && slots.crown != null) blit(ctx, input.crownImage, slots.crown);
  if (input.ptImage != null && slots.ptPlate != null) blit(ctx, input.ptImage, slots.ptPlate);

  ctx.fillStyle = INK;
  ctx.textBaseline = "middle";

  if (slots.title != null) drawFitted(ctx, face.name, slots.title, TITLE_FONT, slots.title.h * TITLE_SCALE);
  if (slots.type != null) drawFitted(ctx, face.typeLine, slots.type, TITLE_FONT, slots.type.h * TYPE_SCALE);
  if (slots.text != null) drawTextBox(ctx, face.oracle, slots.text, measure);
  if (slots.pt != null) drawPT(ctx, face, slots.pt);

  ctx.restore();
}

function blit(ctx: CanvasRenderingContext2D, image: CanvasImageSource, piece: Blit): void {
  const { src, dst } = piece;
  if (piece.turn == null) {
    ctx.drawImage(image, src.x, src.y, src.w, src.h, dst.x, dst.y, dst.w, dst.h);
    return;
  }
  // A quarter turn counterclockwise about the destination's centre. Inside the turned frame the
  // box is its own height by its own width, so a tall rail fills a wide band.
  ctx.save();
  ctx.translate(dst.x + dst.w / 2, dst.y + dst.h / 2);
  ctx.rotate(-Math.PI / 2);
  ctx.drawImage(image, src.x, src.y, src.w, src.h, -dst.h / 2, -dst.w / 2, dst.h, dst.w);
  ctx.restore();
}

/**
 * Art fills its window, cropped centre. Scryfall's `art_crop` is a wide rectangle and the Arena
 * square is 1:1, so stretching it to the window would visibly squash the artwork — take the
 * largest centred slice of the source that has the window's aspect instead.
 */
function drawArt(ctx: CanvasRenderingContext2D, art: CanvasImageSource, box: Rect): void {
  const sw = typeof art === "object" && "width" in art ? Number(art.width) : Number.NaN;
  const sh = typeof art === "object" && "height" in art ? Number(art.height) : Number.NaN;
  if (!Number.isFinite(sw) || !Number.isFinite(sh) || sw <= 0 || sh <= 0) {
    // A source with no intrinsic size (an SVG image) cannot be cropped; stretch and move on.
    ctx.drawImage(art, box.x, box.y, box.w, box.h);
    return;
  }
  const wanted = box.w / box.h;
  const cropW = sw / sh > wanted ? sh * wanted : sw;
  const cropH = sw / sh > wanted ? sh : sw / wanted;
  ctx.drawImage(art, (sw - cropW) / 2, (sh - cropH) / 2, cropW, cropH, box.x, box.y, box.w, box.h);
}

/** One line, shrunk to fit its slot's width, left-aligned like a printed title. */
function drawFitted(ctx: CanvasRenderingContext2D, text: string, box: Rect, font: string, maxPx: number): void {
  if (text === "") return;
  let size = maxPx;
  ctx.font = `${size}px ${font}, serif`;
  while (size > maxPx * 0.5 && ctx.measureText(text).width > box.w) {
    size -= 0.5;
    ctx.font = `${size}px ${font}, serif`;
  }
  ctx.textAlign = "left";
  ctx.fillText(text, box.x + box.w * 0.02, box.y + box.h / 2);
}

/** Body font for one run — reminder text prints italic, the way a printed card sets it. */
function bodyFont(fontPx: number, reminder: boolean): string {
  return `${reminder ? "italic " : ""}${fontPx}px ${BODY_FONT}, serif`;
}

/**
 * Rules text, inset from the printed box and centred in what is left — a short ability sits in the
 * middle of its box on a real card, not pinned to the top edge.
 */
function drawTextBox(ctx: CanvasRenderingContext2D, text: string, box: Rect, measure: Measure): void {
  if (text === "") return;
  const padX = box.w * TEXT_PAD_X;
  const padY = box.h * TEXT_PAD_Y;
  const inner = { w: box.w - 2 * padX, h: box.h - 2 * padY };
  const size = fitOracleSize(text, inner, box.h * RULES_SCALE, measure);
  const lines = wrapOracle(text, inner.w, size, measure);
  const step = size * LINE_HEIGHT;

  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  let y = box.y + Math.max(padY, (box.h - lines.length * step) / 2) + step / 2;
  for (const line of lines) {
    let x = box.x + padX;
    for (const piece of line) {
      if (piece.kind === "symbol") {
        drawManaSymbol(ctx, piece, x, y, size);
      } else {
        ctx.font = bodyFont(size, piece.reminder);
        ctx.fillStyle = INK;
        ctx.fillText(piece.value, x, y);
      }
      x += measure(piece, size);
    }
    y += step;
  }
}

function drawPT(ctx: CanvasRenderingContext2D, face: FaceData, box: Rect): void {
  const label = face.loyalty !== "" ? face.loyalty : `${face.power}/${face.toughness}`;
  if (label === "/") return;
  ctx.font = `${box.h * PT_SCALE}px ${TITLE_FONT}, serif`;
  ctx.textAlign = "center";
  ctx.fillText(label, box.x + box.w / 2, box.y + box.h / 2);
}
