import { CARD_W } from "../geometry/layout";
import { STACK_CARD_W } from "../geometry/stackLayout";

export const HAND_FACE_W = 208;
/** Re-export resting stack face width so flight scale stays coupled to the HTML stack. */
export { STACK_CARD_W };

const TAU_MS = 75;
const EPSILON_PX = 0.5;
const EPSILON_SCALE = 0.02;
/** Authority handoff radius — near enough that a correction ease would read as a second short glide. */
export const FLIGHT_HANDOFF_PX = 72;
export const FLIGHT_HANDOFF_SCALE = 0.25;

export type FlightPhase = "flying" | "settled";
export type FlightKind = "battlefield" | "stack" | "from-stack";

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
  fromCardId?: number;
  /**
   * Local seed awaiting authoritative rebind/retarget. Settles at the aim pose but stays in the
   * flight set so game sync can continue the same flight instead of spawning a second one.
   */
  hold?: boolean;
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
  hold?: boolean;
};

export type FlightStepResult = {
  flights: Map<number, CardFlight>;
  settled: boolean;
};

export function spawnFlight(spawn: FlightSpawn): CardFlight {
  return {
    id: spawn.id,
    print: spawn.print,
    name: spawn.name,
    x: spawn.x,
    y: spawn.y,
    scale: spawn.scale,
    targetX: spawn.targetX,
    targetY: spawn.targetY,
    targetScale: spawn.targetScale,
    phase: "flying",
    kind: spawn.kind,
    fromCardId: spawn.fromCardId,
    hold: spawn.hold,
  };
}

export function flightSettled(flight: CardFlight): boolean {
  return flight.phase === "settled";
}

export function flightOwnsId(flight: CardFlight): boolean {
  return flight.phase === "flying" || flight.hold === true;
}

export function handFlightScale(zoom: number): number {
  return HAND_FACE_W / (CARD_W * Math.max(zoom, 0.01));
}

export function stackFlightScale(zoom: number): number {
  return STACK_CARD_W / (CARD_W * Math.max(zoom, 0.01));
}

export function stepFlights(
  prev: ReadonlyMap<number, CardFlight>,
  dtMs: number,
  reducedMotion: boolean,
): FlightStepResult {
  const flights = new Map<number, CardFlight>();
  let allSettled = true;
  const alpha = 1 - Math.exp(-dtMs / TAU_MS);

  for (const [id, cur] of prev) {
    if (reducedMotion || alreadyAtTarget(cur)) {
      flights.set(id, snapToTarget(cur));
      continue;
    }

    const next = {
      ...cur,
      x: cur.x + (cur.targetX - cur.x) * alpha,
      y: cur.y + (cur.targetY - cur.y) * alpha,
      scale: cur.scale + (cur.targetScale - cur.scale) * alpha,
      phase: "flying" as const,
    };

    if (alreadyAtTarget(next)) {
      flights.set(id, snapToTarget(next));
      continue;
    }

    flights.set(id, next);
    allSettled = false;
  }

  return { flights, settled: allSettled || flights.size === 0 };
}

export function rebindFlightId(
  flights: ReadonlyMap<number, CardFlight>,
  fromId: number,
  toId: number,
): Map<number, CardFlight> {
  const flight = flights.get(fromId);
  if (!flight) return new Map(flights);

  const next = new Map(flights);
  next.delete(fromId);
  next.set(toId, { ...flight, id: toId });
  return next;
}

export function retargetFlight(flight: CardFlight, target: { x: number; y: number; scale: number }): CardFlight {
  return {
    ...flight,
    targetX: target.x,
    targetY: target.y,
    targetScale: target.scale,
    phase: "flying",
    hold: false,
  };
}

export function flyingCardIds(flights: ReadonlyMap<number, CardFlight>): Set<number> {
  const ids = new Set<number>();
  for (const [id, flight] of flights) {
    if (flightOwnsId(flight)) ids.add(id);
  }
  return ids;
}

/** True when pose is within settle epsilon of the target (shared by step + authority handoff). */
export function poseAtTarget(
  pose: { x: number; y: number; scale: number },
  target: { x: number; y: number; scale: number },
): boolean {
  return (
    Math.hypot(target.x - pose.x, target.y - pose.y) <= EPSILON_PX &&
    Math.abs(target.scale - pose.scale) <= EPSILON_SCALE
  );
}

/**
 * True when a held local seed is close enough to the authoritative pose that retargeting would
 * only play a short second ease after the main glide.
 */
export function poseNearHandoff(
  pose: { x: number; y: number; scale: number },
  target: { x: number; y: number; scale: number },
): boolean {
  return (
    Math.hypot(target.x - pose.x, target.y - pose.y) <= FLIGHT_HANDOFF_PX &&
    Math.abs(target.scale - pose.scale) <= FLIGHT_HANDOFF_SCALE
  );
}

/** Preserve on-screen flight size when camera zoom changes (scale is zoom-coupled in paint). */
export function remapFlightsForZoom(
  flights: ReadonlyMap<number, CardFlight>,
  oldZoom: number,
  newZoom: number,
): Map<number, CardFlight> {
  if (!(oldZoom > 0) || !(newZoom > 0) || oldZoom === newZoom) return new Map(flights);
  const factor = oldZoom / newZoom;
  const next = new Map<number, CardFlight>();
  for (const [id, flight] of flights) {
    next.set(id, {
      ...flight,
      scale: flight.scale * factor,
      targetScale: flight.targetScale * factor,
    });
  }
  return next;
}

function alreadyAtTarget(flight: CardFlight): boolean {
  return poseAtTarget(flight, { x: flight.targetX, y: flight.targetY, scale: flight.targetScale });
}

function snapToTarget(flight: CardFlight): CardFlight {
  return {
    ...flight,
    x: flight.targetX,
    y: flight.targetY,
    scale: flight.targetScale,
    phase: "settled",
  };
}
