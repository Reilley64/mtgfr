// TableSurface — deep module for the board foundations AGENTS.md assumes:
// camera transform (SoT for pan/zoom) + screen→world hit-testing + pointer→SurfaceEffects
// + paint-only tween inputs + aux inspect hover.
//
// Board is the thin composition root; combat staging and ActionSession *call*
// hitCard/hitSeat and apply SurfaceEffects — they are not deps inside the surface.
//
// Invariant: hits use density-overlaid logical `cards` (layout positions + hover-raise /
// cluster fan). Tweened/drawn positions belong to the paint path only — never pass them
// into hit*.

import { type Accessor, createEffect, createMemo, createSignal, onCleanup } from "solid-js";
import { seedEntrances, type ZonePileKind } from "~/controllers/tableEntrances";
import { avatarPos, type RenderCard, ZONE } from "~/layout";
import { withBoardDensity } from "~/lib/boardDensity";
import { type Camera, panBy, zoomAt as zoomCameraAt } from "~/lib/camera";
import { hitAvatar, hitTest } from "~/lib/hitTest";
import { type InspectPin, pinFromHit } from "~/lib/inspect";
import {
  fitCamera,
  type PointerPhase,
  pointerDown as reduceDown,
  pointerMove as reduceMove,
  pointerUp as reduceUp,
} from "~/lib/interaction";
import { type Positions, snapAll, stepScalar, stepToward } from "~/lib/tween";

export type { EntranceSeedOpts, ZonePileKind } from "~/controllers/tableEntrances";
export { seedEntrances } from "~/controllers/tableEntrances";

export type Vec = { x: number; y: number };

/** Touch hold duration before a permanent cluster fans (ADR 0028). */
export const CLUSTER_LONG_PRESS_MS = 400;

export type PointerCtx = {
  combatStep: boolean;
  me: number;
  /** Seats legal to aim (e.g. staged player targets). */
  aimSeats: readonly number[];
};

/** Effects from pointerUp / aim resolution — applied by Board to session/combat/selection. */
export type SurfaceEffect =
  | { kind: "click"; card: RenderCard }
  | { kind: "combat-drop"; card: RenderCard; x: number; y: number }
  | { kind: "aim-seat"; seat: number }
  | { kind: "clear-selection" }
  | { kind: "none" };

/** Chrome-only feedback from pointerMove while a combat drag is active. */
export type PointerMoveEffect = { kind: "drag"; card: RenderCard };

export type TableSurfaceDeps = {
  me: Accessor<number>;
  playerCount: Accessor<number>;
  /** Logical layout only — never tweened/drawn positions. */
  cards: Accessor<RenderCard[]>;
  handBarH: number;
  /** Initial viewport size (defaults to `window` when omitted). */
  initialSize?: Vec;
  /** Cross-zone glide predecessors (newId → oldId). Defaults to empty. */
  zoneMoves?: Accessor<Map<number, number>>;
  /** Permanents that entered from stack resolution. Defaults to empty. */
  fromStack?: Accessor<Set<number>>;
  /** Cards that left the stack to GY/exile. Defaults to empty. */
  fromStackExit?: Accessor<Set<number>>;
  /** Token id → creator object id. Defaults to empty. */
  tokenCreators?: Accessor<Map<number, number>>;
  /** Own play permanent id → world origin (optional; PlayMotion usually owns via skipIds). */
  playEntrances?: Accessor<Map<number, Vec>>;
  /** Zone-pile BF entrances. Defaults to empty. */
  zonePileEntrances?: Accessor<Map<number, { zone: ZonePileKind; seat: number }>>;
  /** Object ids that were on the stack in the prior frame (token creator hybrid). */
  stackObjectIds?: Accessor<Set<number>>;
  /** Live stack length for stack→battlefield entrance seed. */
  stackLength?: Accessor<number>;
  /** Ids owned by PlayMotion — skip competing entrance seeds. */
  flightOwnedIds?: Accessor<ReadonlySet<number>>;
  /** Override for tests; defaults to `prefers-reduced-motion`. */
  reducedMotion?: Accessor<boolean>;
  /** Selected permanent — stays raised / fans pinned until cleared. */
  selectedId?: Accessor<number | null>;
};

