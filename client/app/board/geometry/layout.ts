// Pure layout: turn a per-viewer VisibleState into positioned render cards for the canvas,
// arranged as a Commander table. Each seat gets a three-row battlefield (Noncreature, Creatures,
// Lands — centerward → outer), a compact zone column on the left (top → bottom: commander, exile,
// deck, graveyard), a life-orb avatar on its outer edge, and a mana tray under the zone column
// outside the seat band. Seats sit around the table as a 2×2 quadrant — you at the bottom-left,
// and the other players in turn order at *front* (directly above you), *side* (beside you), then
// *diagonal* — matching where they'd physically sit. Fewer than four just leave later cells empty
// (2p → you + front, a vertical pair; 3p → drops the diagonal). Top-row seats are flipped to face
// down across the table; the two bottom seats are upright. One camera pans/zooms the whole table.
// The hand is a DOM overlay (components/molecules/hand.tsx); the mana tray is a world-anchored DOM overlay (ManaTray.tsx).

import type { ObjectView, VisibleState, WireKind } from "~/wire/types";
import { BLANK_FACE, type FaceData, faceDataFrom } from "../../domain/card-render/frame";
import { ZONE } from "../../domain/zones";
import { PERMANENT_SIDE, PERMANENT_STEP, type PermanentRowMetrics, permanentRowMetrics } from "./permanent-layout";

export { ZONE };

/** Step discriminants — must match `engine::Step`'s declaration order. */
export const STEP = {
  Untap: 0,
  Upkeep: 1,
  Draw: 2,
  Main1: 3,
  BeginCombat: 4,
  DeclareAttackers: 5,
  DeclareBlockers: 6,
  FirstStrikeCombatDamage: 7,
  CombatDamage: 8,
  EndCombat: 9,
  Main2: 10,
  End: 11,
  Cleanup: 12,
} as const;

export const STEP_NAMES = [
  "Untap",
  "Upkeep",
  "Draw",
  "Main 1",
  "Begin Combat",
  "Declare Attackers",
  "Declare Blockers",
  "First Strike Damage",
  "Combat Damage",
  "End Combat",
  "Main 2",
  "End",
  "Cleanup",
] as const;

/** MTGA-style phase bands: each of the 13 steps rolls up into one of these, shown as a
 * segmented track so the turn's shape reads at a glance. */
export const PHASES = [
  { name: "Beginning", steps: [STEP.Untap, STEP.Upkeep, STEP.Draw] },
  { name: "Main 1", steps: [STEP.Main1] },
  {
    name: "Combat",
    steps: [
      STEP.BeginCombat,
      STEP.DeclareAttackers,
      STEP.DeclareBlockers,
      STEP.FirstStrikeCombatDamage,
      STEP.CombatDamage,
      STEP.EndCombat,
    ],
  },
  { name: "Main 2", steps: [STEP.Main2] },
  { name: "End", steps: [STEP.End, STEP.Cleanup] },
] as const;

/** Index into `PHASES` for a step discriminant (-1 if unknown). */
export const phaseOf = (step: number): number => PHASES.findIndex((p) => (p.steps as readonly number[]).includes(step));

export type Kind = WireKind["kind"];

export interface RenderCard {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
  name: string;
  /** Card (oracle) id, when known — drives Alt-pin inspect's oracle-text lookup (accounts-decks-and-catalog spec). */
  cardId: string;
  /** Printing UUID for this object's art (accounts-decks-and-catalog spec); empty renders a broken image. */
  print: string;
  pt: string;
  tapped: boolean;
  counters: number;
  markedDamage: number;
  faceDown: boolean;
  zone: number;
  controller: number;
  owner: number;
  kind: Kind;
  /** Tapping this permanent makes mana — the board's tap-for-mana click (see `ObjectView`). */
  tapsForMana: boolean;
  summoningSick: boolean;
  hasHaste: boolean;
  /** Effective keywords (wire snake_case ids) for Arena-style ability badges. */
  keywords: string[];
  /** Goaded (CR 701.38) — Arena status chip. */
  goaded: boolean;
  /** Everything the card-face renderer draws. Built once here so paint stays a pure blit. */
  face: FaceData;
  isCommander: boolean;
  /** Prepare-DFC status — drives card-inspect play-face default. */
  prepared: boolean;
  /** >0 means this is a pile (graveyard/exile) standing in for `pile` cards. */
  pile: number;
  /** >1 means a permanent cluster (count badge); 0 when not clustered. Not a pile. */
  cluster: number;
  /** Member object ids when `cluster > 1` (lowest id is the face / `id`). */
  clusterMembers: number[];
  /** Host id when this card is an attached Aura/Equipment; null/undefined when free. */
  attachedTo?: number | null;
  /** Animated tap rotation 0 (upright) → 1 (tapped), filled in by the board's tween; absent in raw
   * layout (the draw falls back to the binary `tapped`). */
  tapFrac?: number;
}

