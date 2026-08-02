import type { ObjectView } from "../wire/types";
import type { FrameKey } from "./assets";

export type FaceVariant = "permanent" | "full" | "stack";

/**
 * Everything the renderer draws about one card, read off the board's own `ObjectView`.
 *
 * Deliberately not per-printing. Every print draws in the modern frame, the way Arena does — the
 * printing shows through the **art**, which the CDN already serves by print id. No mana cost: the
 * pip tray under the card owns cost, so the face never draws one.
 *
 * `typeLine` and `oracle` are `""` here and only the `full`/`stack` variants draw them. When those
 * slices land they fill from `CatalogCard` — which already carries `oracle` and `subtypes` over the
 * catalog RPC the board calls for inspect — not from a new wire field.
 */
export type FaceData = {
  /** Scryfall print id — the art key on the card CDN, and part of the face cache key. */
  print: string;
  name: string;
  /** WUBRG indices with at least one pip in the printed cost (see `engine::Color::index`). */
  colors: readonly number[];
  isLand: boolean;
  isToken: boolean;
  legendary: boolean;
  /** Printed as drawn — the battlefield's current power, not the printed one. `""` when absent. */
  power: string;
  toughness: string;
  loyalty: string;
  /** `""` on the `permanent` variant, which draws neither. Slice 3 fills them from `CatalogCard`. */
  typeLine: string;
  oracle: string;
};

/** Read the renderer's inputs off a board object. */
export function faceDataFrom(view: ObjectView): FaceData {
  const isCreature = view.kind.kind === "creature";
  const isPlaneswalker = view.kind.kind === "planeswalker";
  return {
    print: view.print ?? "",
    name: view.name,
    colors: view.mana_cost.colored.flatMap((count, index) => (count > 0 ? [index] : [])),
    isLand: view.kind.kind === "land",
    isToken: view.is_token,
    legendary: view.legendary,
    power: isCreature ? String(view.power) : "",
    toughness: isCreature ? String(view.toughness) : "",
    loyalty: isPlaneswalker ? String(view.loyalty ?? 0) : "",
    typeLine: "",
    oracle: "",
  };
}

export type Rect = { x: number; y: number; w: number; h: number };

/** Which slots a variant draws. A null slot is one this variant leaves off the face. */
export type SlotRects = {
  frame: Rect;
  art: Rect;
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

/** Fractions of the canonical face, measured off the M15 template. */
const MARGIN = 0.0455;
const TITLE_TOP = 0.0413;
const TITLE_H = 0.0625;
const ART_TOP = 0.1163;
const TYPE_H = 0.0577;
const PT_W = 0.1638;
const PT_H = 0.0625;

/** WUBRG index (`engine::Color::index`) → frame asset key. */
const COLOR_FRAMES: readonly FrameKey[] = ["w", "u", "b", "r", "g"];

function rect(w: number, h: number, x: number, y: number, rw: number, rh: number): Rect {
  return { x: x * w, y: y * h, w: rw * w, h: rh * h };
}

/**
 * The frame a card draws in. Lands always take the land frame; two or more colours take gold;
 * one colour takes that colour; anything else is colourless.
 *
 * ponytail: read off the printed cost's pips, not the card's colour indicator — a colourless card
 * with a coloured indicator draws in the wrong frame. Nothing in the pool has one today; read the
 * indicator here when one does.
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
 * Where each slot lands on the canonical face. `permanent` shows a title and art only — card
 * inspect is the read-the-card surface — and a token shows art alone, arched, with no title.
 */
export function slotRects(variant: FaceVariant, face: FaceData): SlotRects {
  const { w, h } = CANONICAL[variant];
  const frame = { x: 0, y: 0, w, h };
  const inner = 1 - 2 * MARGIN;
  const pt = hasPT(face) ? rect(w, h, 1 - MARGIN - PT_W, 1 - MARGIN - PT_H * 0.55, PT_W, PT_H) : null;

  if (variant === "permanent") {
    if (face.isToken) {
      return { frame, art: rect(w, h, MARGIN, MARGIN, inner, 1 - 2 * MARGIN), title: null, type: null, text: null, pt };
    }
    return {
      frame,
      art: rect(w, h, MARGIN, ART_TOP, inner, 1 - ART_TOP - MARGIN),
      title: rect(w, h, MARGIN, TITLE_TOP, inner, TITLE_H),
      type: null,
      text: null,
      pt,
    };
  }

  const artH = 0.4413 - ART_TOP;
  const typeTop = ART_TOP + artH + 0.012;
  const textTop = typeTop + TYPE_H + 0.008;
  return {
    frame,
    art: rect(w, h, MARGIN, ART_TOP, inner, artH),
    title: rect(w, h, MARGIN, TITLE_TOP, inner, TITLE_H),
    type: rect(w, h, MARGIN, typeTop, inner, TYPE_H),
    text: rect(w, h, MARGIN, textTop, inner, 1 - textTop - MARGIN - 0.03),
    pt,
  };
}
