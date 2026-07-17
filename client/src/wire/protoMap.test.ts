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
    expect(proto.intent.intent.value).toMatchObject({ player: 0, object: 1, x: 0, kicked: false });
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
    expect(proto.intent.intent.value.target).toMatchObject({ kind: { case: "object", value: { id: 9 } } });
  });

  it("coerces take_action id to bigint (proto uint64 / Effect Schema.BigInt)", () => {
    const envelope: IntentEnvelope = {
      table_id: "ABC123",
      client_seq: 3,
      intent: { kind: "take_action", player: 0, id: 0, sacrifice: [] },
    };
    const proto = intentEnvelopeToProto(envelope) as {
      intent: { intent: { case: string; value: { id: unknown; player: number; attackers: unknown[] } } };
    };
    expect(proto.intent.intent.case).toBe("takeAction");
    expect(proto.intent.intent.value.id).toBe(0n);
    expect(proto.intent.intent.value.player).toBe(0);
    expect(proto.intent.intent.value.attackers).toEqual([]);
  });

  it("omits null optionals and fills proto3 defaults so Effect Schema accepts take_action", () => {
    const envelope: IntentEnvelope = {
      table_id: "ABC123",
      client_seq: 4,
      intent: {
        kind: "take_action",
        player: 0,
        id: 7,
        target: null,
        x: 0,
        modes: [],
        sacrifice: [],
        discard_cost: [],
        graveyard_exile: [],
      },
    };
    const proto = intentEnvelopeToProto(envelope) as {
      intent: { intent: { value: Record<string, unknown> } };
    };
    expect(proto.intent.intent.value).not.toHaveProperty("target");
    expect(proto.intent.intent.value.blocks).toEqual([]);
  });

  it("fills sparse Cast defaults (x/modes/flags) for Effect Schema", () => {
    const envelope: IntentEnvelope = {
      table_id: "ABC123",
      client_seq: 5,
      intent: { kind: "cast", player: 0, object: 1 },
    };
    const proto = intentEnvelopeToProto(envelope) as {
      intent: { intent: { value: { x: number; kicked: boolean; modes: unknown[] } } };
    };
    expect(proto.intent.intent.value.x).toBe(0);
    expect(proto.intent.intent.value.kicked).toBe(false);
    expect(proto.intent.intent.value.modes).toEqual([]);
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

  it("flattens a rich snapshot: pending choice, ObjectIdList actions, nested WireKind/WireTarget", () => {
    const proto = {
      frame: {
        case: "snapshot",
        value: {
          seq: 9n,
          state: {
            viewer: 0,
            activePlayer: 0,
            step: 3,
            priority: 0,
            players: [
              {
                player: 0,
                username: "p0",
                life: 40,
                commanderTax: 0,
                lost: false,
                handCount: 0,
                libraryCount: 99,
                manaPool: { colored: [0, 0, 0, 0, 0], colorless: 0, any: 0, either: [], ofColors: [] },
                commanderDamage: [],
              },
            ],
            objects: [
              {
                id: 10,
                zone: 1,
                owner: 0,
                controller: 0,
                cardId: "card-1",
                name: "Bear",
                print: "print-1",
                kind: { kind: { case: "creature", value: { power: 2, toughness: 2 } } },
                manaCost: { generic: 1, colored: [0, 0, 0, 0, 1], hasX: false },
                needsTarget: false,
                tapped: false,
                summoningSick: false,
                hasHaste: false,
                keywords: ["trample"],
                power: 2,
                toughness: 2,
                loyalty: 0,
                plusCounters: 0,
                markedDamage: 0,
                isCommander: false,
                goaded: false,
                tapsForMana: false,
                prepared: false,
                phasedOut: false,
                faceDown: false,
                modifiers: [],
              },
            ],
            stack: [
              {
                kind: "spell",
                source: 10,
                controller: 0,
                label: "Shock",
                target: { kind: { case: "player", value: { player: 1 } } },
              },
            ],
            combat: { attackers: [], blocks: [], attackersDeclared: false, blockersDeclared: [] },
            canAct: true,
            yielded: false,
            turnYielded: false,
            stackHoldRemainingMs: 0,
            pendingChoice: {
              choice: {
                case: "chooseTarget",
                value: {
                  player: 0,
                  source: 10,
                  label: "Deal 2",
                  items: [{ id: 11, label: "Goblin" }],
                  optional: false,
                },
              },
            },
            actions: [
              {
                id: 42n,
                kind: "cast",
                object: 12,
                section: "hand",
                label: "Lightning Bolt",
                needsTarget: true,
                targets: [{ kind: { case: "player", value: { player: 1 } } }],
                sacrificeChoices: { ids: [13, 14] },
                discardCount: 0,
                graveyardExileMin: 0,
                graveyardExileMax: 0,
                hasX: false,
                autoTap: [],
                requiredAttacks: [],
              },
            ],
          },
        },
      },
    };

    const frame = streamFrameFromProto(proto);
    expect(frame).toMatchObject({
      frame: "snapshot",
      seq: 9,
      state: {
        viewer: 0,
        objects: [{ id: 10, kind: { kind: "creature", power: 2, toughness: 2 }, keywords: ["trample"] }],
        stack: [{ kind: "spell", target: { kind: "player", player: 1 } }],
        pending_choice: {
          kind: "choose_target",
          player: 0,
          source: 10,
          label: "Deal 2",
          items: [{ id: 11, label: "Goblin" }],
          optional: false,
        },
        actions: [
          {
            id: 42,
            kind: "cast",
            sacrifice_choices: [13, 14],
            targets: [{ kind: "player", player: 1 }],
          },
        ],
      },
    });
  });

  it("flattens a delta with divided-damage events and auto_actions", () => {
    const proto = {
      frame: {
        case: "delta",
        value: {
          seq: 10n,
          autoActions: ["auto-pass"],
          events: [
            {
              event: {
                case: "combatDamageDivided",
                value: { attacker: 10, assignment: [{ id: 11, amount: 2 }] },
              },
            },
            {
              event: {
                case: "spellDamageDivided",
                value: {
                  spell: 20,
                  assignment: [{ id: 11, amount: 1 }],
                  players: [{ player: 1, amount: 3 }],
                },
              },
            },
          ],
          state: {
            viewer: 0,
            activePlayer: 0,
            step: 0,
            priority: 0,
            players: [],
            objects: [],
            stack: [],
            canAct: false,
            yielded: false,
            turnYielded: false,
            stackHoldRemainingMs: 0,
            actions: [],
          },
        },
      },
    };

    expect(streamFrameFromProto(proto)).toMatchObject({
      frame: "delta",
      seq: 10,
      auto_actions: ["auto-pass"],
      events: [
        { kind: "combat_damage_divided", attacker: 10, assignment: [[11, 2]] },
        { kind: "spell_damage_divided", spell: 20, assignment: [[11, 1]], players: [[1, 3]] },
      ],
    });
  });
});
