import { BODY_FONT, frameAssetUrl, TITLE_FONT } from "./assets";
import { type Blit, type FaceData, type FaceVariant, frameKey, type Rect, slotRects } from "./frame";
import { drawManaSymbol } from "./symbols";
import {
  blockHeight,
  cardTextBlock,
  fitCardText,
  hangIndent,
  LINE_HEIGHT,
  lineStep,
  type Measure,
  SYMBOL_EM,
  smartQuotes,
} from "./text";

/*
 * Printed type sizes, as a fraction of the slot each sits in. A real M15 card sets its name in about
 * 40px, its type line in 34px and its rules text in 35px on a 745x1040 face — those over the slot
 * heights in `frame.ts` are the numbers below. Rules text is a ceiling: a wordy card shrinks to fit
 * its box, the way a printed one does.
 *
 * These are set by width, not by eye: on Scryfall's png for Llanowar Elves (`fdn`) the name inks
 * 255px across, the type line 304px and a flavor line 560px, and `client/scripts/card-render-diff.mjs`
 * re-measures them.
 */
const TITLE_SCALE = 0.605;
const TYPE_SCALE = 0.5555;
/**
 * Rules text is set off the printed glyphs, not off the region score. Guard Gomazoa (`pca`) sets the
 * same first line as we do — `Defender, flying` — and prints it 263px wide and 34px tall against our
 * 246 and 32: print is about 6.5% larger, both measures agreeing. The region score cannot arbitrate
 * this and must not be asked to: mismatched glyphs cost area, so it rewards under-inking, and at
 * `0.08` — text far smaller than any printed card — every reference card scores *better* than here.
 */
const RULES_SCALE = 0.1225;
const PT_SCALE = 0.62;

/** Printed ink — the near-black a card's text is set in, not pure black. */
const INK = "#17130d";
/**
 * Text-box padding, as a fraction of the box. Measured off a printed card: its rules text inks from
 * about 8px in on a 628px-wide box, and centres vertically in the paper — so the vertical number is
 * only a floor for text too tall to centre, not a printed margin.
 */
const TEXT_PAD_X = 0.0125;
const TEXT_PAD_Y = 0.03;
/**
 * The title and type lines start much closer to their bar's edge than rules text does to the paper —
 * about 5px in on a printed M15 card, against the text box's 19.
 */
const BAR_PAD_X = 0.008;
/** The flavor divider's width, as a fraction of the text box — a printed one stops short of both edges. */
const DIVIDER_W = 0.9;
/**
 * How much narrower a true italic sets than its roman. Only the roman MPlantin is vendored, so the
 * browser slants it — and a slant is a shear, which keeps roman's advance to the pixel. Phyrexian
 * Arena (`c15`) shows what that costs: print inks its whole flavor 588px wide on one line, where the
 * same words measure 625 in our roman at the size print sets. So condense the slanted runs to what
 * print measures, and a flavor line lands on the line print gives it.
 *
 * ponytail: a real italic redraws letterforms, it does not squeeze roman ones — `a` and `g` are
 * different glyphs. Vendor `mplantin-italic.ttf` beside the roman and drop this.
 */
const ITALIC_SET = 0.92;

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
    return ctx.measureText(piece.value).width * (piece.reminder ? ITALIC_SET : 1);
  };

  ctx.save();
  if (input.art != null) drawArt(ctx, input.art, slots.art);
  if (input.frameImage != null) {
    for (const piece of slots.frame) blit(ctx, input.frameImage, piece);
  }
  if (input.crownImage != null && slots.crown != null) blit(ctx, input.crownImage, slots.crown);

  ctx.fillStyle = INK;
  ctx.textBaseline = "middle";

  if (slots.title != null)
    drawFitted(ctx, smartQuotes(face.name), slots.title, TITLE_FONT, slots.title.h * TITLE_SCALE);
  if (slots.type != null)
    drawFitted(ctx, smartQuotes(face.typeLine), slots.type, TITLE_FONT, slots.type.h * TYPE_SCALE);
  if (slots.text != null) drawTextBox(ctx, face, slots.text, measure);
  // The plate goes on last: a wordy card's text box runs under it, and print has the plate on top.
  if (input.ptImage != null && slots.ptPlate != null) blit(ctx, input.ptImage, slots.ptPlate);
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
  ctx.fillText(text, box.x + box.w * BAR_PAD_X, box.y + box.h / 2);
}

