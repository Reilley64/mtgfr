import { Effect, type Queue as EffectQueue, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import { colors } from "~/design-tokens.generated";
import type { ActionView, PlayerView, VisibleState, WireAttack, WireBlock } from "~/wire/types";
import { cardBackUrl, imageUrlByPrint } from "../../../lib/deck-builder/scryfall";
import { type ImageCache, sharedImageCache } from "../../../lib/image-cache";
import type { Vec } from "../action/targeting";
import { TARGET_COLOR } from "../action/targeting";
import { maxCommanderDamage } from "../canvas/avatars";
import { PLAYABLE_BORDER, playableBattlefieldObjectIds } from "../chrome";
import { type Camera, worldToScreen } from "../geometry/camera";
import { AVATAR_R, avatarPos, type RenderCard, seatColor } from "../geometry/layout";
import { ArtLoaded, FlightsSynced } from "../messages";
import { type CardFlight, stepFlights } from "../motion/flights";
import { mergeFlightPoses, restingPaintChanged, restingPaintSnapshot } from "./flight-frame";
import {
  paintAutoTapPreview,
  paintCard,
  paintCardAssignAmount,
  paintCardPickedHighlight,
  paintCardTargetHighlight,
} from "./paint-cards";
import { paintFlightCard } from "./paint-flights";

export type BitmapFrame = {
  width: number;
  height: number;
  camera: Camera;
  cards: readonly RenderCard[];
  viewer: number;
  players: readonly PlayerView[];
  priority: number;
  combat: VisibleState["combat"];
  /** Attackers/blocks declared during the current staging session but not yet committed. */
  stagedAttackers: readonly WireAttack[];
  stagedBlocks: readonly WireBlock[];
  flights: readonly CardFlight[];
  hideCardIds: ReadonlySet<number>;
  targetObjects: ReadonlySet<number>;
  /** Multi-aim picks already toggled in the pending draft (Priority Gold solid ring). */
  pickedObjects: ReadonlySet<number>;
  /** Combat-damage assign draft amounts keyed by blocker object id. */
  assignAmounts: ReadonlyMap<number, number>;
  targetPlayers: ReadonlySet<number>;
  aimFrom: Vec | null;
  cursor: Vec;
  combatDragFrom: Vec | null;
  combatDragStroke: string | null;
  paymentPreviewIds: ReadonlySet<number>;
  actions?: readonly ActionView[];
};

export type FlightClockState = {
  liveFlights: CardFlight[];
  lastRestingSnapshot: ReturnType<typeof restingPaintSnapshot> | null;
};

type LayerQueue = EffectQueue.Enqueue<typeof ArtLoaded.Type | typeof FlightsSynced.Type>;

let currentFrame: BitmapFrame | null = null;
let flightClockState: FlightClockState = {
  liveFlights: [],
  lastRestingSnapshot: null,
};
const mountedLayers = new Set<BitmapMountHandle>();

type BitmapMountHandle = {
  canvas: HTMLCanvasElement;
  /** Paints this layer's slice of the current frame onto its canvas. */
  render: (canvas: HTMLCanvasElement) => void;
  /** Only the flight layer self-animates; the permanents/arrows layer repaints on publish. */
  animates: boolean;
  unsubscribe: () => void;
  rafId: number;
  lastFlightTick: number | null;
  kickRaf: () => void;
};

export function publishBitmapFrame(frame: BitmapFrame): void {
  const published = applyPublishedFrame(flightClockState, frame);
  flightClockState = published.state;
  currentFrame = published.frame;
  preloadFrameArt(published.frame, sharedImageCache);
  for (const handle of mountedLayers) {
    if (handle.animates) {
      if (published.paintFlight) handle.render(handle.canvas);
      handle.kickRaf();
      continue;
    }
    if (published.paintResting) handle.render(handle.canvas);
  }
}

export function applyPublishedFrame(
  state: FlightClockState,
  frame: BitmapFrame,
): { state: FlightClockState; paintResting: boolean; paintFlight: boolean; frame: BitmapFrame } {
  const liveFlights = mergeFlightPoses(state.liveFlights, frame.flights);
  const mergedFrame = { ...frame, flights: liveFlights };
  const { flights: _flights, ...restingFrame } = mergedFrame;
  const nextRestingSnapshot = restingPaintSnapshot(restingFrame);

  return {
    state: {
      liveFlights,
      lastRestingSnapshot: nextRestingSnapshot,
    },
    paintResting: restingPaintChanged(state.lastRestingSnapshot, nextRestingSnapshot),
    paintFlight: state.lastRestingSnapshot == null || flightsChanged(state.liveFlights, liveFlights),
    frame: mergedFrame,
  };
}

export function tickFlightClock(
  state: FlightClockState,
  frame: BitmapFrame,
  now: number,
  dtMs: number,
  reducedMotion: boolean,
): {
  state: FlightClockState;
  frame: BitmapFrame;
  paintFlight: boolean;
  sync: { flights: CardFlight[]; now: number } | null;
} {
  const stepped = stepFlights(new Map(state.liveFlights.map((flight) => [flight.id, flight])), dtMs, reducedMotion);
  const liveFlights = [...stepped.flights.values()];
  const prevFlyingIds = flyingIds(state.liveFlights);
  const nextFlyingIds = flyingIds(liveFlights);
  const flyingMembershipChanged = !sameIdSet(prevFlyingIds, nextFlyingIds);
  const allSettled = prevFlyingIds.size > 0 && nextFlyingIds.size === 0;

  return {
    state: {
      ...state,
      liveFlights,
    },
    frame: { ...frame, flights: liveFlights },
    paintFlight: true,
    sync: flyingMembershipChanged || allSettled ? { flights: liveFlights, now } : null,
  };
}

function flightsChanged(prev: readonly CardFlight[], next: readonly CardFlight[]): boolean {
  if (prev.length !== next.length) return true;

  for (let index = 0; index < prev.length; index += 1) {
    const before = prev[index];
    const after = next[index];
    if (before == null || after == null) return true;
    if (
      before.id !== after.id ||
      before.print !== after.print ||
      before.name !== after.name ||
      before.targetX !== after.targetX ||
      before.targetY !== after.targetY ||
      before.targetScale !== after.targetScale ||
      before.phase !== after.phase ||
      before.kind !== after.kind ||
      before.fromCardId !== after.fromCardId
    ) {
      return true;
    }
  }

  return false;
}

function flyingIds(flights: readonly CardFlight[]): Set<number> {
  return new Set(flights.filter((flight) => flight.phase === "flying").map((flight) => flight.id));
}

function sameIdSet(a: ReadonlySet<number>, b: ReadonlySet<number>): boolean {
  if (a.size !== b.size) return false;
  for (const id of a) {
    if (!b.has(id)) return false;
  }
  return true;
}

function resetClockState(): void {
  currentFrame = null;
  flightClockState = {
    liveFlights: [],
    lastRestingSnapshot: null,
  };
}

export function bitmapFrameNeedsRaf(frame: Pick<BitmapFrame, "flights"> | null): boolean {
  return frame?.flights.some((flight) => flight.phase === "flying") ?? false;
}

/** Size the backing store to the DPR, reset the transform, and clear. Returns the 2D context. */
function prepareLayerCtx(canvas: HTMLCanvasElement, frame: BitmapFrame): CanvasRenderingContext2D | null {
  const dpr = window.devicePixelRatio || 1;
  const targetWidth = Math.max(1, Math.floor(frame.width * dpr));
  const targetHeight = Math.max(1, Math.floor(frame.height * dpr));
  if (canvas.width !== targetWidth) canvas.width = targetWidth;
  if (canvas.height !== targetHeight) canvas.height = targetHeight;

  const ctx = canvas.getContext("2d");
  if (ctx == null) return null;

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, frame.width, frame.height);
  return ctx;
}

