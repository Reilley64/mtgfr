import { describe, expect, it } from "vitest";
import type { PlayerView } from "~/api/generated";
import { emptyManaPool } from "~/lib/manaPips";
import { outcome, worstCommanderDamage } from "~/lib/outcome";
import { SPECTATOR_VIEWER } from "~/store";

function seat(player: number, lost = false): PlayerView {
  return { player, life: 40, commander_tax: 0, lost, hand_count: 7, library_count: 92, mana_pool: emptyManaPool() };
}

describe("outcome", () => {
  it("a full table mid-game is still playing", () => {
    expect(outcome([seat(0), seat(1), seat(2), seat(3)], 0)).toEqual({ kind: "playing" });
  });

  it("an eliminated seat with the game still going has no winner yet", () => {
    expect(outcome([seat(0, true), seat(1), seat(2)], 0)).toEqual({ kind: "lost", winner: null });
  });

  it("the last player standing has won", () => {
    expect(outcome([seat(0), seat(1, true), seat(2, true), seat(3, true)], 0)).toEqual({ kind: "won" });
  });

  it("an eliminated seat learns who won", () => {
    expect(outcome([seat(0, true), seat(1), seat(2, true), seat(3, true)], 0)).toEqual({ kind: "lost", winner: 1 });
  });

  it("a spectator sees the game end", () => {
    expect(outcome([seat(0, true), seat(1)], SPECTATOR_VIEWER)).toEqual({ kind: "over", winner: 1 });
  });

  it("a mutual death ends the game with nobody winning", () => {
    expect(outcome([seat(0, true), seat(1, true)], 0)).toEqual({ kind: "lost", winner: null });
  });

  it("the empty seat list of the first frame is not a finished game", () => {
    // Guard against flashing a game-over overlay while the board is still connecting.
    expect(outcome([], 0)).toEqual({ kind: "playing" });
    expect(outcome([seat(0)], 0)).toEqual({ kind: "playing" });
  });
});

describe("worstCommanderDamage", () => {
  it("is zero when no commander has connected", () => {
    expect(worstCommanderDamage([])).toBe(0);
    expect(worstCommanderDamage(undefined)).toBe(0);
  });

  it("takes the largest single commander's tally, not the sum", () => {
    // The 21 must come from one commander (CR 903.10a) — two at 20 apiece kill nobody.
    expect(
      worstCommanderDamage([
        { from: 1, amount: 20 },
        { from: 2, amount: 20 },
      ]),
    ).toBe(20);
  });
});