export type TableSurface = {
  camera: Accessor<Camera>;
  size: Accessor<Vec>;
  setSize: (s: Vec) => void;
  /** Pan by screen pixels; marks the view as user-controlled (stops auto-fit). */
  pan(dx: number, dy: number): void;
  /** Zoom about a screen point; marks user-controlled. */
  zoomAt(sx: number, sy: number, factor: number): void;
  /** Topmost density-overlaid logical card under the screen point, or null. */
  hitCard(sx: number, sy: number): RenderCard | null;
  /** Seat whose avatar is under the screen point among `seats`, or null. */
  hitSeat(sx: number, sy: number, seats: readonly number[]): number | null;

  pointerDown(sx: number, sy: number, ctx: PointerCtx): void;
  /** Pan applied inside; returns drag chrome while a combat drag is active. */
  pointerMove(sx: number, sy: number): PointerMoveEffect | null;
  pointerUp(sx: number, sy: number): SurfaceEffect;
  pointerCancel(): void;
  /** Combat-drag card under the cursor chrome, or null. */
  dragging: Accessor<RenderCard | null>;

  /** Paint path only — never feed to hit*. */
  drawnCards: Accessor<RenderCard[]>;

  notePointer(sx: number, sy: number): void;
  setAuxHover(source: "hand" | "stack", card: { name: string; cardId?: string; print?: string } | null): void;
  tryPinInspect(): InspectPin | null;
  clearInspect(): void;
  inspectPin: Accessor<InspectPin | null>;
};

function defaultSize(): Vec {
  return { x: window.innerWidth, y: window.innerHeight };
}

function defaultReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Pure hit: id from hitTest → card from the same logical list (no tween). */
export function hitLogicalCard(cam: Camera, cards: readonly RenderCard[], sx: number, sy: number): RenderCard | null {
  const id = hitTest(cam, sx, sy, cards);
  if (id === null || id < 0) return null;
  return cards.find((c) => c.id === id) ?? null;
}

/** Avatar world centers for the given seats, oriented for `me` / playerCount. */
export function avatarWorldFor(seats: readonly number[], me: number, playerCount: number): Record<number, Vec> {
  const out: Record<number, Vec> = {};
  for (const s of seats) out[s] = avatarPos(s, me, playerCount);
  return out;
}

/** True when `id` is a member of the currently fanned permanent cluster. */
function pinsFan(logical: readonly RenderCard[], fannedClusterId: number | null, id: number | null): boolean {
  if (id == null || fannedClusterId == null) return false;
  const cluster = logical.find((c) => c.id === fannedClusterId);
  return !!cluster && cluster.clusterMembers.includes(id);
}

/** Member under the pointer on an open fan (no per-member raise — peeks resolve fairly). */
export function fanMemberAt(
  cam: Camera,
  logical: readonly RenderCard[],
  fannedClusterId: number,
  sx: number,
  sy: number,
  seat: { viewer: number; playerCount: number },
): number | null {
  const cluster = logical.find((c) => c.id === fannedClusterId && c.cluster > 1);
  if (!cluster) return null;
  const fanned = withBoardDensity(logical, {
    hoverId: null,
    fannedClusterId,
    raiseId: null,
    viewer: seat.viewer,
    playerCount: seat.playerCount,
  });
  const hit = hitLogicalCard(cam, fanned, sx, sy);
  if (!hit || !cluster.clusterMembers.includes(hit.id)) return null;
  return hit.id;
}

/** Update hover-raise / cluster-fan from a hit on the current density overlay. */
export function densityHoverFromHit(
  logical: readonly RenderCard[],
  hit: RenderCard | null,
  fannedClusterId: number | null,
  selectedId: number | null = null,
): { hoverId: number | null; fannedClusterId: number | null } {
  if (!hit) {
    if (pinsFan(logical, fannedClusterId, selectedId)) {
      return { hoverId: null, fannedClusterId };
    }
    return { hoverId: null, fannedClusterId: null };
  }

  const asCluster = logical.find((c) => c.id === hit.id && c.cluster > 1);
  // Open the fan without raising the face — its full rect would steal peeks of every member.
  if (asCluster) return { hoverId: null, fannedClusterId: asCluster.id };

  if (fannedClusterId != null) {
    const cluster = logical.find((c) => c.id === fannedClusterId);
    if (cluster?.clusterMembers.includes(hit.id)) {
      return { hoverId: hit.id, fannedClusterId };
    }
  }

  if (pinsFan(logical, fannedClusterId, selectedId)) {
    return { hoverId: hit.id, fannedClusterId };
  }

  return { hoverId: hit.id, fannedClusterId: null };
}

