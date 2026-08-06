// Mana pips drawn on canvas: a coloured disk with the mana-font glyph on it, the way CSS
// `.ms-cost` composes one from a background and a `::before`. The board's own pip tray already
// draws these in HTML; a card face has no DOM to hang them on, so it composes them here.

import { costPipPlate } from "../costPips";
import { MANA_GLYPH, MANA_GLYPH_AFTER } from "../mana-glyphs.generated";
import { SYMBOL_FONT } from "./assets";
import { SYMBOL_EM } from "./text";

/** Disk diameter as a multiple of the font size — a printed pip sits a touch under the cap height. */
const PIP_EM = 0.86;
/** The glyph inside the disk, as a fraction of the disk. Halves of a hybrid draw smaller, offset. */
const GLYPH_SCALE = 0.68;
const HALF_SCALE = 0.5;
const HALF_OFFSET = 0.2;
/** mana-font's `.ms-cost` ink. */
const PIP_INK = "#111";

const COLOURED = /^[wubrg]$/i;

/** A pip's disk colour: the code's coloured half (`{2/W}` is white), or the generic grey. */
function plate(code: string): string {
  const halves = code.split("/");
  return costPipPlate(halves.find((half) => COLOURED.test(half)) ?? code);
}

/**
 * Fills the disk. A two-colour hybrid prints as a diagonal split; everything else is one colour.
 *
 * ponytail: `{2/W}` and Phyrexian `{W/P}` print split too — grey/white and white/Phyrexian — and
 * draw solid here. Both halves of those are already legible from the glyph pair on top.
 */
function fillDisk(ctx: CanvasRenderingContext2D, code: string, cx: number, cy: number, r: number): void {
  const halves = code.split("/").filter((half) => COLOURED.test(half));
  if (halves.length < 2) {
    ctx.fillStyle = plate(code);
    return;
  }
  const [top, bottom] = halves.map((half) => costPipPlate(half));
  const split = ctx.createLinearGradient(cx - r, cy - r, cx + r, cy + r);
  split.addColorStop(0, top ?? "");
  split.addColorStop(0.5, top ?? "");
  split.addColorStop(0.5, bottom ?? "");
  split.addColorStop(1, bottom ?? "");
  ctx.fillStyle = split;
}

/**
 * Draws one mana symbol whose advance starts at `x`, centred on the line's middle at `midY`.
 * Returns nothing: the caller advances by `fontPx * SYMBOL_EM`, the same width it measured with.
 */
export function drawManaSymbol(
  ctx: CanvasRenderingContext2D,
  piece: { code: string; ms: string },
  x: number,
  midY: number,
  fontPx: number,
): void {
  const glyph = MANA_GLYPH[piece.ms];
  if (glyph == null) return;
  const r = (fontPx * PIP_EM) / 2;
  const cx = x + (fontPx * SYMBOL_EM) / 2;

  ctx.save();
  fillDisk(ctx, piece.code, cx, midY, r);
  ctx.beginPath();
  ctx.arc(cx, midY, r, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = PIP_INK;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const after = MANA_GLYPH_AFTER[piece.ms];
  if (after == null) {
    ctx.font = `${r * 2 * GLYPH_SCALE}px ${SYMBOL_FONT}`;
    ctx.fillText(glyph, cx, midY);
  } else {
    // A hybrid prints both halves in one disk — mana-font's `::before` up-left, `::after` down-right.
    ctx.font = `${r * 2 * HALF_SCALE}px ${SYMBOL_FONT}`;
    ctx.fillText(glyph, cx - r * HALF_OFFSET, midY - r * HALF_OFFSET);
    ctx.fillText(after, cx + r * HALF_OFFSET, midY + r * HALF_OFFSET);
  }
  ctx.restore();
}