/** Layer 3 + 4: resting permanents with card chrome, then avatars and arrows on top. No flights. */
export function paintBitmapLayer(canvas: HTMLCanvasElement, frame: BitmapFrame, cache: Pick<ImageCache, "get">): void {
  const ctx = prepareLayerCtx(canvas, frame);
  if (ctx == null) return;

  const playableObjects = playableBattlefieldObjectIds(
    frame.actions,
    frame.cards.map((card) => ({
      id: card.id,
      summoningSick: card.summoningSick,
      hasHaste: card.hasHaste,
    })),
  );
  for (const card of frame.cards) {
    if (frame.hideCardIds.has(card.id)) continue;
    const outline = playableObjects.has(card.id) ? { color: PLAYABLE_BORDER, dash: [] } : null;
    paintCard(ctx, frame.camera, card, cache, frame.viewer, { outline });
    if (frame.paymentPreviewIds.has(card.id)) {
      paintAutoTapPreview(ctx, frame.camera, card, frame.viewer);
    }
    if (frame.pickedObjects.has(card.id)) {
      paintCardPickedHighlight(ctx, frame.camera, card, frame.viewer);
    } else if (frame.targetObjects.has(card.id)) {
      paintCardTargetHighlight(ctx, frame.camera, card, frame.viewer);
    }
    const assignAmount = frame.assignAmounts.get(card.id) ?? 0;
    if (assignAmount > 0) {
      paintCardAssignAmount(ctx, frame.camera, card, frame.viewer, assignAmount);
    }
  }

  paintAvatars(ctx, frame);
  paintCombatArrows(ctx, frame);
  paintStagingAimArrow(ctx, frame);
  if (frame.combatDragFrom != null && frame.combatDragStroke != null) {
    paintArrow(ctx, frame.combatDragFrom, frame.cursor, frame.combatDragStroke);
  }
}