/**
 * A resting permanent is an Arena-style square tile: name slot and art, no oracle text. Square so
 * three rows of seven fit four seats without the camera zooming out past readability.
 */
export const CARD_W = PERMANENT_SIDE;
export const CARD_H = PERMANENT_SIDE;

/**
 * A card in motion keeps the printed card's proportions — a flight paints the full face and the
 * square tile appears only when the card comes to rest, with no mid-air morph.
 */
export const FLIGHT_CARD_W = 96;
export const FLIGHT_CARD_H = 134;

/** Radius of a player's life-orb avatar, in world units (so it pans/zooms with the board). */
export const AVATAR_R = 40;
export const AVATAR_HAND_LABEL_OFFSET = -29;
export const AVATAR_LIFE_LABEL_BELOW = 48;
export const AVATAR_USERNAME_LABEL_BELOW = 66;
export const AVATAR_LABEL_BELOW = 80;

export type AvatarLabelOffsets = {
  hand: number;
  life: number;
  username: number;
  commander: number;
};

// Each seat: a battlefield of three rows (Noncreature → Creatures → Lands, centerward → outer)
// with a compact zone column on the left (deck / graveyard / exile / commander), a life orb on
// its outer edge, and a mana tray under the zone column outside the seat band. Empty rows still
// reserve height (stable seat silhouette).
// Sized for the 4-seat Commander footprint: every world unit of height/gutter costs zoom on every
// card, so gaps stay tight and avatars hang outside the inter-seat gutter (not inside it).
const GAP = 8;
const CARD_HSTEP = PERMANENT_STEP; // horizontal distance between card centers
const ROW_H = PERMANENT_STEP; // one battlefield row (tilted card extent + clearance)
const BATTLE_H = 3 * ROW_H;
// Avatars hang off the *outer* edge of each band (above the top row / below the bottom), not in
// the gutter between seats — so the inter-row gutter only needs a hair of separation.
const BAND_GAP = GAP;
const BAND_STRIDE = BATTLE_H + BAND_GAP; // vertical distance between the two table rows

// The left column's cards are rendered at half size so four stack alongside the three-row
// battlefield (4 × COL_STRIDE ≈ BATTLE_H). Top → bottom: commander, exile, deck, graveyard.
// A pile is a stack of cards, not a permanent, so it keeps the printed card's proportions — the
// Arena square is the battlefield's treatment alone.
const COL_W = FLIGHT_CARD_W * 0.5;
const COL_H = Math.round(FLIGHT_CARD_H * 0.5);
const COL_STRIDE = BATTLE_H / 4;
const COL_X = -(COL_W + 2 * GAP); // just left of the battlefield's first card (x = 0)

// Horizontal grid: the two table columns. A seat's content spans its zone column (COL_X) out to a
// nominal SEAT_COLS battlefield slots; the second column starts a COLUMN_GAP past that so boards
// don't touch. BAND_W is the seat outline/footprint width used for the highlight and bounds.
// Rows beyond SEAT_COLS shrink collision-safely, then expand at the 24-unit size floor. Seven
// full-size slots plus one permanent step between table columns keeps the normal 2×2 table dense.
const SEAT_COLS = 7;
const BATTLEFIELD_ROW_W = (SEAT_COLS - 1) * PERMANENT_STEP + PERMANENT_SIDE;
const SEAT_RIGHT = BATTLEFIELD_ROW_W + GAP;
const COLUMN_GAP = CARD_HSTEP;
const SEAT_STRIDE_X = SEAT_RIGHT - COL_X + COLUMN_GAP; // x distance between column 0 and column 1
const BAND_W = SEAT_RIGHT - COL_X + GAP; // seat footprint width (zone column + nominal battlefield)

// Base RGB per seat (Commander-ready: 4 seats). Build rgba(...) strings at the
// call site so callers pick their own alpha.
export const SEAT_RGB: [number, number, number][] = [
  [90, 200, 140], // P0 green (keeps today's look)
  [90, 150, 240], // P1 blue
  [240, 120, 90], // P2 orange
  [200, 140, 240], // P3 purple
];
export const seatColor = (seat: number, alpha = 1): string =>
  `rgba(${SEAT_RGB[seat % SEAT_RGB.length].join(",")},${alpha})`;

