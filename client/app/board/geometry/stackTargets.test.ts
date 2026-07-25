import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ObjectView, PlayerView, StackObjectView, VisibleState } from "~/wire/types";
import { formatStackTargetSuffix, stackEntryTargets } from "./stackTargets";

const entry = (over: Partial<StackObjectView> = {}): StackObjectView => ({
  controller: 0,
  kind: "spell",
  label: testMessageRef("Bolt"),
  source: 1,
  ...over,
});

function player(over: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...over,
  };
}

function visibleState(over: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [player(), player({ player: 1, username: "Bob" })],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

function bearObject(): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 22,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 2, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "bear-print",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: 0,
  };
}

test("stackEntryTargets prefers non-empty targets list", () => {
  expect(
    stackEntryTargets(
      entry({
        target: { kind: "object", id: 1 },
        targets: [
          { kind: "object", id: 2 },
          { kind: "player", player: 1 },
        ],
      }),
    ),
  ).toEqual([
    { kind: "object", id: 2 },
    { kind: "player", player: 1 },
  ]);
});

test("stackEntryTargets falls back to singular target", () => {
  expect(stackEntryTargets(entry({ target: { kind: "object", id: 9 } }))).toEqual([
    { kind: "object", id: 9 },
  ]);
});

test("stackEntryTargets is empty when targetless", () => {
  expect(stackEntryTargets(entry({}))).toEqual([]);
  expect(stackEntryTargets(entry({ target: null, targets: [] }))).toEqual([]);
});

test("formatStackTargetSuffix joins labels once", () => {
  const state = visibleState({ objects: [bearObject()] });
  expect(
    formatStackTargetSuffix(
      [
        { kind: "object", id: 22 },
        { kind: "player", player: 1 },
      ],
      state,
    ),
  ).toBe(" → Bear, Bob");
  expect(formatStackTargetSuffix([], state)).toBe("");
});
