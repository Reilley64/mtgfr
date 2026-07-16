// Pure layout: turn a per-viewer VisibleState into positioned render cards for the canvas,
// arranged as a Commander table. Each seat gets a three-row battlefield (Noncreature, Creatures,
// Lands — centerward → outer), a compact zone column on the left (top → bottom: commander, exile,
// deck, graveyard), a life-orb avatar on its outer edge, and a mana tray under the zone column
// outside the seat band. Seats sit around the table as a 2×2 quadrant — you at the bottom-left,
// and the other players in turn order at *front* (directly above you), *side* (beside you), then
// *diagonal* — matching where they'd physically sit. Fewer than four just leave later cells empty
// (2p → you + front, a vertical pair; 3p → drops the diagonal). Top-row seats are flipped to face
// down across the table; the two bottom seats are upright. One camera pans/zooms the whole table.
// The hand is a DOM overlay (Hand.tsx); the mana tray is a world-anchored DOM overlay (ManaTray.tsx).

import type { ObjectView, VisibleState, WireKind } from "~/api/generated";

/** Zone discriminants — must match `engine::Zone`'s declaration order. */
export const ZONE = {
  Library: 0,
  Hand: 1,
  Battlefield: 2,
  Graveyard: 3,
  Exile: 4,
  Command: 5,
  Stack: 6,
} as const;

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
  /** Card (oracle) id, when known — drives Alt-pin inspect's oracle-text lookup (ADR 0031). */
  cardId: string;
  /** Printing UUID for this object's art (ADR 0031); empty renders a broken image. */
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
  /** Extra world-space tilt for a fanned permanent-cluster member (radians, about card center). */
  fanAngle?: number;
}

export const CARD_W = 96;
export const CARD_H = 134;
const GAP = 16;
const CARD_HSTEP = CARD_W + GAP; // horizontal distance between card centers
const VSTEP = CARD_H + GAP;
const ROW_H = VSTEP; // one battlefield row (card + gap)

/** Radius of a player's life-orb avatar, in world units (so it pans/zooms with the board). */
export const AVATAR_R = 60;

// Each seat: a battlefield of three rows (Noncreature → Creatures → Lands, centerward → outer)
// with a compact zone column on the left (deck / graveyard / exile / commander), a life orb on
// its outer edge, and a mana tray under the zone column outside the seat band. Empty rows still
// reserve height (stable seat silhouette).
const BATTLE_H = 3 * ROW_H;
const BAND_GAP = 2 * AVATAR_R + 2 * GAP; // room for the avatar above/below each band
const BAND_STRIDE = BATTLE_H + BAND_GAP; // vertical distance between the two table rows

// The left column's cards are rendered at half size so four stack alongside the three-row
// battlefield (4 × COL_STRIDE ≈ BATTLE_H). Top → bottom: commander, exile, deck, graveyard.
const COL_W = CARD_W * 0.5;
const COL_H = CARD_H * 0.5;
const COL_STRIDE = BATTLE_H / 4;
const COL_X = -(COL_W + 2 * GAP); // just left of the battlefield's first card (x = 0)

// Horizontal grid: the two table columns. A seat's content spans its zone column (COL_X) out to a
// nominal SEAT_COLS battlefield slots; the second column starts a COLUMN_GAP past that so boards
// don't touch. BAND_W is the seat outline/footprint width used for the highlight and bounds.
// Rows that exceed SEAT_COLS pack (compress step) inside the seat — see ADR 0028.
const SEAT_COLS = 9;
const SEAT_RIGHT = SEAT_COLS * CARD_HSTEP;
const COLUMN_GAP = 2 * CARD_HSTEP;
const SEAT_STRIDE_X = SEAT_RIGHT - COL_X + COLUMN_GAP; // x distance between column 0 and column 1
const BAND_W = SEAT_RIGHT - COL_X + GAP; // seat footprint width (zone column + nominal battlefield)

/** Seat-local centerward offset per attachment index (peek behind/above the host). */
const ATTACH_OFFSET = CARD_H * 0.2;

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
function seatCell(seat: number, viewer: number, count: number): { col: number; row: number } {
  return CELLS[(seat - viewer + count) % count] ?? CELLS[0];
}

/** The world-space top-left of a seat's band (its battlefield origin). */
function seatOrigin(seat: number, viewer: number, count: number): { x: number; y: number } {
  const { col, row } = seatCell(seat, viewer, count);
  return { x: col * SEAT_STRIDE_X, y: row * BAND_STRIDE };
}

