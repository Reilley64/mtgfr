import { describe, expect, it, test } from "vitest";
import { describe as describeEvent, extractProvenance } from "./event-fold";
import type { ObjectView, PlayerView, VisibleEvent, VisibleState } from "./wire/types";

function state(objects: ObjectView[] = []): VisibleState {
  const players: PlayerView[] = [0, 1].map((player) => ({
    commander_tax: 0,
    hand_count: 7,
    library_count: 90,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player,
    username: player === 0 ? "Alice" : "Bob",
  }));
  return {
    actions: [],
    attacks: [],
    blocks: [],
    objects,
    phase: 0,
    players,
    priority: 0,
    stack: [],
    step: 0,
    turn: 1,
    turn_player: 0,
    viewer: 0,
  } as unknown as VisibleState;
}

// Poison is a lose condition (CR 704.5c) and rad drives a mill clock, so both kinds are named
// in the log — unlike permanent counters, whose kind index has no client name table.
test("player counter lines name poison and rad", () => {
  expect(describeEvent({ kind: "player_counters_placed", player: 1, counter_kind: 0, count: 2 }, state())).toBe(
    "Bob gets 2 poison counters",
  );
  expect(describeEvent({ kind: "player_counters_placed", player: 1, counter_kind: 1, count: 1 }, state())).toBe(
    "Bob gets 1 rad counter",
  );
});

// Rad counters come off as their mill resolves, so removal must not read as a gain.
test("removing a player counter reads as a loss", () => {
  expect(describeEvent({ kind: "player_counters_placed", player: 0, counter_kind: 1, count: -1 }, state())).toBe(
    "Alice loses 1 rad counter",
  );
});

// CR 114.5 — nothing removes an emblem, so the log line is its only trace.
test("emblem creation names the controller and the emblem", () => {
  expect(
    describeEvent({ kind: "emblem_created", emblem: 40, controller: 0, name: "Garruk, Cursed Huntsman" }, state()),
  ).toBe("Alice gets an emblem (Garruk, Cursed Huntsman)");
});

test("monstrosity names the permanent", () => {
  const bear = { id: 5, name: "Grizzly Bear", controller: 0 } as unknown as ObjectView;
  expect(describeEvent({ kind: "became_monstrous", object: 5 }, state([bear]))).toBe("Grizzly Bear becomes monstrous");
});

describe("extractProvenance battlefieldExits", () => {
  it("tags BF→graveyard as destroy path", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_graveyard", card: 10, from: 10 }];
    const priorBf = new Set([10]);
    const p = extractProvenance(events, new Set(), 0, priorBf);
    expect(p.battlefieldExits.get(10)).toBe("graveyard");
    expect(p.moves.get(10)).toBe(10);
  });

  it("tags BF→exile as exile path", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_exile", card: 11, from: 11 }];
    const p = extractProvenance(events, new Set(), 0, new Set([11]));
    expect(p.battlefieldExits.get(11)).toBe("exile");
  });

  it("does not tag mill or non-BF graveyard entrance", () => {
    const events: VisibleEvent[] = [
      { kind: "milled", card: 12, from: 12, player: 0 },
      { kind: "moved_to_graveyard", card: 13, from: 13 },
    ];
    const p = extractProvenance(events, new Set(), 0, new Set()); // 13 not on BF
    expect(p.battlefieldExits.has(12)).toBe(false);
    expect(p.battlefieldExits.has(13)).toBe(false);
  });

  it("tags when prior BF id is `from` after rebind-style id change", () => {
    const events: VisibleEvent[] = [{ kind: "moved_to_graveyard", card: 20, from: 19 }];
    const p = extractProvenance(events, new Set(), 0, new Set([19]));
    expect(p.battlefieldExits.get(20)).toBe("graveyard");
  });
});

// Both of these are events with no other trace on the board: the shield eats the points before a
// damage or life line would be minted, and looking at a hand moves nothing at all.
test("prevention and hand-peeking lines name their target", () => {
  expect(describeEvent({ kind: "damage_prevented", amount: 3, player: 1 }, state())).toBe(
    "3 damage to Bob is prevented",
  );
  expect(describeEvent({ kind: "looked_at_hand", player: 0, target: 1 }, state())).toBe("Alice looks at Bob's hand");
});

// CR 104.4: a draw ends the game for everyone at once, so no `player_lost` fires and there is no
// seat to name. Before this line the client showed nothing — the game simply stopped responding.
test("a draw says so, since no player_lost fires to explain the stop", () => {
  expect(describeEvent({ kind: "game_drawn" }, state())).toBe("The game is a draw");
});
