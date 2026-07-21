import { beforeEach, describe, expect, it } from "vitest";
import { applyDelta, applySnapshot, foldProvenance, game, lastTableFeelBatch, resetGame, setGame } from "~/store";
import type { ObjectView, StackObjectView, StreamFrame, VisibleEvent, VisibleState } from "~/wire/types";

/** The delta payload (delta arm of `StreamFrame` minus its `frame` tag; the generator inlines it). */
type DeltaEnvelope = Omit<Extract<StreamFrame, { frame: "delta" }>, "frame">;

import { ZONE } from "~/layout";

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
    zone: ZONE.Battlefield,
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

beforeEach(() => {
  setGame({ state: null, seq: 0, reject: null, log: [] });
});

describe("resetGame", () => {
  it("clears state/seq/log (M4: a fresh Board mount must not render the last game)", () => {
    setGame({ state: mkState(), seq: 500, reject: "rejected", log: [{ seq: 500, text: "stale" }] });
    resetGame();
    expect(game).toEqual({ state: null, seq: 0, reject: null, log: [] });
  });
});

describe("applyDelta", () => {
  it("drops a delta older than what's already shown", () => {
    setGame({ state: mkState(), seq: 5, log: [{ seq: 5, text: "kept" }] });
    applyDelta(mkDelta(3, [{ kind: "player_lost", player: 0 }]));
    expect(game.seq).toBe(5);
    expect(game.log).toEqual([{ seq: 5, text: "kept" }]);
  });

  it("drops a duplicate delta at the same seq that carries events", () => {
    setGame({ state: mkState(), seq: 5, log: [{ seq: 5, text: "kept" }] });
    applyDelta(mkDelta(5, [{ kind: "player_lost", player: 0 }]));
    expect(game.seq).toBe(5);
    expect(game.log).toEqual([{ seq: 5, text: "kept" }]);
  });

  it("patches stack_hold_remaining_ms on a same-seq hold tick", () => {
    setGame({ state: { ...mkState(), stack_hold_remaining_ms: 2000 }, seq: 5, log: [] });
    applyDelta({
      ...mkDelta(5, []),
      state: { ...mkState(), stack_hold_remaining_ms: 3500 },
    });
    expect(game.seq).toBe(5);
    expect(game.state?.stack_hold_remaining_ms).toBe(3500);
    expect(game.log).toEqual([]);
  });

  it("replaces the rendered state and bumps seq on a fresh delta", () => {
    const bears = mkObject({ id: 1, name: "Grizzly Bears" });
    applyDelta(mkDelta(1, [], [bears]));
    expect(game.seq).toBe(1);
    expect(game.state?.objects).toEqual([bears]);
  });

  it("applies snapshot-sourced mulligan updates with no visible events", () => {
    applyDelta({
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
    expect(game.state?.players[0]?.hand_kept).toBe(true);
    expect(game.state?.players[0]?.can_mulligan).toBe(false);
    expect(game.log).toEqual([]);
  });

  it("appends a narrated log line for each event with narrative value", () => {
    applyDelta(mkDelta(1, [{ kind: "player_lost", player: 2 }]));
    expect(game.log).toEqual([{ seq: 1, text: "P2 loses the game" }]);
  });

  it("drops events the switch doesn't narrate, without throwing", () => {
    applyDelta(mkDelta(1, [{ kind: "step_began", active_player: 0, step: 3 }]));
    expect(game.log).toEqual([]);
  });

  it("drops the new bookkeeping-only kinds without logging or throwing", () => {
    applyDelta(
      mkDelta(1, [
        { kind: "spell_damage_divided", spell: 1, assignment: [[0, 1]], players: [] },
        { kind: "keywords_stripped", object: 2 },
        { kind: "flash_permission_granted", player: 0 },
        { kind: "put_on_bottom_of_library", card: 3, player: 0 },
      ]),
    );
    expect(game.log).toEqual([]);
  });

  it("keeps only the last 200 log lines", () => {
    const events: VisibleEvent[] = Array.from({ length: 250 }, (_, i) => ({
      kind: "attacker_declared",
      object: i,
      defender: 1,
    }));
    applyDelta(mkDelta(1, events));
    expect(game.log).toHaveLength(200);
    expect(game.log[0].text).toBe("#50 attacks");
    expect(game.log[199].text).toBe("#249 attacks");
  });

  describe("narration per event kind", () => {
    const bolt = mkObject({ id: 1, name: "Lightning Bolt" });
    const bear = mkObject({ id: 2, name: "Grizzly Bears" });
    const forest = mkObject({ id: 3, name: "Forest" });

    it("spell_cast, with and without a target", () => {
      applyDelta(
        mkDelta(1, [{ kind: "spell_cast", controller: 0, from: 9, spell: 1, escape: false, flashback: false }], [bolt]),
      );
      expect(game.log[0].text).toBe("P0 casts Lightning Bolt");

      applyDelta(
        mkDelta(
          2,
          [
            {
              kind: "spell_cast",
              controller: 0,
              from: 9,
              spell: 1,
              escape: false,
              flashback: false,
              target: { kind: "object", id: 2 },
            },
          ],
          [bolt, bear],
        ),
      );
      expect(game.log[1].text).toBe("P0 casts Lightning Bolt → Grizzly Bears");
    });

    it("land_played", () => {
      applyDelta(mkDelta(1, [{ kind: "land_played", from: 9, permanent: 3, player: 1 }], [forest]));
      expect(game.log[0].text).toBe("P1 plays Forest");
    });

    it("drew_from_empty_library — decking out is the one loss with no visible cause", () => {
      applyDelta(
        mkDelta(1, [
          { kind: "drew_from_empty_library", player: 2 },
          { kind: "player_lost", player: 2 },
        ]),
      );
      expect(game.log.map((l) => l.text)).toEqual(["P2 tries to draw from an empty library", "P2 loses the game"]);
    });

    it("permanent_entered", () => {
      applyDelta(mkDelta(1, [{ kind: "permanent_entered", from: 9, permanent: 2 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears enters");
    });

    it("triggered_ability_on_stack, with and without a target", () => {
      applyDelta(mkDelta(1, [{ kind: "triggered_ability_on_stack", controller: 0, source: 2 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears's ability triggers");

      applyDelta(
        mkDelta(
          2,
          [{ kind: "triggered_ability_on_stack", controller: 0, source: 2, target: { kind: "object", id: 1 } }],
          [bear, bolt],
        ),
      );
      expect(game.log[1].text).toBe("Grizzly Bears's ability triggers → Lightning Bolt");
    });

    it("damage_marked, with and without a source", () => {
      applyDelta(mkDelta(1, [{ kind: "damage_marked", amount: 3, object: 2 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears takes 3");

      applyDelta(mkDelta(2, [{ kind: "damage_marked", amount: 3, object: 2, source: 1 }], [bear, bolt]));
      expect(game.log[1].text).toBe("Grizzly Bears takes 3 from Lightning Bolt");
    });

    it("life_changed, losing and gaining", () => {
      applyDelta(mkDelta(1, [{ kind: "life_changed", amount: -3, player: 0 }]));
      expect(game.log[0].text).toBe("P0 loses 3 life");

      applyDelta(mkDelta(2, [{ kind: "life_changed", amount: 5, player: 0 }]));
      expect(game.log[1].text).toBe("P0 gains 5 life");
    });

    it("moved_to_graveyard", () => {
      applyDelta(mkDelta(1, [{ kind: "moved_to_graveyard", card: 2, from: 9 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears dies");
    });

    it("moved_to_command_zone", () => {
      applyDelta(mkDelta(1, [{ kind: "moved_to_command_zone", card: 2, from: 9 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears returns to the command zone");
    });

    it("counters_placed", () => {
      applyDelta(mkDelta(1, [{ kind: "counters_placed", object: 2, count: 1 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears gets 1 +1/+1 counter");

      applyDelta(mkDelta(2, [{ kind: "counters_placed", object: 2, count: 3 }], [bear]));
      expect(game.log[1].text).toBe("Grizzly Bears gets 3 +1/+1 counters");
    });

    it("attacker_declared", () => {
      applyDelta(mkDelta(1, [{ kind: "attacker_declared", object: 2, defender: 1 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears attacks");
    });

    it("blocker_declared", () => {
      applyDelta(mkDelta(1, [{ kind: "blocker_declared", attacker: 2, blocker: 3 }], [bear, forest]));
      expect(game.log[0].text).toBe("Forest blocks Grizzly Bears");
    });

    it("card_drawn, with and without a known name", () => {
      applyDelta(mkDelta(1, [{ kind: "card_drawn", from: 9, object: 1, player: 0, card: "Lightning Bolt" }]));
      expect(game.log[0]).toEqual({ seq: 1, text: "Drew Lightning Bolt", auto: true });

      applyDelta(mkDelta(2, [{ kind: "card_drawn", from: 9, object: 1, player: 0, card: null }]));
      expect(game.log[1]).toEqual({ seq: 2, text: "Drew a card", auto: true });

      applyDelta(mkDelta(3, [{ kind: "card_drawn", from: 9, object: 2, player: 1, card: null }]));
      expect(game.log[2]).toEqual({ seq: 3, text: "P1 draws a card" });
    });

    it("player_lost", () => {
      applyDelta(mkDelta(1, [{ kind: "player_lost", player: 3 }]));
      expect(game.log[0].text).toBe("P3 loses the game");
    });

    it("flipped", () => {
      applyDelta(mkDelta(1, [{ kind: "flipped", object: 2 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears flips");
    });

    it("color_set_until_end_of_turn", () => {
      applyDelta(mkDelta(1, [{ kind: "color_set_until_end_of_turn", object: 2, color: 2 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears becomes black until end of turn");
    });

    it("creature_type_chosen", () => {
      applyDelta(mkDelta(1, [{ kind: "creature_type_chosen", object: 2, subtype: "Goblin" }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears is chosen as Goblin");
    });

    it("kind_counters_placed — counter_kind unnamed (no client name table)", () => {
      applyDelta(mkDelta(1, [{ kind: "kind_counters_placed", object: 2, count: 1, counter_kind: 0 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears gets 1 counter");

      applyDelta(mkDelta(2, [{ kind: "kind_counters_placed", object: 2, count: 2, counter_kind: 3 }], [bear]));
      expect(game.log[1].text).toBe("Grizzly Bears gets 2 counters");
    });

    it("control_gained", () => {
      applyDelta(mkDelta(1, [{ kind: "control_gained", object: 2, controller: 1 }], [bear]));
      expect(game.log[0].text).toBe("P1 gains control of Grizzly Bears");
    });

    it("token_entered_attacking", () => {
      applyDelta(mkDelta(1, [{ kind: "token_entered_attacking", token: 2, defender: 1 }], [bear]));
      expect(game.log[0].text).toBe("Grizzly Bears enters attacking");
    });

    it("citys_blessing_gained", () => {
      applyDelta(mkDelta(1, [{ kind: "citys_blessing_gained", player: 2 }]));
      expect(game.log[0].text).toBe("P2 gains the city's blessing");
    });

    it("revealed_top_of_library — uses def (hidden-zone card not in state.objects)", () => {
      applyDelta(mkDelta(1, [{ kind: "revealed_top_of_library", card: 77, def: "Sol Ring", player: 0 }]));
      expect(game.log[0].text).toBe("P0 reveals Sol Ring");
    });

    it("library_shuffled", () => {
      applyDelta(mkDelta(1, [{ kind: "library_shuffled", player: 1 }]));
      expect(game.log[0].text).toBe("P1 shuffles their library");
    });

    it("falls back to a #id label for an object id not in the delta's state", () => {
      applyDelta(mkDelta(1, [{ kind: "damage_marked", amount: 1, object: 999 }]));
      expect(game.log[0].text).toBe("#999 takes 1");
    });
  });

  describe("foldProvenance (cross-zone glide + table-feel)", () => {
    it("maps a moved card's new id to the id it came from", () => {
      applyDelta(mkDelta(1, [{ kind: "moved_to_graveyard", card: 50, from: 12 }]));
      expect(foldProvenance().zoneMoves.get(50)).toBe(12); // graveyard object 50 came from battlefield object 12
    });

    it("covers entering permanents too, and resets each delta", () => {
      applyDelta(mkDelta(1, [{ kind: "permanent_entered", permanent: 60, from: 30 }]));
      expect(foldProvenance().zoneMoves.get(60)).toBe(30);
      // A later delta with no zone move clears the map (stale provenance must not linger).
      applyDelta(mkDelta(2, [{ kind: "priority_passed", player: 0 }]));
      expect(foldProvenance().zoneMoves.size).toBe(0);
    });

    it("marks a permanent entering from the stack, so it glides from the stack overlay", () => {
      // permanent_entered's `from` is the resolved spell's stack object — the overlay is where
      // that card was drawn, so the canvas seeds the entrance glide there. A graveyard move is
      // ordinary zone provenance, not a stack resolution.
      applyDelta(
        mkDelta(1, [
          { kind: "permanent_entered", permanent: 60, from: 30 },
          { kind: "moved_to_graveyard", card: 50, from: 12 },
        ]),
      );
      const { resolvedFromStack } = foldProvenance();
      expect(resolvedFromStack.has(60)).toBe(true);
      expect(resolvedFromStack.has(50)).toBe(false);
      // And it resets with the next delta, like the move map.
      applyDelta(mkDelta(2, [{ kind: "priority_passed", player: 0 }]));
      expect(foldProvenance().resolvedFromStack.size).toBe(0);
    });

    it("records land_played permanent → hand card for play-origin matching", () => {
      applyDelta(
        mkDelta(
          1,
          [{ kind: "land_played", from: 9, permanent: 3, player: 0 }],
          [mkObject({ id: 3, name: "Forest", kind: { kind: "land", colors: [4] } })],
        ),
      );
      expect(foldProvenance().landPlayFrom.get(3)).toBe(9);
      applyDelta(mkDelta(2, [{ kind: "priority_passed", player: 0 }]));
      expect(foldProvenance().landPlayFrom.size).toBe(0);
    });

    it("flags table-feel batches once per delta kind", () => {
      applyDelta(
        mkDelta(
          1,
          [
            { kind: "land_played", from: 9, permanent: 3, player: 0 },
            { kind: "combat_damage_dealt_to_player", player: 1, source: 2, amount: 3 },
          ],
          [mkObject({ id: 3, name: "Forest", kind: { kind: "land", colors: [4] } })],
        ),
      );
      expect(lastTableFeelBatch()).toEqual({ land: true, stack: false, resolve: false, damage: true });
      applyDelta(mkDelta(2, [{ kind: "priority_passed", player: 0 }]));
      expect(lastTableFeelBatch()).toEqual({ land: false, stack: false, resolve: false, damage: false });
    });
  });

  describe("priorStackObjectIds via foldProvenance", () => {
    const stackSpell = (source: number): StackObjectView => ({
      controller: 0,
      kind: "spell",
      label: "Spell",
      source,
    });

    it("freezes the pre-delta stack so resolving creators still match this frame", () => {
      applySnapshot(1, { ...mkState(), stack: [stackSpell(30)] });
      // Spell resolves off the stack in this delta — post-state stack is empty.
      applyDelta({
        ...mkDelta(2, [{ kind: "token_created", controller: 0, token: 99, creator: 30 }]),
        state: { ...mkState(), stack: [] },
      });
      expect(foldProvenance().priorStackObjectIds.has(30)).toBe(true);
      // Next delta: prior was already empty after the resolve.
      applyDelta(mkDelta(3, [{ kind: "priority_passed", player: 0 }]));
      expect(foldProvenance().priorStackObjectIds.has(30)).toBe(false);
    });

    it("clears on snapshot", () => {
      applySnapshot(1, { ...mkState(), stack: [stackSpell(30)] });
      applyDelta({
        ...mkDelta(2, []),
        state: { ...mkState(), stack: [] },
      });
      expect(foldProvenance().priorStackObjectIds.has(30)).toBe(true);
      applySnapshot(3, mkState());
      expect(foldProvenance().priorStackObjectIds.size).toBe(0);
    });
  });
});

// Forced choices the server auto-submits used to toast; they now append to the game log with an
// AUTO chip so the board stays quiet and the log is the single narrative surface.
describe("auto-action log lines", () => {
  beforeEach(resetGame);

  it("marks server auto-actions and the viewer's draw as auto, after event lines", () => {
    applyDelta({
      ...mkDelta(1, [{ kind: "card_drawn", from: 9, object: 1, player: 0, card: "Shock" }]),
      auto_actions: ["Discarded to hand size (forced)"],
    });
    expect(game.log).toEqual([
      { seq: 1, text: "Drew Shock", auto: true },
      { seq: 1, text: "Discarded to hand size (forced)", auto: true },
    ]);
  });

  it("still logs auto-actions when the delta has no narratable events", () => {
    applyDelta({ ...mkDelta(1, []), auto_actions: ["Discarded to hand size (forced)"] });
    expect(game.log).toEqual([{ seq: 1, text: "Discarded to hand size (forced)", auto: true }]);
  });
});
