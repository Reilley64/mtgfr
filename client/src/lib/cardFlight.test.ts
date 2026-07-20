import { describe, expect, it } from "vitest";
import { HAND_CARD_W } from "~/components/molecules/hand";
import { CARD_W } from "~/layout";
import { STACK_CARD_W } from "~/lib/boardDraw";
import {
  type CardFlight,
  flightSettled,
  HAND_FACE_W,
  handFlightScale,
  spawnFlight,
  stackFlightScale,
  stepFlights,
} from "~/lib/cardFlight";

describe("cardFlight", () => {
  it("spawns at the given screen pose and scale", () => {
    const f = spawnFlight({
      id: 1,
      print: "abc",
      name: "Swamp",
      x: 100,
      y: 200,
      scale: 1.5,
      targetX: 300,
      targetY: 400,
      targetScale: 1,
      kind: "battlefield",
    });
    expect(f).toMatchObject({
      id: 1,
      x: 100,
      y: 200,
      scale: 1.5,
      targetX: 300,
      targetY: 400,
      targetScale: 1,
      phase: "flying",
      kind: "battlefield",
    });
    expect(flightSettled(f)).toBe(false);
  });

  it("eases position and scale toward the target without overshooting", () => {
    let flights = new Map<number, CardFlight>([
      [
        1,
        spawnFlight({
          id: 1,
          print: "",
          name: "Swamp",
          x: 0,
          y: 0,
          scale: 2,
          targetX: 100,
          targetY: 50,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
    ]);
    let dist = Math.hypot(100, 50);
    let scaleGap = 1;
    for (let i = 0; i < 5; i++) {
      const r = stepFlights(flights, 16, false);
      flights = r.flights;
      const f = flights.get(1)!;
      const d = Math.hypot(100 - f.x, 50 - f.y);
      expect(d).toBeLessThan(dist);
      dist = d;
      const sg = Math.abs(1 - f.scale);
      expect(sg).toBeLessThan(scaleGap);
      scaleGap = sg;
      expect(r.settled).toBe(false);
    }
  });

  it("settles exactly on the target and marks phase settled", () => {
    let flights = new Map<number, CardFlight>([
      [
        1,
        spawnFlight({
          id: 1,
          print: "",
          name: "Bear",
          x: 0,
          y: 0,
          scale: 2,
          targetX: 400,
          targetY: 0,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
    ]);
    let settled = false;
    let elapsed = 0;
    while (!settled) {
      const r = stepFlights(flights, 16, false);
      flights = r.flights;
      settled = r.settled;
      elapsed += 16;
      expect(elapsed).toBeLessThanOrEqual(1000);
    }
    const f = flights.get(1)!;
    expect(f.x).toBe(400);
    expect(f.y).toBe(0);
    expect(f.scale).toBe(1);
    expect(f.phase).toBe("settled");
    expect(flightSettled(f)).toBe(true);
  });

  it("retargets mid-flight when targets change", () => {
    let flights = new Map<number, CardFlight>([
      [
        1,
        spawnFlight({
          id: 1,
          print: "",
          name: "Swamp",
          x: 0,
          y: 0,
          scale: 2,
          targetX: 100,
          targetY: 0,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
    ]);
    flights = stepFlights(flights, 16, false).flights;
    const mid = flights.get(1)!;
    // Retarget farther away — flight should keep current pose and chase the new target.
    flights = new Map([
      [
        1,
        {
          ...mid,
          targetX: 500,
          targetY: 200,
          targetScale: 0.8,
          phase: "flying",
        },
      ],
    ]);
    const next = stepFlights(flights, 16, false).flights.get(1)!;
    expect(next.targetX).toBe(500);
    expect(next.x).toBeGreaterThan(mid.x);
    expect(next.phase).toBe("flying");
  });

  it("snaps immediately when reduced motion is requested", () => {
    const flights = new Map<number, CardFlight>([
      [
        1,
        spawnFlight({
          id: 1,
          print: "",
          name: "Swamp",
          x: 0,
          y: 0,
          scale: 2,
          targetX: 100,
          targetY: 50,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
    ]);
    const r = stepFlights(flights, 16, true);
    expect(r.settled).toBe(true);
    expect(r.flights.get(1)).toMatchObject({
      x: 100,
      y: 50,
      scale: 1,
      phase: "settled",
    });
  });

  it("drops flights that are no longer in the map input", () => {
    // stepFlights only advances flights present in the input map — caller removes settled ones.
    const flights = new Map<number, CardFlight>([
      [
        1,
        spawnFlight({
          id: 1,
          print: "",
          name: "A",
          x: 0,
          y: 0,
          scale: 1,
          targetX: 0,
          targetY: 0,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
      [
        2,
        spawnFlight({
          id: 2,
          print: "",
          name: "B",
          x: 0,
          y: 0,
          scale: 1,
          targetX: 10,
          targetY: 0,
          targetScale: 1,
          kind: "battlefield",
        }),
      ],
    ]);
    // Already at target for #1.
    const onlyTwo = new Map([[2, flights.get(2)!]]);
    const r = stepFlights(onlyTwo, 16, false);
    expect(r.flights.has(1)).toBe(false);
    expect(r.flights.has(2)).toBe(true);
  });

  it("handFlightScale and stackFlightScale are relative to canvas card screen width", () => {
    const zoom = 0.5;
    expect(HAND_FACE_W).toBe(HAND_CARD_W);
    // Canvas card screen width = CARD_W * zoom = 48; hand face matches HAND_CARD_W.
    expect(handFlightScale(zoom)).toBeCloseTo(HAND_FACE_W / (CARD_W * zoom), 5);
    expect(stackFlightScale(zoom)).toBeCloseTo(STACK_CARD_W / (CARD_W * zoom), 5);
  });
});