// Where each seat sits, by its offset from the viewer in turn order (viewer = offset 0). A 2×2
// grid: {col, row} with row 0 the top of the table, row 1 the bottom. You anchor bottom-left; the
// next player is front (directly above), then side (beside you), then diagonal.
const CELLS: { col: number; row: number }[] = [
  { col: 0, row: 1 }, // offset 0 — you (bottom-left)
  { col: 0, row: 0 }, // offset 1 — front (across, directly ahead)
  { col: 1, row: 1 }, // offset 2 — side (beside you)
  { col: 1, row: 0 }, // offset 3 — diagonal
];
/** A seat's screen slot: its offset from the viewer in turn order (viewer = slot 0). A viewer
 *  that sits at no seat — the spectator sentinel — anchors on seat 0, so spectators get plain
 *  seat order rather than every seat folding onto one slot. */
export function seatSlot(seat: number, viewer: number, count: number): number {
  const seats = Math.max(1, count);
  const anchor = viewer < seats ? viewer : 0;
  return (seat - anchor + seats) % seats;
}

export function seatCell(seat: number, viewer: number, count: number): { col: number; row: number } {
  return CELLS[seatSlot(seat, viewer, count)] ?? CELLS[0];
}

/** The world-space top-left of a seat's band (its battlefield origin). */
function seatOrigin(seat: number, viewer: number, count: number): { x: number; y: number } {
  const { col, row } = seatCell(seat, viewer, count);
  return { x: col * SEAT_STRIDE_X, y: row * BAND_STRIDE };
}

/** A top-row seat is flipped to face down across the table (rows swapped, zone column reversed,
 * avatar on the far edge); the two bottom-row seats — including you — stay upright. */
export function isFlipped(seat: number, viewer: number, count: number): boolean {
  return seatCell(seat, viewer, count).row === 0;
}

export function avatarLabelOffsets(seat: number, viewer: number, count: number): AvatarLabelOffsets {
  if (isFlipped(seat, viewer, count)) {
    return {
      hand: -AVATAR_HAND_LABEL_OFFSET,
      life: -AVATAR_LIFE_LABEL_BELOW,
      username: -AVATAR_USERNAME_LABEL_BELOW,
      commander: -AVATAR_LABEL_BELOW,
    };
  }

  return {
    hand: AVATAR_HAND_LABEL_OFFSET,
    life: AVATAR_LIFE_LABEL_BELOW,
    username: AVATAR_USERNAME_LABEL_BELOW,
    commander: AVATAR_LABEL_BELOW,
  };
}

/** The world-space band covering a seat's battlefield + zone column, for the outline/highlight. */
export function seatBand(seat: number, viewer: number, count: number): { x: number; y: number; w: number; h: number } {
  const o = seatOrigin(seat, viewer, count);
  return { x: o.x + COL_X - GAP, y: o.y - GAP, w: BAND_W, h: BATTLE_H + GAP };
}

/** World-space center of a seat's life-orb avatar — centered under its band, on the seat's outer
 * edge (below a bottom-row board; above a flipped top-row board). */
export function avatarPos(seat: number, viewer: number, count: number): { x: number; y: number } {
  const o = seatOrigin(seat, viewer, count);
  const band = seatBand(seat, viewer, count);
  const y = isFlipped(seat, viewer, count) ? o.y - AVATAR_R - GAP : o.y + BATTLE_H + AVATAR_R + GAP;
  return { x: band.x + band.w / 2, y };
}

/** World-space center of the first land slot in a seat's lands row — provisional land-play flight aim. */
export function landRowCenter(seat: number, viewer: number, count: number): { x: number; y: number } {
  const o = seatOrigin(seat, viewer, count);
  const landsY = isFlipped(seat, viewer, count) ? o.y : o.y + 2 * ROW_H;
  const metrics = permanentRowMetrics(1, BATTLEFIELD_ROW_W);
  return { x: centerOutX(o.x, 0, 1, metrics) + CARD_W / 2, y: landsY + ROW_H / 2 };
}

/** Synthetic canvas id for a seat's Library pile face (`deckCard`). */
export function libraryPileId(owner: number): number {
  return -1 - owner;
}

/**
 * World-space top-left of a seat's zone-column pile (library / graveyard / exile).
 * Slot order matches `layout()`: upright commander→exile→deck→graveyard; flipped seats reverse.
 */
