// Paint/hit overlays for overcrowded battlefields: hover-raise packed cards and fan
// permanent clusters. Pure transforms over layout output — see client-game-board-and-interaction spec.

import { CARD_H, CARD_W, type RenderCard, seatBand } from "./layout";

/** Peek step between fanned cluster members (less than full tidy spacing). */
const FAN_STEP = CARD_W * 0.45;

/** Max tilt for an outer fan member (matches the hand bar's MTGA-style arc). */
const FAN_ANGLE_STEP = (3 * Math.PI) / 180;
const FAN_ANGLE_MAX = (12 * Math.PI) / 180;

export type ClusterFanOpts = {
  viewer: number;
  playerCount: number;
};

/** Host id for an attachment stack raise: the permanent under the pointer, or its host. */
function raiseHostId(hit: RenderCard): number {
  return hit.attachedTo ?? hit.id;
}

/**
 * Bring the raised card (and its real attachment stack) to the end of the list so it paints
 * and hit-tests on top. Uses `attachedTo`, not shared x — same-column neighbours stay put.
 */
export function withHoverRaise(cards: readonly RenderCard[], raiseId: number | null): RenderCard[] {
  if (raiseId == null) return cards as RenderCard[];
  const hit = cards.find((c) => c.id === raiseId);
  if (!hit) return cards as RenderCard[];

  const hostId = raiseHostId(hit);
  const raise = new Set<number>([hostId]);
  for (const c of cards) {
    if (c.attachedTo === hostId) raise.add(c.id);
  }

  const rest: RenderCard[] = [];
  const lifted: RenderCard[] = [];
  for (const c of cards) {
    if (raise.has(c.id)) lifted.push(c);
    else rest.push(c);
  }
  return [...rest, ...lifted];
}

/** MTGA-style arc offset for fan index `i` of `n` (tilt + sink). */
export function clusterFanPose(i: number, n: number): { fanAngle: number; drop: number } {
  const off = i - (n - 1) / 2;
  const fanAngle = Math.max(-FAN_ANGLE_MAX, Math.min(FAN_ANGLE_MAX, off * FAN_ANGLE_STEP));
  const drop = Math.min(CARD_H * 0.2, off * off * 2.2);
  return { fanAngle, drop };
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n));
}

/**
 * Replace a permanent-cluster face with one card per member, fanned in an arc about the
 * cluster slot. Members share the face's visible fields (they are identical by construction).
 * When `opts` is set, the fan is clamped inside the controller's seat band.
 */
export function withClusterFan(
  cards: readonly RenderCard[],
  fannedClusterId: number | null,
  opts?: ClusterFanOpts,
): RenderCard[] {
  if (fannedClusterId == null) return cards as RenderCard[];
  const idx = cards.findIndex((c) => c.id === fannedClusterId && c.cluster > 1);
  if (idx < 0) return cards as RenderCard[];

  const cluster = cards[idx];
  const n = cluster.clusterMembers.length;
  let step = FAN_STEP;
  let startX = cluster.x - ((n - 1) * step) / 2;

  let minX = -Infinity;
  let maxX = Infinity;
  let minY = -Infinity;
  let maxY = Infinity;
  if (opts) {
    const band = seatBand(cluster.controller, opts.viewer, opts.playerCount);
    minX = band.x;
    maxX = band.x + band.w - CARD_W;
    minY = band.y;
    maxY = band.y + band.h - CARD_H;
    const maxSpan = Math.max(0, maxX - minX);
    const idealSpan = (n - 1) * step;
    if (n > 1 && idealSpan > maxSpan) {
      step = maxSpan / (n - 1);
      startX = minX;
    } else {
      startX = clamp(startX, minX, maxX - (n - 1) * step);
    }
  }

  const members = cluster.clusterMembers.map((id, i) => {
    const { fanAngle, drop } = clusterFanPose(i, n);
    return {
      ...cluster,
      id,
      x: startX + i * step,
      y: clamp(cluster.y + drop, minY, maxY),
      fanAngle,
      cluster: 0,
      clusterMembers: [] as number[],
    };
  });
  // Fan paints/hits above every other permanent; hover-raise then pulls one member to the top.
  return [...cards.slice(0, idx), ...cards.slice(idx + 1), ...members];
}

/** Fan first (so members exist), then raise selection (else hover). */
export function withBoardDensity(
  cards: readonly RenderCard[],
  opts: {
    hoverId: number | null;
    fannedClusterId: number | null;
    raiseId?: number | null;
    viewer?: number;
    playerCount?: number;
  },
): RenderCard[] {
  const raiseId = opts.raiseId !== undefined ? opts.raiseId : opts.hoverId;
  const fanOpts =
    opts.viewer !== undefined && opts.playerCount !== undefined
      ? { viewer: opts.viewer, playerCount: opts.playerCount }
      : undefined;
  return withHoverRaise(withClusterFan(cards, opts.fannedClusterId, fanOpts), raiseId);
}
