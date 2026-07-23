import { describe, expect, it } from "vitest";
import { mulliganChrome } from "~/mulligan";

describe("mulliganChrome", () => {
  it("offers keep and mulligan for undecided local seat", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 1,
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.show).toBe(true);
    expect(c.showControls).toBe(true);
    expect(c.canMulligan).toBe(true);
    expect(c.waitingCount).toBe(2);
  });

  it("still offers keep when another mulligan is unavailable", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          hand_kept: false,
          can_mulligan: false,
          mulligans_taken: 6,
          lost: false,
        },
        {
          player: 1,
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.showControls).toBe(true);
    expect(c.keepLabel).toBe("Keep");
    expect(c.canMulligan).toBe(false);
  });

  it("hides controls when local seat kept but still shows waiting", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          hand_kept: true,
          can_mulligan: false,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 1,
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.show).toBe(true);
    expect(c.showControls).toBe(false);
    expect(c.waitingCount).toBe(1);
  });

  it("does not count lost seats as waiting", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          username: "Alice",
          hand_kept: true,
          can_mulligan: false,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 1,
          username: "Bob",
          hand_kept: false,
          can_mulligan: false,
          mulligans_taken: 0,
          lost: true,
        },
        {
          player: 2,
          username: "Carol",
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.waitingCount).toBe(1);
    expect(c.status).toBe("Waiting for Carol to choose.");
  });

  it("names undecided seats in the waiting status", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          username: "Alice",
          hand_kept: true,
          can_mulligan: false,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 1,
          username: "Bob",
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 2,
          username: "Carol",
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.waitingCount).toBe(2);
    expect(c.status).toBe("Waiting for Bob and Carol to choose.");
  });

  it("falls back to seat labels when username is empty", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        {
          player: 0,
          username: "Alice",
          hand_kept: true,
          can_mulligan: false,
          mulligans_taken: 0,
          lost: false,
        },
        {
          player: 1,
          username: "  ",
          hand_kept: false,
          can_mulligan: true,
          mulligans_taken: 0,
          lost: false,
        },
      ],
    });

    expect(c.status).toBe("Waiting for P1 to choose.");
  });

  it("hidden when not mulliganing", () => {
    expect(mulliganChrome({ mulliganing: false, localSeat: 0, players: [] }).show).toBe(false);
  });
});