export function zonePilePos(
  zone: typeof ZONE.Library | typeof ZONE.Graveyard | typeof ZONE.Exile,
  seat: number,
  viewer: number,
  count: number,
): { x: number; y: number } {
  const o = seatOrigin(seat, viewer, count);
  const flip = isFlipped(seat, viewer, count);
  // Indices in the upright column: commander=0, exile=1, library=2, graveyard=3.
  const upright = zone === ZONE.Exile ? 1 : zone === ZONE.Library ? 2 : 3;
  const i = flip ? 3 - upright : upright;
  return { x: o.x + COL_X, y: o.y + i * COL_STRIDE };
}

/** World-space anchor for a seat's mana tray — under the zone column's battlefield-side edge,
 * just outside the seat band on the outer edge (below upright boards; above flipped top-row boards).
 * X sits past the zone column (not its center) so the tray is less likely to collide with the
 * fixed bottom-left game log on the viewer's seat. */
export function manaTrayPos(seat: number, viewer: number, count: number): { x: number; y: number } {
  const o = seatOrigin(seat, viewer, count);
  const band = seatBand(seat, viewer, count);
  const x = o.x + COL_X + COL_W + GAP;
  const y = isFlipped(seat, viewer, count) ? band.y - GAP : band.y + band.h + GAP;
  return { x, y };
}

/** World-space bounding box of the whole table (the union of every seat's band + avatar), so the
 * camera fits it. Shape depends only on how many seats are occupied, not on which is the viewer. */
export type BoardBounds = Readonly<{ minX: number; minY: number; maxX: number; maxY: number }>;

export type BoardLayout = Readonly<{
  cards: readonly RenderCard[];
  seatBands: ReadonlyMap<number, { x: number; y: number; w: number; h: number }>;
  avatarPositions: Readonly<Record<number, { x: number; y: number }>>;
  bounds: BoardBounds;
  boundsKey: string;
}>;

export function boardBounds(count: number): BoardBounds {
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (let seat = 0; seat < count; seat++) {
    const b = seatBand(seat, 0, count);
    const a = avatarPos(seat, 0, count);
    const labelY = a.y + avatarLabelOffsets(seat, 0, count).commander;
    minX = Math.min(minX, b.x, a.x - AVATAR_R);
    minY = Math.min(minY, b.y, a.y - AVATAR_R, labelY);
    maxX = Math.max(maxX, b.x + b.w, a.x + AVATAR_R);
    maxY = Math.max(maxY, b.y + b.h, a.y + AVATAR_R, labelY);
  }
  return { minX, minY, maxX, maxY };
}

function pt(o: ObjectView): string {
  if (o.kind.kind === "creature") return `${o.power}/${o.toughness}`;
  // Current loyalty in the P/T badge slot. Fall back to printed starting loyalty when the
  // live field is absent (old payloads / partial fixtures).
  if (o.kind.kind === "planeswalker") return `${o.loyalty ?? o.kind.loyalty}`;
  // Battles reuse ObjectView.loyalty for live defense counters.
  if (o.kind.kind === "battle") return `${o.loyalty ?? o.kind.defense}`;
  return "";
}

function toCard(o: ObjectView): RenderCard {
  return {
    id: o.id,
    x: 0,
    y: 0,
    w: CARD_W,
    h: CARD_H,
    name: o.name,
    cardId: o.card_id ?? "",
    print: o.print ?? "",
    pt: pt(o),
    tapped: o.tapped,
    counters: o.plus_counters,
    markedDamage: o.marked_damage,
    faceDown: o.face_down ?? false,
    zone: o.zone,
    controller: o.controller,
    owner: o.owner,
    kind: o.kind.kind,
    tapsForMana: o.taps_for_mana ?? false,
    summoningSick: o.summoning_sick,
    hasHaste: o.has_haste,
    keywords: o.keywords ?? [],
    goaded: o.goaded ?? false,
    face: faceDataFrom(o),
    isCommander: o.is_commander,
    prepared: o.prepared ?? false,
    pile: 0,
    cluster: 0,
    clusterMembers: [],
    attachedTo: o.attached_to ?? null,
  };
}

/** A single card standing in for a whole pile (graveyard/exile), showing the top card + count. */
function pileCard(cards: ObjectView[], zone: number): RenderCard | null {
  if (cards.length === 0) return null;
  return { ...toCard(cards[cards.length - 1]), zone, pile: cards.length };
}

