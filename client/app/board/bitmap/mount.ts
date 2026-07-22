import { Effect, type Queue as EffectQueue, Queue, Stream } from "effect";
import * as Mount from "foldkit/mount";
import type { PlayerView, VisibleState, WireAttack, WireBlock } from "~/wire/types";
import { cardBackUrl, imageUrlByPrint } from "../../../lib/deck-builder/scryfall";
import { type ImageCache, sharedImageCache } from "../../../lib/image-cache";
import type { Vec } from "../action/targeting";
import { TARGET_COLOR } from "../action/targeting";
import { type Camera, worldToScreen } from "../geometry/camera";
import { AVATAR_R, avatarPos, type RenderCard, seatColor } from "../geometry/layout";
import { ArtLoaded, TickedFrame } from "../messages";
import type { CardFlight } from "../motion/flights";
import { paintAutoTapPreview, paintCard, paintCardTargetHighlight } from "./paint-cards";
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
  targetPlayers: ReadonlySet<number>;
  aimFrom: Vec | null;
  cursor: Vec;
  combatDragFrom: Vec | null;
  combatDragStroke: string | null;
  paymentPreviewIds: ReadonlySet<number>;
};

type LayerQueue = EffectQueue.Enqueue<typeof ArtLoaded.Type | typeof TickedFrame.Type>;

let currentFrame: BitmapFrame | null = null;
const mountedLayers = new Set<BitmapMountHandle>();

type BitmapMountHandle = {
  canvas: HTMLCanvasElement;
  /** Paints this layer's slice of the current frame onto its canvas. */
  render: (canvas: HTMLCanvasElement) => void;
  /** Only the flight layer self-animates; the permanents/arrows layer repaints on publish. */
  animates: boolean;
  unsubscribe: () => void;
  rafId: number;
  kickRaf: () => void;
};

export function publishBitmapFrame(frame: BitmapFrame): void {
  currentFrame = frame;
  preloadFrameArt(frame, sharedImageCache);
  for (const handle of mountedLayers) {
    handle.render(handle.canvas);
    handle.kickRaf();
  }
}

export function bitmapFrameNeedsRaf(frame: Pick<BitmapFrame, "flights"> | null): boolean {
  return (frame?.flights.length ?? 0) > 0;
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

  for (const card of frame.cards) {
    if (frame.hideCardIds.has(card.id)) continue;
    paintCard(ctx, frame.camera, card, cache, frame.viewer);
    if (frame.paymentPreviewIds.has(card.id)) {
      paintAutoTapPreview(ctx, frame.camera, card, frame.viewer);
    }
    if (frame.targetObjects.has(card.id)) {
      paintCardTargetHighlight(ctx, frame.camera, card, frame.viewer);
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
    if (handle == null) return;
    handle.rafId = 0;
    Queue.offerUnsafe(queue, TickedFrame({ now, reducedMotion: prefersReducedMotion() }));
    render(handle.canvas);
    handle.kickRaf();
  };
  const kickRaf = (): void => {
    if (handle == null) return;
    if (!animates) return;
    if (handle.rafId !== 0) return;
    if (!bitmapFrameNeedsRaf(currentFrame)) return;
    handle.rafId = requestAnimationFrame(frame);
  };
  handle = { canvas: element, render, animates, unsubscribe, rafId: 0, kickRaf };
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
}

function defineLayerMount(name: string, render: (canvas: HTMLCanvasElement) => void, animates: boolean) {
  return Mount.defineStream(
    name,
    ArtLoaded,
    TickedFrame,
  )((element) =>
    Stream.callback<typeof ArtLoaded.Type | typeof TickedFrame.Type>((queue) =>
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
    const stroke = frame.priority === player.player ? "#ffd76a" : seatColor(player.player, 0.9);

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
    paintArrow(ctx, cardCenter(frame.camera, from), to, "#ff6b6b");
  }

  for (const block of [...frame.stagedBlocks, ...frame.combat.blocks]) {
    const from = cardsById.get(block.blocker);
    const to = cardsById.get(block.attacker);
    if (from == null || to == null) continue;
    paintArrow(ctx, cardCenter(frame.camera, from), cardCenter(frame.camera, to), "#66ff99");
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