/** A top-row seat is flipped to face down across the table (rows swapped, zone column reversed,
 * avatar on the far edge); the two bottom-row seats — including you — stay upright. */
function isFlipped(seat: number, viewer: number, count: number): boolean {
  return seatCell(seat, viewer, count).row === 0;
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
export function boardBounds(count: number): { minX: number; minY: number; maxX: number; maxY: number } {
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (let seat = 0; seat < count; seat++) {
    const b = seatBand(seat, 0, count);
    const a = avatarPos(seat, 0, count);
    minX = Math.min(minX, b.x, a.x - AVATAR_R);
    minY = Math.min(minY, b.y, a.y - AVATAR_R);
    maxX = Math.max(maxX, b.x + b.w, a.x + AVATAR_R);
    maxY = Math.max(maxY, b.y + b.h, a.y + AVATAR_R);
  }
  return { minX, minY, maxX, maxY };
}

function pt(o: ObjectView): string {
  if (o.kind.kind === "creature") return `${o.power}/${o.toughness}`;
  // Current loyalty in the P/T badge slot. Fall back to printed starting loyalty when the
  // live field is absent (old payloads / partial fixtures).
  if (o.kind.kind === "planeswalker") return `${o.loyalty ?? o.kind.loyalty}`;
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

/** Shrink a card to the left-column size (deck/graveyard/exile/commander render smaller). */
function colCard(card: RenderCard): RenderCard {
  card.w = COL_W;
  card.h = COL_H;
  return card;
}

/** A face-down "deck" placeholder for a player's hidden library, showing its card count.
 * Empty libraries omit the slot (same as an empty graveyard/exile pile). */
function deckCard(owner: number, count: number): RenderCard | null {
  if (count <= 0) return null;
  return {
    id: -1 - owner, // synthetic (no object); negative so it never collides with a real id
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

/** Visible-object equality for permanent clusters — explicit fields, sorted arrays (ADR 0028). */
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
 * On overflow, collapse every eligible identical group (hosts with attachment stacks never merge).
 */
function rowSlots(objects: ObjectView[], hostsWithAttachments: Set<number>): RowSlot[] {
  if (objects.length <= SEAT_COLS) {
    return objects.map((o) => ({ members: [o] }));
  }
  const keyOf = (o: ObjectView) => (hostsWithAttachments.has(o.id) ? `id:${o.id}` : clusterKey(o));
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
    const members = [...groups.get(k)!].sort((a, b) => a.id - b.id);
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

/** Horizontal step when packing `n` cards into the seat band (top-left xs in [0, SEAT_RIGHT - CARD_W]). */
function packStep(n: number): number {
  return (SEAT_RIGHT - CARD_W) / (n - 1);
}

/** Center-out X for Creatures / Lands rows. Packs inside the seat when n > SEAT_COLS. */
function centerOutX(originX: number, i: number, n: number): number {
  if (n <= SEAT_COLS) return originX + ((SEAT_COLS - n) / 2 + i) * CARD_HSTEP;
  return originX + i * packStep(n);
}

/**
 * Noncreature row X: artifacts then enchantments from seat-local left; planeswalkers from
 * seat-local right. Packs inside the seat when left + PW counts exceed SEAT_COLS.
 */
function noncreatureXs(originX: number, leftCount: number, pwCount: number): { left: number[]; pws: number[] } {
  const n = leftCount + pwCount;
  if (n <= SEAT_COLS) {
    const left = Array.from({ length: leftCount }, (_, i) => originX + i * CARD_HSTEP);
    const pws = Array.from({ length: pwCount }, (_, i) => originX + (SEAT_COLS - pwCount + i) * CARD_HSTEP);
    return { left, pws };
  }
  const step = packStep(n);
  const left = Array.from({ length: leftCount }, (_, i) => originX + i * step);
  const pws = Array.from({ length: pwCount }, (_, i) => originX + (leftCount + i) * step);
  return { left, pws };
}

/** Kind goes in the Noncreature left block (artifacts, enchantments, and unknown non-PW kinds). */
function isNoncreatureLeft(kind: Kind): boolean {
  return kind !== "creature" && kind !== "land" && kind !== "planeswalker";
}

export function layout(state: VisibleState, viewer: number): RenderCard[] {
  const count = state.players.length;
  const out: RenderCard[] = [];

  const inZone = (zone: number, who: number) => state.objects.filter((o) => o.zone === zone && o.owner === who);
  const controls = (zone: number, who: number) => state.objects.filter((o) => o.zone === zone && o.controller === who);

  // Hosts that will get a row slot — attachments whose host is missing fall back to a free slot
  // so they never vanish from the board.
  const freeHostIds = new Set(
    state.objects.filter((o) => o.zone === ZONE.Battlefield && !isAttached(o)).map((o) => o.id),
  );
  const stacksOnHost = (o: ObjectView) => isAttached(o) && freeHostIds.has(o.attached_to!);
  const hostsWithAttachments = new Set(
    state.objects.filter((o) => o.zone === ZONE.Battlefield && stacksOnHost(o)).map((o) => o.attached_to!),
  );

  /** Host id → world top-left of the host card (filled as free permanents are placed). */
  const hostPos = new Map<number, { x: number; y: number; flip: boolean }>();

  for (const p of state.players) {
    const who = p.player;
    const o = seatOrigin(who, viewer, count);
    const flip = isFlipped(who, viewer, count);
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
      deckCard(who, p.library_count),
      pileCard(inZone(ZONE.Graveyard, who), ZONE.Graveyard),
    ];
    (flip ? [...slots].reverse() : slots).forEach((card, i) => {
      if (card) out.push(place(colCard(card), o.x + COL_X, o.y + i * COL_STRIDE));
    });

    // Free permanents + orphan attachments (host missing). Stacked attachments wait for pass 2.
    const bf = controls(ZONE.Battlefield, who).filter((c) => !stacksOnHost(c));
    const planeswalkers = bf.filter((c) => c.kind.kind === "planeswalker");
    const creatures = bf.filter((c) => c.kind.kind === "creature");
    const lands = bf.filter((c) => c.kind.kind === "land");
    // Artifacts, enchantments, and any other non-PW kind (safety net for unexpected WireKinds).
    const leftBlock = bf.filter((c) => isNoncreatureLeft(c.kind.kind));
    // Stable type order within the left block: artifacts, then enchantments, then the rest.
    leftBlock.sort((a, b) => {
      const rank = (k: Kind) => (k === "artifact" ? 0 : k === "enchantment" ? 1 : 2);
      return rank(a.kind.kind) - rank(b.kind.kind);
    });

    const placeSlot = (slot: RowSlot, x: number, y: number) => {
      const card = place(toSlotCard(slot), x, y);
      out.push(card);
      for (const m of slot.members) hostPos.set(m.id, { x, y, flip });
    };

    const ncSlots = rowSlots([...leftBlock, ...planeswalkers], hostsWithAttachments);
    const leftSlots = ncSlots.filter((s) => isNoncreatureLeft(s.members[0].kind.kind));
    const pwSlots = ncSlots.filter((s) => s.members[0].kind.kind === "planeswalker");
    const { left: leftXs, pws: pwXs } = noncreatureXs(o.x, leftSlots.length, pwSlots.length);
    leftSlots.forEach((slot, i) => {
      placeSlot(slot, leftXs[i], noncreatureY);
    });
    pwSlots.forEach((slot, i) => {
      placeSlot(slot, pwXs[i], noncreatureY);
    });

    const creatureSlots = rowSlots(creatures, hostsWithAttachments);
    creatureSlots.forEach((slot, i) => {
      placeSlot(slot, centerOutX(o.x, i, creatureSlots.length), creaturesY);
    });

    const landSlots = rowSlots(lands, hostsWithAttachments);
    landSlots.forEach((slot, i) => {
      placeSlot(slot, centerOutX(o.x, i, landSlots.length), landsY);
    });
  }

  // Attached Auras/Equipment stack on their host (any controller), under the host in draw/hit order.
  const attachments = state.objects.filter((o) => o.zone === ZONE.Battlefield && stacksOnHost(o));
  const byHost = new Map<number, ObjectView[]>();
  for (const a of attachments) {
    const hostId = a.attached_to!;
    const list = byHost.get(hostId) ?? [];
    list.push(a);
    byHost.set(hostId, list);
  }
  for (const [hostId, list] of byHost) {
    const host = hostPos.get(hostId);
    if (!host) continue; // defensive — stacksOnHost already required a free host
    const hostIdx = out.findIndex((c) => c.id === hostId);
    if (hostIdx < 0) continue;
    const dy = host.flip ? ATTACH_OFFSET : -ATTACH_OFFSET;
    const cards = list.map((a, i) => place(toCard(a), host.x, host.y + dy * (i + 1)));
    out.splice(hostIdx, 0, ...cards);
  }

  return out;
}
