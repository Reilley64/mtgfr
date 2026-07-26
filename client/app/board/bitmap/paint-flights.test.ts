import { describe, expect, it } from "vitest";
import { LIFT_SHADOW_BLUR, LIFT_SHADOW_COLOR, LIFT_SHADOW_OFFSET_Y } from "../lift-shadow";
import { FLIGHT_SHADOW_BLUR, FLIGHT_SHADOW_COLOR, FLIGHT_SHADOW_OFFSET_Y } from "./paint-flights";

describe("flight lift shadow", () => {
  it("uses the shared drag-token lift constants", () => {
    expect(FLIGHT_SHADOW_BLUR).toBe(LIFT_SHADOW_BLUR);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(LIFT_SHADOW_OFFSET_Y);
    expect(FLIGHT_SHADOW_COLOR).toBe(LIFT_SHADOW_COLOR);
    expect(FLIGHT_SHADOW_BLUR).toBe(36);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(16);
    expect(FLIGHT_SHADOW_COLOR).toBe("rgba(0,0,0,0.72)");
  });
});