/** Shrink a card to the left-column size (deck/graveyard/exile/commander render smaller).
 * Clears P/T so combat chrome does not sit on the half-size face (command/GY/exile art). */
function colCard(card: RenderCard): RenderCard {
  card.w = COL_W;
  card.h = COL_H;
  card.pt = "";
  return card;
}

/** A player's deck slot: the revealed top card face-up when the server itemized one (Field of
 * Dreams — "Players play with the top card of their libraries revealed"), otherwise a face-down
 * placeholder. Either way it shows the whole library's count; an empty library omits the slot
 * (same as an empty graveyard/exile pile). */
function deckCard(owner: number, count: number, revealedTop?: ObjectView): RenderCard | null {
  if (count <= 0) return null;
  if (revealedTop) return { ...toCard(revealedTop), zone: ZONE.Library, pile: count };
  return {
    id: libraryPileId(owner), // synthetic (no object); negative so it never collides with a real id
    x: 0,
    y: 0,
    w: COL_W,
    h: COL_H,
    name: "Library",
    cardId: "",
    print: "",
    pt: "",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: true,
    zone: ZONE.Library,
    controller: owner,
    owner,
    kind: "land",
    tapsForMana: false,
    summoningSick: false,
    hasHaste: false,
    keywords: [],
    goaded: false,
    face: BLANK_FACE,
    isCommander: false,
    prepared: false,
    pile: count,
    cluster: 0,
    clusterMembers: [],
  };
}

function place(card: RenderCard, x: number, y: number): RenderCard {
  card.x = x;
  card.y = y;
  return card;
}

function isAttached(o: ObjectView): boolean {
  return o.attached_to != null;
}

/** Visible-object equality for permanent clusters — explicit fields, sorted arrays (client-game-board-and-interaction spec). */
function clusterKey(o: ObjectView): string {
  const keywords = [...(o.keywords ?? [])].sort().join(",");
  const modifiers = [...(o.modifiers ?? [])]
    .map((m) => `${m.source_name}:${[...m.contributions].sort().join(",")}`)
    .sort()
    .join("|");
  return [
    o.zone,
    o.owner,
    o.controller,
    o.name,
    JSON.stringify(o.kind),
    JSON.stringify(o.mana_cost),
    o.needs_target ? 1 : 0,
    o.tapped ? 1 : 0,
    o.summoning_sick ? 1 : 0,
    o.has_haste ? 1 : 0,
    keywords,
    o.power,
    o.toughness,
    o.loyalty ?? 0,
    o.plus_counters,
    o.marked_damage,
    o.is_commander ? 1 : 0,
    o.goaded ? 1 : 0,
    o.taps_for_mana ? 1 : 0,
    o.prepared ? 1 : 0,
    o.phased_out ? 1 : 0,
    o.attached_to ?? "",
    modifiers,
  ].join("\0");
}

/** One layout face in a battlefield row — a single permanent or a collapsed cluster. */
type RowSlot = { members: ObjectView[] };

/**
 * Build row slots. When the row fits at full spacing, every permanent is its own slot.
 * On overflow, collapse every eligible identical group. `neverMerge` ids always take their own
 * slot — hosts with attachment stacks, and permanents committed to a combat declaration or a
 * target (see `board/engagement.ts`).
 */
function rowSlots(objects: ObjectView[], neverMerge: Set<number>): RowSlot[] {
  if (objects.length <= SEAT_COLS) {
    return objects.map((o) => ({ members: [o] }));
  }
  const keyOf = (o: ObjectView) => (neverMerge.has(o.id) ? `id:${o.id}` : clusterKey(o));
  const order: string[] = [];
  const groups = new Map<string, ObjectView[]>();
  for (const o of objects) {
    const k = keyOf(o);
    if (!groups.has(k)) {
      order.push(k);
      groups.set(k, []);
    }
    groups.get(k)?.push(o);
  }
  return order.map((k) => {
    const group = groups.get(k);
    if (!group) return { members: [] as ObjectView[] };
    const members = [...group].sort((a, b) => a.id - b.id);
    return { members };
  });
}

function toSlotCard(slot: RowSlot): RenderCard {
  const top = slot.members[0];
  const card = toCard(top);
  if (slot.members.length <= 1) return card;
  card.cluster = slot.members.length;
  card.clusterMembers = slot.members.map((m) => m.id);
  return card;
}

