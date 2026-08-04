import { ZONE } from "../../board/geometry/layout";
import type { ObjectView } from "../wire/types";
import { ASSET_H, ASSET_W, type FrameKey } from "./assets";

export type FaceVariant = "permanent" | "full" | "stack";

/**
 * Everything the renderer draws about one card, read off the board's own `ObjectView`.
 *
 * The frame is deliberately not per-printing. Every print draws in the modern frame, the way Arena
 * does — the printing shows through the **art**, which the CDN already serves by print id, and
 * through the **flavor**, which the printing writes. No mana cost: the pip tray under the card owns
 * cost, so the face never draws one.
 *
 * `typeLine`, `oracle` and `flavor` come from `CatalogCard` over the catalog RPC, not from a new wire field —
 * `faceDataFrom` leaves them `""` and the caller folds them in (see `card-text.ts`). Only the
 * `full`/`stack` variants draw them; the square permanent has no room and shows none.
 */
export type FaceData = {
  /** Scryfall print id — the art key on the card CDN, and part of the face cache key. */
  print: string;
  name: string;
  /**
   * The object's colors (CR 105.2) as WUBRG indices (`engine::Color::index`), straight off
   * `ObjectView.colors`. The engine already folded in devoid, hybrid pips, a token's stated color,
   * and color-setting effects, so this is the frame's colour with no further interpretation.
   */
  colors: readonly number[];
  isLand: boolean;
  isToken: boolean;
  legendary: boolean;
  /** The battlefield's live P/T; printed P/T anywhere else. `""` when the card has none. */
  power: string;
  toughness: string;
  /** A planeswalker's loyalty or a battle's defence — one badge slot, same corner. */
  loyalty: string;
  /** `""` until the catalog lookup lands; the `permanent` variant draws neither in any case. */
  typeLine: string;
  oracle: string;
  /** Printed flavor text, set in italics under the rules divider. `""` when the printing has none. */
  flavor: string;
};

/**
 * The P/T-corner numbers. Live values only exist on the battlefield: `Game::pt_base` bails on a
 * non-permanent, so the projection reports 0 for a creature in hand or on the stack, and the
 * printed numbers on `view.kind` are the only real ones there. Same fallback the board's P/T badge
 * uses (`board/geometry/layout.ts`).
 */
function badges(view: ObjectView): { power: string; toughness: string; loyalty: string } {
  const blank = { power: "", toughness: "", loyalty: "" };
  const live = view.zone === ZONE.Battlefield;
  const kind = view.kind;
  if (kind.kind === "creature") {
    const power = live ? view.power : kind.power;
    const toughness = live ? view.toughness : kind.toughness;
    return { power: String(power), toughness: String(toughness), loyalty: "" };
  }
  if (kind.kind === "planeswalker") {
    return { ...blank, loyalty: String(live ? (view.loyalty ?? kind.loyalty) : kind.loyalty) };
  }
  // A battle's live defence counters ride on `loyalty` too, and print in the same corner.
  if (kind.kind === "battle") {
    return { ...blank, loyalty: String(live ? (view.loyalty ?? kind.defense) : kind.defense) };
  }
  return blank;
}

/** A face with nothing to draw — the library placeholder, and a base for test fixtures. */
export const BLANK_FACE: FaceData = {
  print: "",
  name: "",
  colors: [],
  isLand: false,
  isToken: false,
  legendary: false,
  power: "",
  toughness: "",
  loyalty: "",
  typeLine: "",
  oracle: "",
  flavor: "",
};

/** Read the renderer's inputs off a board object. */
export function faceDataFrom(view: ObjectView): FaceData {
  return {
    print: view.print ?? "",
    name: view.name,
    colors: view.colors ?? [],
    isLand: view.kind.kind === "land",
    isToken: view.is_token,
    legendary: view.legendary,
    ...badges(view),
    typeLine: "",
    oracle: "",
    flavor: "",
  };
}

/** A rectangle in whichever space its holder names — asset pixels or canonical face pixels. */
export type Rect = { x: number; y: number; w: number; h: number };

/**
 * A piece of a frame asset: `src` in asset pixels (750x1050), `dst` in canonical face pixels.
 * `turn` lays the piece on its side inside `dst` — a quarter turn counterclockwise about the
 * destination's centre, which is how a vertical rail becomes a horizontal one.
 */
export type Blit = { src: Rect; dst: Rect; turn?: "ccw" };

/** Which pieces a variant draws. A null slot is one this variant leaves off the face. */
export type SlotRects = {
  /** Frame pieces, drawn over the art in order. Empty when the variant draws no frame (a token). */
  frame: Blit[];
  /** Where the printing's art goes. Drawn first, under `frame`. */
  art: Rect;
  /** The legend crown, from the crown asset. Null when the card is not legendary. */
  crown: Blit | null;
  /** The P/T plate, from the P/T asset. Null when the variant or the card has none. */
  ptPlate: Blit | null;
  /** Text rects. Null means this variant does not draw that text. */
  title: Rect | null;
  type: Rect | null;
  text: Rect | null;
  pt: Rect | null;
};

