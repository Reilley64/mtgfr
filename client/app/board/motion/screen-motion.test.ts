import { describe, expect, it } from "vitest";
import { CARD_W } from "../geometry/layout";
import { HAND_FACE_W, handFlightScale } from "./flights";
import { dragGhostFromHandDrag } from "./screen-motion";

describe("dragGhostFromHandDrag", () => {
  it("maps pointer pose to hand flight scale", () => {
    const zoom = 1.25;
    const ghost = dragGhostFromHandDrag(
      {
        name: "Lightning Bolt",
        print: "bolt",
        x: 120,
        y: 340,
        zone: "hand",
      },
      zoom,
    );
    expect(ghost).toEqual({
      name: "Lightning Bolt",
      print: "bolt",
      x: 120,
      y: 340,
      scale: handFlightScale(zoom),
      zone: "hand",
    });
    expect(ghost.scale).toBe(HAND_FACE_W / (CARD_W * zoom));
  });

  it("defaults missing zone to hand", () => {
    const ghost = dragGhostFromHandDrag({ name: "X", print: "", x: 1, y: 2 }, 1);
    expect(ghost.zone).toBe("hand");
  });

  it("preserves command zone for outline paint", () => {
    const ghost = dragGhostFromHandDrag({ name: "Zimone", print: "z", x: 0, y: 0, zone: "command" }, 1);
    expect(ghost.zone).toBe("command");
  });
});