/** Center-out X for Creatures / Lands rows. Packs inside the seat when n > SEAT_COLS. */
function centerOutX(originX: number, i: number, n: number, metrics: PermanentRowMetrics): number {
  if (n <= SEAT_COLS) return originX + ((SEAT_COLS - n) / 2 + i) * CARD_HSTEP;
  return originX + i * metrics.step;
}

/**
 * Noncreature row X: artifacts then enchantments from seat-local left; planeswalkers from
 * seat-local right. Packs inside the seat when left + PW counts exceed SEAT_COLS.
 */
function noncreatureXs(
  originX: number,
  leftCount: number,
  pwCount: number,
  metrics: PermanentRowMetrics,
): { left: number[]; pws: number[] } {
  const n = leftCount + pwCount;
  if (n <= SEAT_COLS) {
    const left = Array.from({ length: leftCount }, (_, i) => originX + i * CARD_HSTEP);
    const pws = Array.from({ length: pwCount }, (_, i) => originX + (SEAT_COLS - pwCount + i) * CARD_HSTEP);
    return { left, pws };
  }
  const left = Array.from({ length: leftCount }, (_, i) => originX + i * metrics.step);
  const pws = Array.from({ length: pwCount }, (_, i) => originX + (leftCount + i) * metrics.step);
  return { left, pws };
}

/** Kind goes in the Noncreature left block (artifacts, enchantments, and unknown non-PW kinds). */
function isNoncreatureLeft(kind: Kind): boolean {
  return kind !== "creature" && kind !== "land" && kind !== "planeswalker";
}

type SeatRows = Readonly<{
  who: number;
  player: VisibleState["players"][number];
  cell: { col: number; row: number };
  flip: boolean;
  landSlots: RowSlot[];
  landMetrics: PermanentRowMetrics;
  creatureSlots: RowSlot[];
  creatureMetrics: PermanentRowMetrics;
  leftSlots: RowSlot[];
  pwSlots: RowSlot[];
  noncreatureMetrics: PermanentRowMetrics;
}>;

