import { Story } from "foldkit";
import { expect, test } from "vitest";
import type { ObjectView, VisibleState } from "../../lib/wire/types";
import { ZONE } from "../board/geometry/layout";
import { init, update } from "../main-exports";
import { ReceivedDelta } from "../messages";
import { emptyGameSlice } from "../model";
import { TableRoute } from "../routes";

function object(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 3,
    is_commander: false,
    kind: { kind: "land", colors: [4] },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Forest",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "forest-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function state(objects: ObjectView[] = []): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects,
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
      },
    ],
    priority: 0,
    stack: [],
    step: 0,
    viewer: 0,
  };
}

test("ReceivedDelta folds into game seq", () => {
  const [model] = init();

  Story.story(
    update,
    Story.with({
      ...model,
      route: TableRoute({ table: "ABC123" }),
      game: { ...emptyGameSlice(), active: true, tableId: "ABC123" },
    }),
    Story.message(ReceivedDelta({ seq: 7, state: state(), events: [], auto_actions: undefined })),
    Story.model((m) => {
      expect(m.game?.seq).toBe(7);
    }),
  );
});

test("ReceivedDelta with land_played provenance spawns a board flight", () => {
  const [model] = init();

  Story.story(
    update,
    Story.with({
      ...model,
      route: TableRoute({ table: "ABC123" }),
      game: { ...emptyGameSlice(), active: true, tableId: "ABC123" },
    }),
    Story.message(
      ReceivedDelta({
        seq: 7,
        state: state([object()]),
        events: [{ kind: "land_played", from: 9, permanent: 3, player: 0 }],
        auto_actions: undefined,
      }),
    ),
    Story.model((m) => {
      expect(m.game?.board.flights.has(3) || m.game?.board.handHidden.has(9)).toBe(true);
      expect(m.game?.board.hideCardIds.has(3)).toBe(true);
      expect(m.game?.board.ownedIds.has(3)).toBe(true);
    }),
  );
});