/**
 * The size each variant renders at before it is scaled down at paint time. `full`/`stack` use the
 * printed card's proportions at Scryfall `normal` size; `permanent` is the Arena-style square.
 */
export const CANONICAL: Record<FaceVariant, { w: number; h: number }> = {
  permanent: { w: 745, h: 745 },
  full: { w: 745, h: 1040 },
  stack: { w: 745, h: 1040 },
};

/*
 * Where each printed element sits on the vendored 750x1050 asset canvas.
 *
 * `ART_WINDOW` (the transparent hole every frame leaves for the art), `PT_PLATE` and the crown's
 * `TOP_STRIP` bound are alpha-measured off the art itself — `magick <asset> -alpha extract
 * -threshold 50% -format "%@" info:` — and must not be nudged.
 *
 * `TITLE_BAR` and `TYPE_BAR` are where those two lines *sit*, not the plates they sit on: the frame
 * is opaque there, so alpha cannot segment a plate, and the renderer only ever reads their width (to
 * shrink an overlong line) and their vertical centre. Both are calibrated against Scryfall's png for
 * an M15 printing with `client/scripts/card-render-diff.mjs` — a printed name inks rows 64..92 of a
 * 1040-tall face, a printed type line rows 599..623 — so nudge them only against that measurement.
 *
 * `TEXT_BOX` is the printed paper, read off a Scryfall 744x1040 png by luminance: the pale box runs
 * y 655..957 there, which is these numbers once scaled to the asset. Getting the bottom right is
 * what makes plain vertical centring land where a real card's text lands — on both a one-line card
 * and a rules-plus-flavour one, the printed block centres on this box to within a pixel. The P/T
 * plate overlaps its bottom corner, exactly as it does in print.
 */
const ART_WINDOW: Rect = { x: 58, y: 119, w: 634, h: 463 };
const TITLE_BAR: Rect = { x: 58, y: 49, w: 634, h: 66 };
const TYPE_BAR: Rect = { x: 58, y: 589, w: 634, h: 61 };
const TEXT_BOX: Rect = { x: 58, y: 661, w: 633, h: 305 };
const PT_PLATE: Rect = { x: 579, y: 932, w: 130, h: 64 };
/**
 * The printed black card border, measured off the asset's own top and left edges. The square crops
 * it away: `board/bitmap/paint-cards.ts` clips the tile to a rounded rect and strokes its outline,
 * so a printed square border inside that reads as a second edge that misses the corners. Cropped,
 * the face is the card's colour to its own edge and the tile's outline is the only black rim.
 */
const CARD_BORDER = 30;
/** Card top down through the crown's bottom edge (measured at y+h = 195), inside the border. */
const TOP_STRIP: Rect = {
  x: CARD_BORDER,
  y: CARD_BORDER,
  w: ASSET_W - 2 * CARD_BORDER,
  h: 195 - CARD_BORDER,
};
/**
 * The square's remaining three edges. `SIDE_W` is the gap from the card edge to the art window, so
 * a rail is what is left of it once the printed border is cropped off — the same colour the top
 * strip shows at its own left and right, so the sides continue it without a seam.
 *
 * M15 prints no coloured band along the bottom of a card: under the text box it is the black
 * collector border, which on a dark board reads as no border at all. The bottom edge is therefore
 * the same side rail laid on its side. Its texture stretches along the card's width; at the size a
 * permanent paints, it reads as the border the sides already draw.
 */
const SIDE_W = ART_WINDOW.x;
const rail = (x: number): Rect => ({ x, y: ART_WINDOW.y, w: SIDE_W - CARD_BORDER, h: ART_WINDOW.h });
const WHOLE_ASSET: Rect = { x: 0, y: 0, w: ASSET_W, h: ASSET_H };

/** WUBRG index (`engine::Color::index`) → frame asset key. */
const COLOR_FRAMES: readonly FrameKey[] = ["w", "u", "b", "r", "g"];

function scale(rect: Rect, sx: number, sy: number): Rect {
  return { x: rect.x * sx, y: rect.y * sy, w: rect.w * sx, h: rect.h * sy };
}

/**
 * The frame a card draws in. Lands always take the land frame; two or more colours take gold;
 * one colour takes that colour; anything else is colourless.
 */
export function frameKey(face: FaceData): FrameKey {
  if (face.isLand) return "land";
  if (face.colors.length > 1) return "m";
  const [only] = face.colors;
  return only == null ? "c" : (COLOR_FRAMES[only] ?? "c");
}