export function layoutBoard(
  state: VisibleState,
  viewer: number,
  engaged: ReadonlySet<number> = new Set(),
): BoardLayout {
  const count = state.players.length;
  const out: RenderCard[] = [];

  const inZone = (zone: number, who: number) => state.objects.filter((o) => o.zone === zone && o.owner === who);
  const controls = (zone: number, who: number) => state.objects.filter((o) => o.zone === zone && o.controller === who);

  const battlefieldObjects = state.objects.filter((object) => object.zone === ZONE.Battlefield);
  const battlefieldById = new Map(battlefieldObjects.map((object) => [object.id, object]));
  const attachedHostId = (o: ObjectView): number | null => (o.attached_to == null ? null : o.attached_to);
  const attachmentRootMemo = new Map<number, number | null>();
  const attachmentRoot = (object: ObjectView, visiting: ReadonlySet<number> = new Set()): number | null => {
    const memoized = attachmentRootMemo.get(object.id);
    if (memoized !== undefined || attachmentRootMemo.has(object.id)) return memoized ?? null;
    if (visiting.has(object.id)) {
      attachmentRootMemo.set(object.id, null);
      return null;
    }

    const hostId = attachedHostId(object);
    if (hostId == null) {
      attachmentRootMemo.set(object.id, object.id);
      return object.id;
    }
    const host = battlefieldById.get(hostId);
    if (host == null) {
      attachmentRootMemo.set(object.id, null);
      return null;
    }

    const nextVisiting = new Set(visiting);
    nextVisiting.add(object.id);
    const root = attachmentRoot(host, nextVisiting);
    attachmentRootMemo.set(object.id, root);
    return root;
  };
  // Only chains that terminate at a free battlefield root stack. Missing hosts and cycles fall
  // back to their controller's semantic row so malformed authority never makes a card vanish.
  const stacksOnHost = (object: ObjectView) => isAttached(object) && attachmentRoot(object) != null;
  const hostsWithAttachments = new Set(
    battlefieldObjects
      .filter((o) => stacksOnHost(o))
      .map((o) => attachedHostId(o))
      .filter((id): id is number => id != null),
  );

  // Attachment hosts and committed permanents share one rule: never collapse into a cluster.
  const neverMerge = new Set([...hostsWithAttachments, ...engaged]);

  const seats: SeatRows[] = state.players.map((player) => {
    const who = player.player;
    const cell = seatCell(who, viewer, count);
    const flip = cell.row === 0;
    const bf = controls(ZONE.Battlefield, who).filter((card) => !stacksOnHost(card));
    const planeswalkers = bf.filter((card) => card.kind.kind === "planeswalker");
    const creatures = bf.filter((card) => card.kind.kind === "creature");
    const lands = bf.filter((card) => card.kind.kind === "land");
    const leftBlock = bf.filter((card) => isNoncreatureLeft(card.kind.kind));
    leftBlock.sort((left, right) => {
      const rank = (kind: Kind) => (kind === "artifact" ? 0 : kind === "enchantment" ? 1 : 2);
      return rank(left.kind.kind) - rank(right.kind.kind);
    });

    const noncreatureSlots = rowSlots([...leftBlock, ...planeswalkers], neverMerge);
    const leftSlots = noncreatureSlots.filter((slot) => isNoncreatureLeft(slot.members[0].kind.kind));
    const pwSlots = noncreatureSlots.filter((slot) => slot.members[0].kind.kind === "planeswalker");
    const creatureSlots = rowSlots(creatures, neverMerge);
    const landSlots = rowSlots(lands, neverMerge);

    return {
      who,
      player,
      cell,
      flip,
      landSlots,
      landMetrics: permanentRowMetrics(landSlots.length, BATTLEFIELD_ROW_W),
      creatureSlots,
      creatureMetrics: permanentRowMetrics(creatureSlots.length, BATTLEFIELD_ROW_W),
      leftSlots,
      pwSlots,
      noncreatureMetrics: permanentRowMetrics(noncreatureSlots.length, BATTLEFIELD_ROW_W),
    };
  });

  const columnWidths: [number, number] = [BATTLEFIELD_ROW_W, BATTLEFIELD_ROW_W];
  for (const seat of seats) {
    columnWidths[seat.cell.col] = Math.max(
      columnWidths[seat.cell.col],
      seat.landMetrics.width,
      seat.creatureMetrics.width,
      seat.noncreatureMetrics.width,
    );
  }
  const columnRights: [number, number] = [
    Math.max(SEAT_RIGHT, columnWidths[0] + GAP),
    Math.max(SEAT_RIGHT, columnWidths[1] + GAP),
  ];
  const columnOrigins: [number, number] = [0, columnRights[0] - COL_X + COLUMN_GAP];
  const seatBands = new Map<number, { x: number; y: number; w: number; h: number }>();
  const avatarPositions: Record<number, { x: number; y: number }> = {};

  const originFor = (seat: SeatRows) => ({
    x: columnOrigins[seat.cell.col],
    y: seat.cell.row * BAND_STRIDE,
  });

  for (const seat of seats) {
    const origin = originFor(seat);
    const band = {
      x: origin.x + COL_X - GAP,
      y: origin.y - GAP,
      w: columnRights[seat.cell.col] - COL_X + GAP,
      h: BATTLE_H + GAP,
    };
    seatBands.set(seat.who, band);
    avatarPositions[seat.who] = {
      x: band.x + band.w / 2,
      y: seat.flip ? origin.y - AVATAR_R - GAP : origin.y + BATTLE_H + AVATAR_R + GAP,
    };
  }

  /** Host id → world top-left and size of the host card (filled as free permanents are placed). */
  const hostPos = new Map<number, { x: number; y: number; flip: boolean; side: number }>();

  for (const seat of seats) {
    const p = seat.player;
    const who = seat.who;
    const o = originFor(seat);
    const flip = seat.flip;
    // Centerward → outer: Noncreature, Creatures, Lands. Flipped seats reverse so the same
    // reading holds (center = combat/noncreature, outer = mana).
    const noncreatureY = flip ? o.y + 2 * ROW_H : o.y;
    const creaturesY = o.y + ROW_H;
    const landsY = flip ? o.y : o.y + 2 * ROW_H;

    // Zone column, top → bottom for your own board: commander, exile, deck, graveyard. Flipped
    // for a top-row seat so it reads bottom → top graveyard, deck, exile, commander from their side.
    const cmd = inZone(ZONE.Command, who)[0];
    const slots: (RenderCard | null)[] = [
      cmd ? toCard(cmd) : null,
      pileCard(inZone(ZONE.Exile, who), ZONE.Exile),
      deckCard(who, p.library_count, inZone(ZONE.Library, who)[0]),
      pileCard(inZone(ZONE.Graveyard, who), ZONE.Graveyard),
    ];
    (flip ? [...slots].reverse() : slots).forEach((card, i) => {
      if (card) out.push(place(colCard(card), o.x + COL_X, o.y + i * COL_STRIDE));
    });

    const placeSlot = (slot: RowSlot, x: number, rowY: number, metrics: PermanentRowMetrics) => {
      const y = rowY + (ROW_H - metrics.side) / 2;
      const card = place({ ...toSlotCard(slot), w: metrics.side, h: metrics.side }, x, y);
      out.push(card);
      for (const member of slot.members) hostPos.set(member.id, { x, y, flip, side: metrics.side });
    };

    seat.landSlots.forEach((slot, index) => {
      placeSlot(slot, centerOutX(o.x, index, seat.landSlots.length, seat.landMetrics), landsY, seat.landMetrics);
    });

    seat.creatureSlots.forEach((slot, index) => {
      placeSlot(
        slot,
        centerOutX(o.x, index, seat.creatureSlots.length, seat.creatureMetrics),
        creaturesY,
        seat.creatureMetrics,
      );
    });

    const { left: leftXs, pws: pwXs } = noncreatureXs(
      o.x,
      seat.leftSlots.length,
      seat.pwSlots.length,
      seat.noncreatureMetrics,
    );
    seat.leftSlots.forEach((slot, index) => {
      placeSlot(slot, leftXs[index], noncreatureY, seat.noncreatureMetrics);
    });
    seat.pwSlots.forEach((slot, index) => {
      placeSlot(slot, pwXs[index], noncreatureY, seat.noncreatureMetrics);
    });
  }

  // Attached Auras/Equipment stack on their immediate host (any controller). Descendant subtrees
  // emit before their parent, leaving every host above its attachment in draw/hit order.
  const attachments = battlefieldObjects.filter((object) => stacksOnHost(object));
  const byHost = new Map<number, ObjectView[]>();
  for (const attachment of attachments) {
    const hostId = attachedHostId(attachment);
    if (hostId == null) continue;
    const list = byHost.get(hostId) ?? [];
    list.push(attachment);
    byHost.set(hostId, list);
  }

  const cardsBelowHost = (hostId: number, ancestors: ReadonlySet<number>): RenderCard[] => {
    const host = hostPos.get(hostId);
    if (host == null || ancestors.has(hostId)) return [];

    const nextAncestors = new Set(ancestors);
    nextAncestors.add(hostId);
    const children = byHost.get(hostId) ?? [];
    const cards: RenderCard[] = [];
    for (const [index, attachment] of children.entries()) {
      if (nextAncestors.has(attachment.id)) continue;
      const dy = (host.flip ? 1 : -1) * host.side * 0.2 * (index + 1);
      const pose = { x: host.x, y: host.y + dy, flip: host.flip, side: host.side };
      hostPos.set(attachment.id, pose);
      cards.push(...cardsBelowHost(attachment.id, nextAncestors));
      cards.push(place({ ...toCard(attachment), w: pose.side, h: pose.side }, pose.x, pose.y));
    }
    return cards;
  };

  const attachmentRoots = new Set(
    attachments.map((attachment) => attachmentRoot(attachment)).filter((id): id is number => id != null),
  );
  for (const rootId of attachmentRoots) {
    const hostIdx = out.findIndex((card) => card.id === rootId);
    if (hostIdx < 0) continue;
    out.splice(hostIdx, 0, ...cardsBelowHost(rootId, new Set()));
  }

  let bounds: BoardBounds;
  if (seats.length === 0) {
    bounds = boardBounds(1);
  } else {
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const seat of seats) {
      const band = seatBands.get(seat.who);
      const avatar = avatarPositions[seat.who];
      if (band == null || avatar == null) continue;
      const labelY = avatar.y + avatarLabelOffsets(seat.who, viewer, count).commander;
      minX = Math.min(minX, band.x, avatar.x - AVATAR_R);
      minY = Math.min(minY, band.y, avatar.y - AVATAR_R, labelY);
      maxX = Math.max(maxX, band.x + band.w, avatar.x + AVATAR_R);
      maxY = Math.max(maxY, band.y + band.h, avatar.y + AVATAR_R, labelY);
    }
    bounds = { minX, minY, maxX, maxY };
  }

  return {
    cards: out,
    seatBands,
    avatarPositions,
    bounds,
    boundsKey: `${bounds.minX}:${bounds.minY}:${bounds.maxX}:${bounds.maxY}`,
  };
}

export function layout(state: VisibleState, viewer: number, engaged: ReadonlySet<number> = new Set()): RenderCard[] {
  return [...layoutBoard(state, viewer, engaged).cards];
}
