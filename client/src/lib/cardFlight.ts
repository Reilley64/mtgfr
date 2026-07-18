// Canvas flight layer (ADR 0035): screen-space pose + scale for a card in transit between
// resting surfaces (hand/stack DOM ↔ battlefield canvas). Same exponential ease as tween.ts
// (~150–200ms settle). Pure and clock-free — the caller passes dt and reduced-motion.

import { CARD_W } from "~/layout";
import { STACK_CARD_W } from "~/lib/boardDraw";

/** Hand bar card face width (px) — must match `hand.tsx` CARD_FACE. */
export const HAND_FACE_W = 112;

/** Exponential time constant — shared feel with board position tweens. */
const TAU_MS = 75;
/** Snap-to-target distance in screen px. */
const EPSILON_PX = 0.5;
/** Snap-to-target scale delta. */
const EPSILON_SCALE = 0.02;

export type FlightPhase = "flying" | "settled";

export type FlightKind = "battlefield" | "stack" | "from-stack";

/** One in-flight card, tracked in screen space (center). Scale is relative to canvas card width
 * (`CARD_W * zoom`): 1 = same size as a battlefield card at the current zoom. */
export interface CardFlight {
  id: number;
  print: string;
  name: string;
  x: number;
  y: number;
  scale: number;
  targetX: number;
  targetY: number;
  targetScale: number;
  phase: FlightPhase;
  kind: FlightKind;
  /** Hand/command object id this flight was spawned from (for delta binding). */
  fromCardId?: number;
}

export type FlightSpawn = {
  id: number;
  print: string;
  name: string;
  x: number;
  y: number;
  scale: number;
  targetX: number;
  targetY: number;
  targetScale: number;
  kind: FlightKind;
  fromCardId?: number;
};

export function spawnFlight(s: FlightSpawn): CardFlight {
  return {
    id: s.id,
    print: s.print,
    name: s.name,
    x: s.x,
    y: s.y,
    scale: s.scale,
    targetX: s.targetX,
    targetY: s.targetY,
    targetScale: s.targetScale,
    phase: "flying",
    kind: s.kind,
    fromCardId: s.fromCardId,
  };
}

export function flightSettled(f: CardFlight): boolean {
  return f.phase === "settled";
}

/** Scale so a canvas card drawn at this scale matches the hand face width. */
export function handFlightScale(zoom: number): number {
  const canvasW = CARD_W * Math.max(zoom, 0.01);
  return HAND_FACE_W / canvasW;
}

/** Scale so a canvas card matches the stack overlay face width. */
export function stackFlightScale(zoom: number): number {
  const canvasW = CARD_W * Math.max(zoom, 0.01);
  return STACK_CARD_W / canvasW;
}

/**
 * Advance every flight toward its target. `reducedMotion` snaps all to target in one step.
 * Flights absent from the input map are dropped (caller removes settled flights when done).
 */
export function stepFlights(
  prev: Map<number, CardFlight>,
  dtMs: number,
  reducedMotion: boolean,
): { flights: Map<number, CardFlight>; settled: boolean } {
  const flights = new Map<number, CardFlight>();
  let allSettled = true;
  const alpha = 1 - Math.exp(-dtMs / TAU_MS);

  for (const [id, cur] of prev) {
    if (reducedMotion || alreadyAtTarget(cur)) {
      flights.set(id, snapToTarget(cur));
      continue;
    }
    const x = cur.x + (cur.targetX - cur.x) * alpha;
    const y = cur.y + (cur.targetY - cur.y) * alpha;
    const scale = cur.scale + (cur.targetScale - cur.scale) * alpha;
    const next: CardFlight = { ...cur, x, y, scale, phase: "flying" };
    if (alreadyAtTarget(next)) {
      flights.set(id, snapToTarget(next));
      continue;
    }
    flights.set(id, next);
    allSettled = false;
  }

  return { flights, settled: allSettled || flights.size === 0 };
}

function alreadyAtTarget(f: CardFlight): boolean {
  return (
    Math.hypot(f.targetX - f.x, f.targetY - f.y) <= EPSILON_PX && Math.abs(f.targetScale - f.scale) <= EPSILON_SCALE
  );
}

function snapToTarget(f: CardFlight): CardFlight {
  return {
    ...f,
    x: f.targetX,
    y: f.targetY,
    scale: f.targetScale,
    phase: "settled",
  };
}

/** Remap a flight's identity when the engine assigns a new object id (hand card → permanent). */
export function rebindFlightId(
  flights: Map<number, CardFlight>,
  fromId: number,
  toId: number,
): Map<number, CardFlight> {
  const f = flights.get(fromId);
  if (!f) return flights;
  const next = new Map(flights);
  next.delete(fromId);
  next.set(toId, { ...f, id: toId });
  return next;
}

/** Update target pose for an in-flight card (layout/camera moved, or destination known). */
export function retargetFlight(f: CardFlight, target: { x: number; y: number; scale: number }): CardFlight {
  return {
    ...f,
    targetX: target.x,
    targetY: target.y,
    targetScale: target.scale,
    phase: "flying",
  };
}