export function useTableSurface(deps: TableSurfaceDeps): TableSurface {
  const initial = deps.initialSize ?? defaultSize();
  const [camera, setCamera] = createSignal<Camera>(fitCamera(initial, deps.playerCount(), deps.handBarH));
  const [size, setSize] = createSignal<Vec>(initial);
  // True once the player has manually panned or zoomed — from then on, auto-fit (on window
  // resize or the real player count arriving) never stomps their chosen view.
  const [userMoved, setUserMoved] = createSignal(false);

  const [hoverId, setHoverId] = createSignal<number | null>(null);
  const [fannedClusterId, setFannedClusterId] = createSignal<number | null>(null);
  const selectedId = () => (deps.selectedId ? deps.selectedId() : null);

  /** Hover fan, or the cluster that owns the current selection (keeps non-face members addressable). */
  const effectiveFanId = createMemo(() => {
    const sel = selectedId();
    if (sel != null) {
      const face = deps.cards().find((c) => c.cluster > 1 && (c.id === sel || c.clusterMembers.includes(sel)));
      if (face) return face.id;
    }
    return fannedClusterId();
  });

  const interactiveCards = createMemo(() =>
    withBoardDensity(deps.cards(), {
      hoverId: hoverId(),
      fannedClusterId: effectiveFanId(),
      raiseId: selectedId() ?? hoverId(),
      viewer: deps.me(),
      playerCount: deps.playerCount(),
    }),
  );

  createEffect(() => {
    const s = size();
    const count = deps.playerCount();
    if (userMoved()) return;
    setCamera(fitCamera(s, count, deps.handBarH));
  });

  const pan = (dx: number, dy: number) => {
    setUserMoved(true);
    setCamera((c) => panBy(c, dx, dy));
  };

  const zoomAt = (sx: number, sy: number, factor: number) => {
    setUserMoved(true);
    setCamera((c) => zoomCameraAt(c, sx, sy, factor));
  };

  const hitCard = (sx: number, sy: number): RenderCard | null => hitLogicalCard(camera(), interactiveCards(), sx, sy);

  const hitSeat = (sx: number, sy: number, seats: readonly number[]): number | null =>
    hitAvatar(camera(), sx, sy, avatarWorldFor(seats, deps.me(), deps.playerCount()));

  const applyDensityHover = (sx: number, sy: number) => {
    const probe = withBoardDensity(deps.cards(), {
      hoverId: null,
      fannedClusterId: effectiveFanId(),
      raiseId: selectedId(),
      viewer: deps.me(),
      playerCount: deps.playerCount(),
    });
    const hit = hitLogicalCard(camera(), probe, sx, sy);
    const next = densityHoverFromHit(deps.cards(), hit, effectiveFanId(), selectedId());
    let hover = next.hoverId;
    // Opening a cluster returns hoverId null so the face isn't raised over peeks — resolve the
    // member under the pointer on the newly fanned list so the right card is on top immediately.
    if (next.fannedClusterId != null && hover == null && selectedId() == null) {
      hover = fanMemberAt(camera(), deps.cards(), next.fannedClusterId, sx, sy, {
        viewer: deps.me(),
        playerCount: deps.playerCount(),
      });
    }
    setHoverId(hover);
    setFannedClusterId(next.fannedClusterId);
  };

  // ── Pointer FSM ───────────────────────────────────────────────────────────────────
  let phase: PointerPhase = { kind: "idle" };
  let pressedSeat: number | null = null;
  const [dragging, setDragging] = createSignal<RenderCard | null>(null);
  let lastPointer: Vec = { x: 0, y: 0 };
  let longPressTimer: ReturnType<typeof setTimeout> | null = null;
  let suppressClickAfterFan = false;

  const clearLongPress = () => {
    if (longPressTimer != null) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  };

  const notePointer = (sx: number, sy: number) => {
    lastPointer = { x: sx, y: sy };
  };

  const pointerDown = (sx: number, sy: number, ctx: PointerCtx) => {
    notePointer(sx, sy);
    clearLongPress();
    suppressClickAfterFan = false;
    pressedSeat = hitSeat(sx, sy, ctx.aimSeats);
    if (pressedSeat !== null) {
      phase = { kind: "idle" };
      setDragging(null);
      return;
    }
    const hit = hitCard(sx, sy);
    phase = reduceDown(hit, sx, sy, ctx.combatStep, ctx.me);
    setDragging(phase.kind === "drag" ? phase.card : null);
    // Long-press fans clusters on touch — never arm during a combat drag (would swallow drops).
    if (hit && hit.cluster > 1 && phase.kind !== "drag") {
      const clusterId = hit.id;
      longPressTimer = setTimeout(() => {
        longPressTimer = null;
        setFannedClusterId(clusterId);
        setHoverId(
          fanMemberAt(camera(), deps.cards(), clusterId, lastPointer.x, lastPointer.y, {
            viewer: deps.me(),
            playerCount: deps.playerCount(),
          }),
        );
        suppressClickAfterFan = true;
      }, CLUSTER_LONG_PRESS_MS);
    }
  };

  const pointerMove = (sx: number, sy: number): PointerMoveEffect | null => {
    notePointer(sx, sy);
    const { phase: next, pan: delta } = reduceMove(phase, sx, sy);
    phase = next;
    if (delta) {
      clearLongPress();
      pan(delta.dx, delta.dy);
    }
    const drag = dragging();
    if (drag) {
      clearLongPress();
      suppressClickAfterFan = false;
      return { kind: "drag", card: drag };
    }
    if (phase.kind === "idle" || phase.kind === "press") applyDensityHover(sx, sy);
    return null;
  };

  const pointerUp = (sx: number, sy: number): SurfaceEffect => {
    notePointer(sx, sy);
    clearLongPress();
    setDragging(null);
    if (pressedSeat !== null) {
      const seatNum = pressedSeat;
      pressedSeat = null;
      // Missed release: leave selection alone (pre-C6 Board returned without clearing).
      if (hitSeat(sx, sy, [seatNum]) === seatNum) return { kind: "aim-seat", seat: seatNum };
      return { kind: "none" };
    }
    if (suppressClickAfterFan) {
      suppressClickAfterFan = false;
      phase = { kind: "idle" };
      applyDensityHover(sx, sy);
      return { kind: "none" };
    }
    const release = reduceUp(phase, sx, sy, hitCard(sx, sy));
    phase = { kind: "idle" };
    applyDensityHover(sx, sy);
    if (release.kind === "combat-drop") return { kind: "combat-drop", card: release.card, x: sx, y: sy };
    if (release.kind === "click") return { kind: "click", card: release.card };
    return { kind: "clear-selection" };
  };

  const pointerCancel = () => {
    clearLongPress();
    suppressClickAfterFan = false;
    setDragging(null);
    pressedSeat = null;
    phase = { kind: "idle" };
  };

  // ── Paint-only tween ──────────────────────────────────────────────────────────────
  // No idle rAF: the loop runs only while unsettled, then stops until the layout next changes.
  let anim: Positions = new Map();
  let tapAnim = new Map<number, number>();
  const [animTick, setAnimTick] = createSignal(0);
  let rafId = 0;
  let lastFrame = 0;

  const tapTargets = (cs: RenderCard[]) => new Map(cs.map((c) => [c.id, c.tapped ? 1 : 0]));

  const animFrame = (now: number) => {
    const dt = now - lastFrame;
    lastFrame = now;
    const cs = deps.cards();
    const pos = stepToward(anim, cs, dt);
    anim = pos.positions;
    const tap = stepScalar(tapAnim, tapTargets(cs), dt);
    tapAnim = tap.values;
    setAnimTick((t) => t + 1);
    rafId = pos.settled && tap.settled ? 0 : requestAnimationFrame(animFrame);
  };

  const reducedMotion = () => deps.reducedMotion?.() ?? defaultReducedMotion();
  const zoneMoves = () => deps.zoneMoves?.() ?? new Map<number, number>();
  const fromStack = () => deps.fromStack?.() ?? new Set<number>();
  const fromStackExit = () => deps.fromStackExit?.() ?? new Set<number>();
  const tokenCreators = () => deps.tokenCreators?.() ?? new Map<number, number>();
  const playEntrances = () => deps.playEntrances?.() ?? new Map<number, Vec>();
  const zonePileEntrances = () => deps.zonePileEntrances?.() ?? new Map();
  const stackObjectIds = () => deps.stackObjectIds?.() ?? new Set<number>();
  const stackLength = () => deps.stackLength?.() ?? 0;
  const flightOwnedIds = () => deps.flightOwnedIds?.() ?? new Set<number>();

  createEffect(() => {
    const targets = deps.cards();
    if (reducedMotion()) {
      anim = snapAll(targets);
      tapAnim = tapTargets(targets);
      setAnimTick((t) => t + 1);
      return;
    }
    const entranceOpts = {
      moves: zoneMoves(),
      fromStack: fromStack(),
      fromStackExit: fromStackExit(),
      tokenCreators: tokenCreators(),
      playEntrances: playEntrances(),
      zonePileEntrances: zonePileEntrances(),
      stackObjectIds: stackObjectIds(),
      stackLength: stackLength(),
      size: size(),
      camera: camera(),
      me: deps.me(),
      playerCount: deps.playerCount(),
      skipIds: flightOwnedIds(),
    };
    // First paint: seed provenance into an empty anim, then snap anything left to layout.
    // (A blanket snapAll first would mark ids present and skip play/stack entrances.)
    if (anim.size === 0) {
      seedEntrances(anim, targets, entranceOpts);
      for (const c of targets) {
        if (!anim.has(c.id)) anim.set(c.id, { x: c.x, y: c.y });
      }
      tapAnim = tapTargets(targets);
    } else {
      seedEntrances(anim, targets, entranceOpts);
    }
    // Paint immediately at entrance seeds — don't wait for the first rAF tick.
    setAnimTick((t) => t + 1);
    if (rafId) return;
    lastFrame = performance.now();
    rafId = requestAnimationFrame(animFrame);
  });
  onCleanup(() => {
    clearLongPress();
    if (rafId && typeof cancelAnimationFrame !== "undefined") cancelAnimationFrame(rafId);
  });

  const drawnCards = createMemo(() => {
    animTick();
    hoverId();
    fannedClusterId();
    effectiveFanId();
    selectedId();
    const logical = deps.cards();
    return interactiveCards().map((c) => {
      const tf = tapAnim.get(c.id);
      const rotating = tf !== undefined && tf !== (c.tapped ? 1 : 0);
      const layoutCard = logical.find((l) => l.id === c.id);
      // Fan (and any density x/y shift) wins over tween — don't snap fanned members back.
      const densityMoved =
        !layoutCard || layoutCard.x !== c.x || layoutCard.y !== c.y || (layoutCard.cluster > 1 && c.cluster === 0);
      if (densityMoved) return rotating ? { ...c, tapFrac: tf } : c;
      const p = anim.get(c.id);
      const moved = p && (p.x !== c.x || p.y !== c.y);
      if (!moved && !rotating) return c;
      return { ...c, ...(moved && p ? { x: p.x, y: p.y } : {}), tapFrac: tf };
    });
  });

  // ── Aux inspect ───────────────────────────────────────────────────────────────────
  const [inspectPin, setInspectPin] = createSignal<InspectPin | null>(null);
  let handHover: { name: string; cardId?: string; print?: string } | null = null;
  let stackHover: { name: string; cardId?: string; print?: string } | null = null;

  const setAuxHover = (source: "hand" | "stack", card: { name: string; cardId?: string; print?: string } | null) => {
    if (source === "hand") handHover = card;
    else stackHover = card;
  };

  const tryPinInspect = (): InspectPin | null => {
    // Hand/stack DOM overlays sit above the canvas — prefer their hover over a battlefield hit.
    const aux = handHover ?? stackHover;
    if (aux) {
      const pin: InspectPin = {
        name: aux.name,
        prepared: false,
        ...(aux.cardId ? { cardId: aux.cardId } : {}),
        ...(aux.print ? { print: aux.print } : {}),
      };
      setInspectPin(pin);
      return pin;
    }
    const card = hitCard(lastPointer.x, lastPointer.y);
    const fromBoard = pinFromHit(
      true,
      card
        ? {
            name: card.name,
            faceDown: card.faceDown,
            prepared: card.prepared,
            id: card.id,
            zone: card.zone,
            cardId: card.cardId,
            print: card.print,
          }
        : null,
      ZONE.Battlefield,
    );
    if (fromBoard) {
      setInspectPin(fromBoard);
      return fromBoard;
    }
    return null;
  };

  const clearInspect = () => setInspectPin(null);

  return {
    camera,
    size,
    setSize,
    pan,
    zoomAt,
    hitCard,
    hitSeat,
    pointerDown,
    pointerMove,
    pointerUp,
    pointerCancel,
    dragging,
    drawnCards,
    notePointer,
    setAuxHover,
    tryPinInspect,
    clearInspect,
    inspectPin,
  };
}
