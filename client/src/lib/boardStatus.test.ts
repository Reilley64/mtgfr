import { describe, expect, it } from "vitest";
import type { PlayerView, VisibleState } from "~/api/generated";
import { boardStatusSummary } from "~/lib/boardStatus";
import { emptyManaPool } from "~/lib/manaPips";
import { SPECTATOR_VIEWER } from "~/store";

function seat(player: number, username: string): PlayerView {
  return {
    player,
    username,
    life: 40,
    commander_tax: 0,
    lost: false,
    hand_count: 7,
    library_count: 92,
    mana_pool: emptyManaPool(),
  };
}

function state(
  partial: Partial<VisibleState> & Pick<VisibleState, "active_player" | "priority" | "step">,
): VisibleState {
  return {
    objects: [],
    players: [seat(0, "alice"), seat(1, "bob")],
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    stack: [],
    viewer: 0,
    ...partial,
  };
}

describe("boardStatusSummary", () => {
  it("reports connecting when there is no state", () => {
    expect(boardStatusSummary(null, 0)).toBe("Commander board. Connecting to the table.");
  });

  it("says Your turn / You have priority for the seated viewer", () => {
    expect(boardStatusSummary(state({ active_player: 0, priority: 0, step: 3 }), 0)).toBe(
      "Commander board. Your turn, Main 1. You have priority. Stack empty.",
    );
  });

  it("names the other player when it is not the viewer's turn or priority", () => {
    expect(boardStatusSummary(state({ active_player: 1, priority: 1, step: 5 }), 0)).toBe(
      "Commander board. bob's turn, Declare Attackers. Priority: bob. Stack empty.",
    );
  });

  it("never says Your turn for a spectator, even when seat 0 is active", () => {
    const s = state({
      active_player: 0,
      priority: 0,
      step: 3,
      // Two stack entries — only length is read.
      stack: [{}, {}] as VisibleState["stack"],
    });
    expect(boardStatusSummary(s, SPECTATOR_VIEWER)).toBe(
      "Commander board. Spectating. alice's turn, Main 1. Priority: alice. Stack: 2 objects.",
    );
  });
});