/** Body font for one run — reminder and flavor text print italic, the way a printed card sets them. */
function bodyFont(fontPx: number, reminder: boolean): string {
  return `${reminder ? "italic " : ""}${fontPx}px ${BODY_FONT}, serif`;
}

/** One run of body text, a slanted one condensed to the set width print gives it — see [`ITALIC_SET`]. */
function fillRun(ctx: CanvasRenderingContext2D, value: string, reminder: boolean, x: number, y: number): void {
  if (!reminder) {
    ctx.fillText(value, x, y);
    return;
  }
  ctx.save();
  ctx.scale(ITALIC_SET, 1);
  ctx.fillText(value, x / ITALIC_SET, y);
  ctx.restore();
}

/**
 * Rules text, inset from the printed box and centred in what is left — a short ability sits in the
 * middle of its box on a real card, not pinned to the top edge.
 */
function drawTextBox(ctx: CanvasRenderingContext2D, face: FaceData, box: Rect, measure: Measure): void {
  if (face.oracle === "" && face.flavor === "") return;
  const padX = box.w * TEXT_PAD_X;
  const padY = box.h * TEXT_PAD_Y;
  const inner = { w: box.w - 2 * padX, h: box.h - 2 * padY };
  const size = fitCardText(face.oracle, face.flavor, inner, box.h * RULES_SCALE, measure);
  const block = cardTextBlock(face.oracle, face.flavor, inner.w, size, measure);
  const { lines, divider } = block;

  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  let y = box.y + Math.max(padY, (box.h - blockHeight(block, size)) / 2) + (size * LINE_HEIGHT) / 2;
  for (const [index, line] of lines.entries()) {
    // Print sets extra air where one ability ends and the next begins.
    if (index > 0) y += lineStep(block, index, size);
    if (index === divider) drawDivider(ctx, box, y);
    let x = box.x + padX + (block.hangs.has(index) ? hangIndent(size, measure) : 0);
    for (const piece of line) {
      if (piece.kind === "symbol") {
        drawManaSymbol(ctx, piece, x, y, size);
      } else {
        ctx.font = bodyFont(size, piece.reminder);
        ctx.fillStyle = INK;
        fillRun(ctx, piece.value, piece.reminder, x, y);
      }
      x += measure(piece, size);
    }
  }
}

/**
 * The rule between rules text and flavor: a whisper, not a line. A printed M15 card separates the
 * two blocks with air — scanned row by row across seven printings at 1040 tall (Abrade `hou`, Black
 * Knight `30a`, Phyrexian Arena `c15`, Rhystic Study `pz1`, Sol Ring `vma`, Fungusaur `30a`, Guard
 * Gomazoa `pca`), no row between the blocks darkens more than about 3% below the paper, and what
 * little there is comes off the glyph edges around it. So the blank row does the separating, and
 * this only breathes on it.
 */
function drawDivider(ctx: CanvasRenderingContext2D, box: Rect, y: number): void {
  const w = box.w * DIVIDER_W;
  const x = box.x + (box.w - w) / 2;
  const ramp = ctx.createLinearGradient(0, y - 4, 0, y + 1);
  ramp.addColorStop(0, "rgba(23, 19, 13, 0)");
  ramp.addColorStop(0.75, "rgba(23, 19, 13, 0.02)");
  ramp.addColorStop(1, "rgba(23, 19, 13, 0.05)");
  ctx.fillStyle = ramp;
  ctx.fillRect(x, y - 4, w, 5);
  ctx.fillStyle = INK;
}

function drawPT(ctx: CanvasRenderingContext2D, face: FaceData, box: Rect): void {
  const label = face.loyalty !== "" ? face.loyalty : `${face.power}/${face.toughness}`;
  if (label === "/") return;
  ctx.font = `${box.h * PT_SCALE}px ${TITLE_FONT}, serif`;
  ctx.textAlign = "center";
  ctx.fillText(label, box.x + box.w / 2, box.y + box.h / 2);
}
