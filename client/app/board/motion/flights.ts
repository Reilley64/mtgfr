import { CARD_W } from "../geometry/layout";

export const HAND_FACE_W = 180;
export const STACK_CARD_W = 112;

const TAU_MS = 75;
const EPSILON_PX = 0.5;
const EPSILON_SCALE = 0.02;

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
  };
}

export function flightSettled(flight: CardFlight): boolean {
  return flight.phase === "settled";
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
  };
}

export function flyingCardIds(flights: ReadonlyMap<number, CardFlight>): Set<number> {
  const ids = new Set<number>();
  for (const [id, flight] of flights) {
    if (flight.phase === "flying") ids.add(id);
  }
  return ids;
}

function alreadyAtTarget(flight: CardFlight): boolean {
  return (
    Math.hypot(flight.targetX - flight.x, flight.targetY - flight.y) <= EPSILON_PX &&
    Math.abs(flight.targetScale - flight.scale) <= EPSILON_SCALE
  );
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
