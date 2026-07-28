// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import type { VisibleState } from "~/wire/types";
import { emptyGameFold, type GameFoldState } from "../game/fold";
import { markRevealSeen, revealSeen, revealSlot, spotlightSteps } from "./first-player-reveal";
import { FirstPlayerRevealFinished, FirstPlayerRevealStepped } from "./messages";
import { armFirstPlayerReveal, initialBoardModel, updateBoard } from "./submodel";

describe("spotlightSteps", () => {
  it("ends on the winner", () => {
    const steps = spotlightSteps(2, 4, false);
    expect(steps.at(-1)?.slot).toBe(2);
  });

  it("hops every seat in screen order and decelerates", () => {
    const steps = spotlightSteps(1, 4, false);
    expect(steps.map((s) => s.slot).slice(0, 5)).toEqual([0, 1, 2, 3, 0]);
    const gaps = steps.slice(1).map((s) => s.delayMs);
    expect(gaps.at(-1)).toBeGreaterThan(gaps[0] ?? 0);
    expect(steps[0]?.delayMs).toBe(0);
  });

  it("skips the hop under reduced motion", () => {
    expect(spotlightSteps(3, 4, true)).toEqual([{ slot: 3, delayMs: 0 }]);
  });

  it("survives a one-seat table", () => {
    expect(spotlightSteps(0, 1, false).at(-1)?.slot).toBe(0);
  });
});

describe("revealSlot", () => {
  it("is viewer-relative", () => {
    expect(revealSlot(2, 2, 4)).toBe(0);
    expect(revealSlot(3, 2, 4)).toBe(1);
  });

  it("falls back to seat order for a spectator", () => {
    expect(revealSlot(2, 255, 4)).toBe(2);
  });

  it("clamps count to avoid NaN", () => {
    expect(revealSlot(0, 0, 0)).toBe(0);
    expect(revealSlot(1, 0, 0)).toBe(0);
  });
});

describe("one-shot storage", () => {
  it("remembers a table it has shown", () => {
    expect(revealSeen("t-1")).toBe(false);
    markRevealSeen("t-1");
    expect(revealSeen("t-1")).toBe(true);
    expect(revealSeen("t-2")).toBe(false);
  });

  it("treats a throwing sessionStorage as not-yet-seen", () => {
    const setItem = sessionStorage.setItem;
    sessionStorage.setItem = () => {
      throw new Error("denied");
    };
    expect(() => markRevealSeen("t-3")).not.toThrow();
    sessionStorage.setItem = setItem;
  });
});

function gameState(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: null,
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
        username: "Alice",
      },
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 1,
        username: "Bob",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

describe("first player reveal (submodel)", () => {
  const mulliganFold = (winner: number): GameFoldState => ({
    ...emptyGameFold(),
    seq: 1,
    state: gameState({ active_player: winner, mulliganing: true, viewer: 0 }),
  });

  it("arms once per table and steps to the winner", () => {
    sessionStorage.clear();
    const [armed, cmds] = armFirstPlayerReveal(initialBoardModel(), mulliganFold(2), "t-arm");
    expect(armed.firstPlayerReveal?.winner).toBe(2);
    expect(cmds).toHaveLength(1);

    const [again, noCmds] = armFirstPlayerReveal(initialBoardModel(), mulliganFold(2), "t-arm");
    expect(again.firstPlayerReveal).toBeNull();
    expect(noCmds).toHaveLength(0);
  });

  it("never arms outside mulligans", () => {
    sessionStorage.clear();
    const fold: GameFoldState = {
      ...emptyGameFold(),
      seq: 1,
      state: gameState({ active_player: 1, mulliganing: false, viewer: 0 }),
    };
    const [model] = armFirstPlayerReveal(initialBoardModel(), fold, "t-late");
    expect(model.firstPlayerReveal).toBeNull();
  });

  it("advances the spotlight and clears when finished", () => {
    sessionStorage.clear();
    const [armed] = armFirstPlayerReveal(initialBoardModel(), mulliganFold(2), "t-step");
    const [stepped, stepCmds] = updateBoard(armed, FirstPlayerRevealStepped(), mulliganFold(2), "t-step");
    expect(stepped.firstPlayerReveal?.index).toBe(1);
    expect(stepCmds).toHaveLength(1);
    const [done] = updateBoard(stepped, FirstPlayerRevealFinished(), mulliganFold(2), "t-step");
    expect(done.firstPlayerReveal).toBeNull();
  });
});
