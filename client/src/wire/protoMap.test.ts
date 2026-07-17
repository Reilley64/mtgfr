import { describe, expect, it } from "vitest";
import {
  deckDetailFromProto,
  fromProtoWire,
  intentEnvelopeToProto,
  saveDeckToProto,
  streamFrameFromProto,
  toProtoWire,
} from "~/wire/protoMap";
import type { DeckDetail, IntentEnvelope, SaveDeckRequest } from "~/wire/types";

describe("fromProtoWire / toProtoWire", () => {
  it("round-trips a WireTarget object target", () => {
    const wire = { kind: "object", id: 7 };
    const proto = toProtoWire(wire);
    expect(proto).toEqual({ kind: { case: "object", value: { id: 7 } } });
    expect(fromProtoWire(proto)).toEqual(wire);
  });

  it("round-trips a WireTarget player target", () => {
    const wire = { kind: "player", player: 2 };
    const proto = toProtoWire(wire);
    expect(proto).toEqual({ kind: { case: "player", value: { player: 2 } } });
    expect(fromProtoWire(proto)).toEqual(wire);
  });

  it("round-trips a WireKind creature", () => {
    const wire = { kind: "creature", power: 2, toughness: 3 };
    const proto = toProtoWire(wire);
    expect(proto).toEqual({ kind: { case: "creature", value: { power: 2, toughness: 3 } } });
    expect(fromProtoWire(proto)).toEqual(wire);
  });

  it("maps DeckDetail commander_print <-> commanderPrint", () => {
    const proto = { id: 5n, name: "Kess", commander: "Kess, Dissident Mage", commanderPrint: "print-1", cards: [] };
    const deck = deckDetailFromProto(proto);
    const expected: DeckDetail = {
      id: 5,
      name: "Kess",
      commander: "Kess, Dissident Mage",
      commander_print: "print-1",
      cards: [],
    };
    expect(deck).toEqual(expected);

    const request: SaveDeckRequest = {
      name: deck.name,
      commander: deck.commander,
      commander_print: deck.commander_print,
      cards: [],
    };
    expect(saveDeckToProto(request)).toEqual({
      name: "Kess",
      commander: "Kess, Dissident Mage",
      commanderPrint: "print-1",
      cards: [],
    });
  });

  it("flattens a StreamFrame heartbeat", () => {
    expect(streamFrameFromProto({ frame: { case: "heartbeat", value: {} } })).toEqual({ frame: "heartbeat" });
  });

  it("flattens a minimal StreamFrame snapshot", () => {
    const proto = {
      frame: {
        case: "snapshot",
        value: {
          seq: 5n,
          state: {
            viewer: 0,
            activePlayer: 0,
            step: 0,
            priority: 0,
            players: [],
            objects: [],
            stack: [],
            canAct: true,
            yielded: false,
            turnYielded: false,
            stackHoldRemainingMs: 0,
            actions: [],
          },
        },
      },
    };
    expect(streamFrameFromProto(proto)).toEqual({
      frame: "snapshot",
      seq: 5,
      state: {
        viewer: 0,
        active_player: 0,
        step: 0,
        priority: 0,
        players: [],
        objects: [],
        stack: [],
        can_act: true,
        yielded: false,
        turn_yielded: false,
        stack_hold_remaining_ms: 0,
        actions: [],
      },
    });
  });

  it("builds an IntentEnvelope Cast request with the intent oneof under intent.case", () => {
    const envelope: IntentEnvelope = {
      table_id: "ABC123",
      client_seq: 1,
      intent: { kind: "cast", player: 0, object: 1 },
    };
    const proto = intentEnvelopeToProto(envelope) as {
      tableId: string;
      clientSeq: bigint;
      intent: { intent: { case: string; value: unknown } };
    };
    expect(proto.tableId).toBe("ABC123");
    expect(proto.clientSeq).toBe(1n);
    expect(proto.intent.intent.case).toBe("cast");
    expect(proto.intent.intent.value).toEqual({ player: 0, object: 1 });
  });

  it("builds a target-carrying Intent with the nested WireTarget also wrapped", () => {
    const envelope: IntentEnvelope = {
      table_id: "ABC123",
      client_seq: 2,
      intent: { kind: "activate_ability", ability_index: 0, object: 3, player: 1, target: { kind: "object", id: 9 } },
    };
    const proto = intentEnvelopeToProto(envelope) as {
      intent: { intent: { case: string; value: { target: unknown } } };
    };
    expect(proto.intent.intent.case).toBe("activateAbility");
    expect(proto.intent.intent.value.target).toEqual({ kind: { case: "object", value: { id: 9 } } });
  });

  it("collapses ObjectAmount assignment arrays into [id, amount] tuples", () => {
    const proto = {
      event: {
        case: "combatDamageDivided",
        value: {
          attacker: 4,
          assignment: [
            { id: 1, amount: 2 },
            { id: 5, amount: 3 },
          ],
        },
      },
    };
    expect(fromProtoWire(proto)).toEqual({
      kind: "combat_damage_divided",
      attacker: 4,
      assignment: [
        [1, 2],
        [5, 3],
      ],
    });
  });

  it("collapses PlayerAmount arrays into [player, amount] tuples", () => {
    const proto = {
      event: {
        case: "spellDamageDivided",
        value: { spell: 7, assignment: [{ id: 2, amount: 1 }], players: [{ player: 0, amount: 4 }] },
      },
    };
    expect(fromProtoWire(proto)).toEqual({
      kind: "spell_damage_divided",
      spell: 7,
      assignment: [[2, 1]],
      players: [[0, 4]],
    });
  });

  it("unwraps ObjectIdList to a plain number array", () => {
    const proto = { id: 5n, kind: "cast", sacrificeChoices: { ids: [1, 2, 3] }, discardCount: 0 };
    expect(fromProtoWire(proto)).toEqual({ id: 5, kind: "cast", sacrifice_choices: [1, 2, 3], discard_count: 0 });
  });

  it("drops an unset oneof instead of emitting a null case", () => {
    expect(fromProtoWire({ kind: { case: undefined, value: undefined } })).toBeUndefined();
  });
});