function hasPT(face: FaceData): boolean {
  return face.power !== "" || face.toughness !== "" || face.loyalty !== "";
}

/**
 * Where the square draws its printed P/T plate, as a fraction of the face — null when the card
 * prints none. The plate hugs the bottom-right corner at the same inset the printed one keeps from
 * the card's right edge, so it overlaps the border rail the way print does.
 *
 * `board/bitmap/paint-cards.ts` writes the live numbers — which track counters and damage without
 * redrawing the face — over this plate, so the rect lives here rather than in either drawer.
 */
export function squarePtPlate(face: FaceData): Rect | null {
  // Same test as `faceAssetUrls`: only a creature has a plate to blit, and no land prints one.
  if (face.isToken || frameKey(face) === "land") return null;
  if (face.power === "" && face.toughness === "") return null;
  const span = ASSET_W - 2 * CARD_BORDER;
  const inset = ASSET_W - CARD_BORDER - (PT_PLATE.x + PT_PLATE.w);
  return {
    x: (span - inset - PT_PLATE.w) / span,
    y: (span - inset - PT_PLATE.h) / span,
    w: PT_PLATE.w / span,
    h: PT_PLATE.h / span,
  };
}

/**
 * The Arena square: art edge to edge, ringed by the frame's own border.
 *
 * Blitting the whole 750x1050 frame into a 745x745 square would crush it to 71% of its own height,
 * so the border is assembled edge by edge instead — the top strip scaled by width in both axes to
 * keep the art's aspect, then the two sides and the bottom stretched to close the ring. Below the
 * title bar the top strip is transparent, so the art shows through and the same strip serves the
 * legendary and nonlegendary cases alike — the crown is that same region of the crown asset.
 */
function squareSlots(w: number, h: number, face: FaceData): SlotRects {
  const art: Rect = { x: 0, y: 0, w, h };
  // A token draws art alone — no frame, no name. Nothing else on the battlefield is borderless, so
  // the absence is the tell, and the art is bigger for it.
  if (face.isToken) {
    return { frame: [], art, crown: null, ptPlate: null, title: null, type: null, text: null, pt: null };
  }
  // Scaled by the asset's width inside its printed border, since that is what the square draws.
  const s = w / (ASSET_W - 2 * CARD_BORDER);
  const strip: Blit = { src: TOP_STRIP, dst: { x: 0, y: 0, w, h: TOP_STRIP.h * s } };
  const side = rail(0).w * s;
  const flank = { y: strip.dst.h, w: side, h: h - strip.dst.h - side };
  const plate = squarePtPlate(face);
  return {
    frame: [
      strip,
      { src: rail(CARD_BORDER), dst: { x: 0, ...flank } },
      { src: rail(ASSET_W - SIDE_W), dst: { x: w - side, ...flank } },
      { src: rail(CARD_BORDER), dst: { x: 0, y: h - side, w, h: side }, turn: "ccw" },
    ],
    art,
    crown: face.legendary ? { src: TOP_STRIP, dst: strip.dst } : null,
    // The plate art only — the numbers on it are live, so `paint-cards.ts` writes them each frame.
    ptPlate:
      plate == null ? null : { src: PT_PLATE, dst: { x: plate.x * w, y: plate.y * h, w: plate.w * w, h: plate.h * h } },
    // The face's origin is the asset's border corner, so a printed rect shifts by it before scaling.
    title: scale({ ...TITLE_BAR, x: TITLE_BAR.x - CARD_BORDER, y: TITLE_BAR.y - CARD_BORDER }, s, s),
    type: null,
    text: null,
    pt: null,
  };
}

/**
 * Where each slot lands on the canonical face. `permanent` shows a title over full-bleed art —
 * card inspect is the read-the-card surface — while `full`/`stack` draw the whole printed frame.
 */
export function slotRects(variant: FaceVariant, face: FaceData): SlotRects {
  const { w, h } = CANONICAL[variant];
  if (variant === "permanent") return squareSlots(w, h, face);

  const sx = w / ASSET_W;
  const sy = h / ASSET_H;
  const whole: Rect = { x: 0, y: 0, w, h };
  return {
    frame: [{ src: WHOLE_ASSET, dst: whole }],
    art: scale(ART_WINDOW, sx, sy),
    crown: face.legendary ? { src: WHOLE_ASSET, dst: whole } : null,
    ptPlate: hasPT(face) ? { src: PT_PLATE, dst: scale(PT_PLATE, sx, sy) } : null,
    title: scale(TITLE_BAR, sx, sy),
    type: scale(TYPE_BAR, sx, sy),
    text: scale(TEXT_BOX, sx, sy),
    pt: hasPT(face) ? scale(PT_PLATE, sx, sy) : null,
  };
}
