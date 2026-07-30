import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ObjectView, VisibleState } from "~/wire/types";
import { type BoardStaging, engagedIds } from "./engagement";
import { ZONE } from "./geometry/layout";

function creature(id: number, over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...over,
  };
}

function state(over: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [creature(1), creature(2), creature(3)],
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

const noStaging: BoardStaging = {
  combatAttackers: [],
  combatBlocks: [],
  promptDraft: null,
};

test("nothing is engaged on an idle board", () => {
  expect(engagedIds(state(), noStaging).size).toBe(0);
});

test("wire attackers, blockers, and blocked attackers are engaged", () => {
  const engaged = engagedIds(
    state({
      combat: {
        attackers: [{ attacker: 1, defender: 1 }],
        blocks: [{ attacker: 1, blocker: 2 }],
        attackers_declared: true,
        blockers_declared: [1],
        blocked_attackers: [3],
      },
    }),
    noStaging,
  );
  expect([...engaged].sort()).toEqual([1, 2, 3]);
});

test("locally staged attackers and blockers are engaged before the wire sees them", () => {
  const engaged = engagedIds(state(), {
    ...noStaging,
    combatAttackers: [{ attacker: 2, defender: 1 }],
    combatBlocks: [{ attacker: 9, blocker: 3 }],
  });
  expect([...engaged].sort()).toEqual([2, 3]);
});

test("objects targeted by a stack entry are engaged", () => {
  const engaged = engagedIds(
    state({
      stack: [
        {
          controller: 0,
          kind: "spell",
          label: testMessageRef("Shock"),
          source: 50,
          targets: [
            { kind: "object", id: 2 },
            { kind: "player", player: 1 },
          ],
        },
      ],
    }),
    noStaging,
  );
  expect([...engaged]).toEqual([2]);
});

test("every prompt-draft target kind is engaged", () => {
  const pick = engagedIds(state(), { ...noStaging, promptDraft: { kind: "card-pick", picked: [1, 3] } });
  expect([...pick].sort()).toEqual([1, 3]);

  const one = engagedIds(state(), { ...noStaging, promptDraft: { kind: "target", id: 2 } });
  expect([...one]).toEqual([2]);

  const many = engagedIds(state(), { ...noStaging, promptDraft: { kind: "targets", ids: [1, 2] } });
  expect([...many].sort()).toEqual([1, 2]);
});

test("a non-targeting draft engages nothing", () => {
  expect(engagedIds(state(), { ...noStaging, promptDraft: { kind: "may", yes: true } }).size).toBe(0);
});
