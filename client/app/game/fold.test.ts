import { describe, expect, it } from "vitest";
import type { ObjectView, StreamFrame, VisibleEvent, VisibleState } from "../../lib/wire/types";
import { applyDeltaPure, applySnapshotPure, emptyGameFold, setRejectPure } from "./fold";

type DeltaEnvelope = Omit<Extract<StreamFrame, { frame: "delta" }>, "frame">;

function mkObject(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 0,
    is_commander: false,
    kind: { kind: "creature", power: 0, toughness: 0 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Object",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: 2,
    ...overrides,
  };
}

function mkState(objects: ObjectView[] = []): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects,
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 0,
    viewer: 0,
  };
}

function mkDelta(seq: number, events: VisibleEvent[], objects: ObjectView[] = []): DeltaEnvelope {
  return { events, seq, state: mkState(objects) };
}

describe("pure game fold", () => {
  it("applyDeltaPure ignores an older seq", () => {
    const current = {
      ...emptyGameFold(),
      seq: 5,
      state: mkState(),
      log: [{ seq: 5, text: "kept" }],
    };

    const next = applyDeltaPure(current, mkDelta(3, [{ kind: "player_lost", player: 0 }]));

    expect(next).toBe(current);
  });

  it("applySnapshotPure replaces state and clears provenance", () => {
    const withProvenance = applyDeltaPure(
      applySnapshotPure(emptyGameFold(), 0, mkState()),
      mkDelta(1, [{ kind: "land_played", from: 9, permanent: 3, player: 1 }]),
    );

    const next = applySnapshotPure(withProvenance, 2, mkState([mkObject({ id: 4, name: "Island" })]));

    expect(next.seq).toBe(2);
    expect(next.state?.objects.map((object) => object.name)).toEqual(["Island"]);
    expect(next.provenance.landPlayFrom.size).toBe(0);
    expect(next.tableFeel).toEqual({ land: false, stack: false, resolve: false, damage: false });
  });

  it("applyDeltaPure records landPlayFrom provenance", () => {
    const forest = mkObject({ id: 3, name: "Forest", kind: { kind: "land", colors: [4] } });
    let game = emptyGameFold();
    game = applySnapshotPure(game, 0, mkState([]));
    game = applyDeltaPure(game, mkDelta(1, [{ kind: "land_played", from: 9, permanent: 3, player: 1 }], [forest]));

    expect(game.log).toEqual([{ seq: 1, text: "P1 plays Forest" }]);
    expect(game.provenance.landPlayFrom.get(3)).toBe(9);
    expect(game.tableFeel.land).toBe(true);
  });

  it("applyDeltaPure refreshes stack_hold_remaining_ms for same-seq empty events", () => {
    const current = applySnapshotPure(emptyGameFold(), 5, { ...mkState(), stack_hold_remaining_ms: 2000 });
    const next = applyDeltaPure(current, {
      ...mkDelta(5, []),
      state: { ...mkState(), stack_hold_remaining_ms: 3500 },
    });

    expect(next.seq).toBe(5);
    expect(next.state?.stack_hold_remaining_ms).toBe(3500);
    expect(next.log).toEqual([]);
    expect(next.provenance).toBe(current.provenance);
  });

  it("applies snapshot-sourced mulligan fields with no visible events", () => {
    const current = applySnapshotPure(emptyGameFold(), 0, mkState());
    const next = applyDeltaPure(current, {
      ...mkDelta(1, []),
      state: {
        ...mkState(),
        mulliganing: true,
        players: [
          {
            player: 0,
            username: "p0",
            life: 40,
            commander_tax: 0,
            lost: false,
            hand_count: 7,
            library_count: 92,
            mulligans_taken: 0,
            hand_kept: true,
            can_mulligan: false,
            mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
          },
        ],
      },
    });

    expect(next.state?.mulliganing).toBe(true);
    expect(next.state?.players[0]?.hand_kept).toBe(true);
    expect(next.state?.players[0]?.can_mulligan).toBe(false);
    expect(next.log).toEqual([]);
  });

  it("setRejectPure updates only the reject reason", () => {
    const current = applySnapshotPure(emptyGameFold(), 1, mkState());

    const next = setRejectPure(current, "Nope");

    expect(next.reject).toBe("Nope");
    expect(next.state).toBe(current.state);
    expect(next.provenance).toBe(current.provenance);
  });
});
