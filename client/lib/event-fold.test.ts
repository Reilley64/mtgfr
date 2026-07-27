import { expect, test } from "vitest";
import { describe as describeEvent } from "~/event-fold";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";

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