/** Layer 6: in-flight cards only, on a canvas above the hand/stack HTML. */
export function paintFlightLayer(canvas: HTMLCanvasElement, frame: BitmapFrame, cache: Pick<ImageCache, "get">): void {
  const ctx = prepareLayerCtx(canvas, frame);
  if (ctx == null) return;

  for (const flight of frame.flights) {
    paintFlightCard(ctx, flight, frame.camera.zoom, cache);
  }
}

function renderBoardLayer(canvas: HTMLCanvasElement): void {
  if (currentFrame == null) return;
  paintBitmapLayer(canvas, currentFrame, sharedImageCache);
}

function renderFlightLayer(canvas: HTMLCanvasElement): void {
  if (currentFrame == null) return;
  paintFlightLayer(canvas, currentFrame, sharedImageCache);
}

function registerLayer(
  element: unknown,
  render: (canvas: HTMLCanvasElement) => void,
  animates: boolean,
  queue: LayerQueue,
): BitmapMountHandle | null {
  if (!(element instanceof HTMLCanvasElement)) return null;

  let handle: BitmapMountHandle | null = null;
  const unsubscribe = sharedImageCache.subscribe(() => {
    Queue.offerUnsafe(queue, ArtLoaded());
    if (handle != null) render(handle.canvas);
    handle?.kickRaf();
  });
  const frame = (now: number): void => {
    if (handle == null || currentFrame == null) return;
    handle.rafId = 0;
    const dtMs = handle.lastFlightTick == null ? 16 : Math.max(0, now - handle.lastFlightTick);
    handle.lastFlightTick = now;
    const tick = tickFlightClock(flightClockState, currentFrame, now, dtMs, prefersReducedMotion());
    flightClockState = tick.state;
    currentFrame = tick.frame;
    if (tick.paintFlight) render(handle.canvas);
    if (tick.sync != null) Queue.offerUnsafe(queue, FlightsSynced(tick.sync));
    handle.kickRaf();
  };
  const kickRaf = (): void => {
    if (handle == null) return;
    if (!animates) return;
    if (handle.rafId !== 0) return;
    if (!bitmapFrameNeedsRaf(currentFrame)) {
      handle.lastFlightTick = null;
      return;
    }
    handle.rafId = requestAnimationFrame(frame);
  };
  handle = { canvas: element, render, animates, unsubscribe, rafId: 0, lastFlightTick: null, kickRaf };
  mountedLayers.add(handle);
  render(handle.canvas);
  kickRaf();

  return handle;
}

function releaseLayer(handle: BitmapMountHandle | null): void {
  if (handle == null) return;
  mountedLayers.delete(handle);
  handle.unsubscribe();
  if (handle.rafId !== 0) cancelAnimationFrame(handle.rafId);
  if (mountedLayers.size === 0) resetClockState();
}

function defineLayerMount(name: string, render: (canvas: HTMLCanvasElement) => void, animates: boolean) {
  return Mount.defineStream(
    name,
    ArtLoaded,
    FlightsSynced,
  )((element) =>
    Stream.callback<typeof ArtLoaded.Type | typeof FlightsSynced.Type>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => registerLayer(element, render, animates, queue)),
          (handle) => Effect.sync(() => releaseLayer(handle)),
        );

        return yield* Effect.never;
      }),
    ),
  );
}

export const MountBitmapLayer = defineLayerMount("MountBitmapLayer", renderBoardLayer, false);
export const MountFlightLayer = defineLayerMount("MountFlightLayer", renderFlightLayer, true);

