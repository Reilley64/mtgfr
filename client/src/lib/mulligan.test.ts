import { describe, expect, it } from "vitest";
import { mulliganChrome } from "~/lib/mulligan";

describe("mulliganChrome", () => {
  it("offers keep and mulligan for undecided local seat", () => {
    const c = mulliganChrome({
      mulliganing: true,
      localSeat: 0,
      players: [
        { player: 0, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
        { player: 1, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
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
        { player: 0, hand_kept: false, can_mulligan: false, mulligans_taken: 6 },
        { player: 1, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
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
        { player: 0, hand_kept: true, can_mulligan: false, mulligans_taken: 0 },
        { player: 1, hand_kept: false, can_mulligan: true, mulligans_taken: 0 },
      ],
    });

    expect(c.show).toBe(true);
    expect(c.showControls).toBe(false);
    expect(c.waitingCount).toBe(1);
  });

  it("hidden when not mulliganing", () => {
    expect(mulliganChrome({ mulliganing: false, localSeat: 0, players: [] }).show).toBe(false);
  });
});
