import { describe, expect, it } from "vitest";
import { FLIGHT_SHADOW_BLUR, FLIGHT_SHADOW_COLOR, FLIGHT_SHADOW_OFFSET_Y } from "./paint-flights";

describe("flight lift shadow", () => {
  it("locks Arena-forward lift constants", () => {
    expect(FLIGHT_SHADOW_BLUR).toBe(28);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(12);
    expect(FLIGHT_SHADOW_COLOR).toBe("rgba(0,0,0,0.55)");
  });
});