function paintAvatars(ctx: CanvasRenderingContext2D, frame: BitmapFrame): void {
  const count = Math.max(1, frame.players.length);
  const radius = AVATAR_R * frame.camera.zoom;

  for (const player of frame.players) {
    const pos = avatarPos(player.player, frame.viewer, count);
    const screen = worldToScreen(frame.camera, pos.x, pos.y);
    const stroke = frame.priority === player.player ? colors.priorityGold : seatColor(player.player, 0.9);

    ctx.save();
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, radius, 0, Math.PI * 2);
    ctx.fillStyle = player.lost ? "rgba(14,26,20,0.5)" : "rgba(14,26,20,0.95)";
    ctx.fill();
    ctx.strokeStyle = stroke;
    ctx.lineWidth = frame.priority === player.player ? 4 : 2;
    ctx.stroke();

    if (frame.targetPlayers.has(player.player)) {
      ctx.beginPath();
      ctx.arc(screen.x, screen.y, radius + 5, 0, Math.PI * 2);
      ctx.strokeStyle = TARGET_COLOR;
      ctx.lineWidth = 3;
      ctx.setLineDash([2, 6]);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    ctx.fillStyle = "#eff";
    ctx.font = `700 ${Math.max(1, Math.round(30 * frame.camera.zoom))}px system-ui, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(`${player.life}`, screen.x, screen.y + 4 * frame.camera.zoom);

    ctx.fillStyle = "#9cb";
    ctx.font = `${Math.max(1, Math.round(14 * frame.camera.zoom))}px system-ui, sans-serif`;
    ctx.fillText(player.username?.trim() || `P${player.player}`, screen.x, screen.y + 27 * frame.camera.zoom);

    ctx.fillStyle = "#89a";
    ctx.font = `${Math.max(1, Math.round(12 * frame.camera.zoom))}px system-ui, sans-serif`;
    ctx.fillText(`Hand ${player.hand_count}`, screen.x, screen.y - 29 * frame.camera.zoom);

    const cmd = maxCommanderDamage(player);
    if (cmd > 0) {
      ctx.fillStyle = "#db8664";
      ctx.font = `${Math.max(1, Math.round(12 * frame.camera.zoom))}px system-ui, sans-serif`;
      ctx.fillText(`Cmd ${cmd}`, screen.x, screen.y + 42 * frame.camera.zoom);
    }
    ctx.restore();
  }
}

function paintCombatArrows(ctx: CanvasRenderingContext2D, frame: BitmapFrame): void {
  const cardsById = new Map(frame.cards.map((card) => [card.id, card]));
  const avatars = new Map<number, { x: number; y: number }>();
  const count = Math.max(1, frame.players.length);
  for (const player of frame.players) {
    const pos = avatarPos(player.player, frame.viewer, count);
    avatars.set(player.player, worldToScreen(frame.camera, pos.x, pos.y));
  }

  // Declare-drag staging arrows share the arrow layer with committed arrows (canvas map layer 4).
  for (const attack of [...frame.stagedAttackers, ...frame.combat.attackers]) {
    const from = cardsById.get(attack.attacker);
    const to = avatars.get(attack.defender);
    if (from == null || to == null) continue;
    // Attack stroke matches arrows.ts ATTACK_STROKE (not colors.mountainRed).
    paintArrow(ctx, cardCenter(frame.camera, from), to, "#ff6b6b");
  }

  for (const block of [...frame.stagedBlocks, ...frame.combat.blocks]) {
    const from = cardsById.get(block.blocker);
    const to = cardsById.get(block.attacker);
    if (from == null || to == null) continue;
    paintArrow(ctx, cardCenter(frame.camera, from), cardCenter(frame.camera, to), colors.wallGreen);
  }
}

function paintStagingAimArrow(ctx: CanvasRenderingContext2D, frame: BitmapFrame): void {
  if (frame.aimFrom == null) return;
  paintArrow(ctx, frame.aimFrom, frame.cursor, TARGET_COLOR, [2, 6]);
}

function cardCenter(camera: Camera, card: RenderCard): { x: number; y: number } {
  return worldToScreen(camera, card.x + card.w / 2, card.y + card.h / 2);
}

function paintArrow(
  ctx: CanvasRenderingContext2D,
  from: { x: number; y: number },
  to: { x: number; y: number },
  stroke: string,
  dash: readonly number[] = [],
): void {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const len = Math.hypot(dx, dy) || 1;
  const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
  const control = {
    x: mid.x - (dy / len) * Math.min(48, len * 0.22),
    y: mid.y + (dx / len) * Math.min(48, len * 0.22),
  };
  const angle = Math.atan2(to.y - control.y, to.x - control.x);

  ctx.save();
  ctx.beginPath();
  ctx.moveTo(from.x, from.y);
  ctx.quadraticCurveTo(control.x, control.y, to.x, to.y);
  ctx.strokeStyle = stroke;
  ctx.lineWidth = 3;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.setLineDash([...dash]);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.beginPath();
  ctx.moveTo(to.x, to.y);
  ctx.lineTo(to.x - 13 * Math.cos(angle - 0.4), to.y - 13 * Math.sin(angle - 0.4));
  ctx.lineTo(to.x - 13 * Math.cos(angle + 0.4), to.y - 13 * Math.sin(angle + 0.4));
  ctx.closePath();
  ctx.fillStyle = stroke;
  ctx.fill();
  ctx.restore();
}

function preloadFrameArt(frame: BitmapFrame, cache: Pick<ImageCache, "preload">): void {
  const urls: string[] = [];
  for (const card of frame.cards) {
    urls.push(card.faceDown ? cardBackUrl() : imageUrlByPrint(card.print));
  }
  for (const flight of frame.flights) {
    if (flight.print) urls.push(imageUrlByPrint(flight.print));
  }
  cache.preload(urls);
}

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}
