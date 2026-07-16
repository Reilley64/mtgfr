import { describe, expect, it } from "vitest";
import { humanReason, rejectMessageFor } from "~/controllers/reject";

describe("humanReason", () => {
  it("maps engine reject names to player-facing copy", () => {
    expect(humanReason("NotYourPriority")).toBe("It's not your turn to act.");
    expect(humanReason("UnknownAction")).toBe("That action expired — try again.");
    expect(humanReason("CannotPayCost")).toBe("Not enough mana for that.");
    expect(humanReason("CannotDiscardCost")).toBe("You don't have cards to discard for that.");
    expect(humanReason("CannotExileCost")).toBe("You don't have cards to exile for that.");
    expect(humanReason("IllegalChoice")).toBe("That choice isn't valid.");
  });

  it("falls back to the raw reason when unmapped", () => {
    expect(humanReason("SomeNewReason")).toBe("SomeNewReason");
  });
});

describe("rejectMessageFor", () => {
  it("surfaces session expiry on 401", () => {
    expect(rejectMessageFor({ response: { status: 401 } })).toBe("Session expired — sign in again.");
  });

  it("uses a generic network message otherwise", () => {
    expect(rejectMessageFor(new Error("offline"))).toBe("Couldn't reach the table.");
  });
});
