import { Story } from "foldkit";
import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView } from "~/wire/types";
import { ZONE } from "../board/geometry/layout";
import type { ObjectView, VisibleState } from "../domain/wire/types";
import { SubmitIntent } from "../game/intents";
import { init, update } from "../main-exports";
import { GotGameMessage } from "../messages";
import { emptyGameSlice } from "../model";
import { TableRoute } from "../routes";
import { ReceivedDelta } from "./messages";

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

function handMode(id: number, kind: "cast" | "cycle", objectId: number): ActionView {
  return {
    id,
    kind,
    label: testMessageRef(`${kind}-${id}`),
    needs_target: false,
    object: objectId,
    section: "hand",
  };
}

function intentFromCommand(cmd: unknown): unknown {
  return (cmd as { args: { intent: unknown } }).args.intent;
}

test("ReceivedDelta folds into game seq", () => {
  const [model] = init();

  Story.story(
    update,
    Story.with({
      ...model,
      route: TableRoute({ deckId: "0", table: "ABC123" }),
      game: { ...emptyGameSlice(), active: true, tableId: "ABC123" },
    }),
    Story.message(
      GotGameMessage({ message: ReceivedDelta({ seq: 7, state: state(), events: [], auto_actions: undefined }) }),
    ),
    Story.model((m) => {
      expect(m.game?.seq).toBe(7);
    }),
  );
});

test("ReceivedDelta auto-continues a play mode pick that sync prunes to one action", () => {
  const [model] = init();
  const tableId = "ABC123";
  const card = object({
    id: 42,
    kind: { kind: "creature", power: 2, toughness: 2 },
    name: "Valley Rannet",
    zone: ZONE.Hand,
  });
  const castAction = handMode(7, "cast", card.id);
  const cycleAction = handMode(8, "cycle", card.id);
  const game = emptyGameSlice(tableId);

  const [next, commands] = update(
    {
      ...model,
      route: TableRoute({ deckId: "0", table: tableId }),
      game: {
        ...game,
        active: true,
        board: {
          ...game.board,
          playModePick: {
            card,
            modes: [castAction, cycleAction],
            dropSeed: { x: 0, y: 0 },
            screenOrigin: { x: 400, y: 200 },
          },
        },
      },
    },
    GotGameMessage({
      message: ReceivedDelta({
        seq: 7,
        state: { ...state([card]), actions: [cycleAction] },
        events: [],
        auto_actions: undefined,
      }),
    }),
  );

  expect(next.game?.board.playModePick).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
  expect(intentFromCommand(commands[0])).toMatchObject({ kind: "take_action", id: cycleAction.id });
});

test("ReceivedDelta with land_played provenance spawns a board flight", () => {
  const [model] = init();

  Story.story(
    update,
    Story.with({
      ...model,
      route: TableRoute({ deckId: "0", table: "ABC123" }),
      game: { ...emptyGameSlice(), active: true, tableId: "ABC123" },
    }),
    Story.message(
      GotGameMessage({
        message: ReceivedDelta({
          seq: 7,
          state: state([object()]),
          events: [{ kind: "land_played", from: 9, permanent: 3, player: 0 }],
          auto_actions: undefined,
        }),
      }),
    ),
    Story.model((m) => {
      expect(m.game?.board.flights.has(3) || m.game?.board.handHidden.has(9)).toBe(true);
      expect(m.game?.board.hideCardIds.has(3)).toBe(true);
      expect(m.game?.board.ownedIds.has(3)).toBe(true);
    }),
  );
});
