import { describe, expect, it } from "vitest";
import { poseNearHandoff, spawnFlight, stepFlights } from "./flights";

describe("stepFlights", () => {
  it("snaps when within handoff distance so the asymptotic crawl does not read as a second glide", () => {
    // Exponential ease leaves a long slow tail (position mostly arrived, scale still creeping).
    // That tail is the every-time land "stop then second ease" — snap the last inches.
    const flight = spawnFlight({
      id: 1,
      print: "p",
      name: "Forest",
      x: 700,
      y: 520,
      scale: 1.2,
      targetX: 737,
      targetY: 565,
      targetScale: 1,
      kind: "battlefield",
      hold: true,
    });
    expect(poseNearHandoff(flight, { x: flight.targetX, y: flight.targetY, scale: flight.targetScale })).toBe(true);

    const stepped = stepFlights(new Map([[1, flight]]), 16, false);
    const next = stepped.flights.get(1);
    expect(next).toMatchObject({
      x: 737,
      y: 565,
      scale: 1,
      phase: "settled",
    });
    expect(stepped.settled).toBe(true);
  });
});
