import { create, toBinary } from "@bufbuild/protobuf";
import * as Schema from "effect/Schema";
import { describe, expect, it } from "vitest";
import { PendingChoiceViewMayReturnFromGraveyardSchema } from "./generated/mtgfr/v1/stream_pb";
import { catalogCardsFromProto, fromProtoWire, intentEnvelopeToProto } from "./protoMap";
import type { ActionView, CatalogCard, IntentEnvelope } from "./types";
import { MessageRef } from "./types";

describe("fromProtoWire", () => {
  it("coerces proto bigint action ids to browser numbers", () => {
    const state = fromProtoWire<{ actions: ActionView[] }>({
      actions: [{ id: 123n, kind: "activate", label: "Scry 1", needsTarget: false, section: "battlefield" }],
    });

    expect(state.actions[0]?.id).toBe(123);
    expect(typeof state.actions[0]?.id).toBe("number");
  });

  it("decodes MessageRef labels and auto action messages", () => {
    const frame = fromProtoWire({
      frame: {
        case: "delta",
        value: {
          autoActions: [
            {
              key: "auto.sacrificed_forced",
              params: [{ name: "name", value: { case: "stringValue", value: "Goblin" } }],
              children: [],
            },
          ],
          state: {
            actions: [
              {
                id: 123n,
                kind: "activate",
                label: {
                  key: "effect.draw_cards",
                  params: [{ name: "count", value: { case: "intValue", value: 2n } }],
                  children: [],
                },
                needsTarget: false,
                section: "battlefield",
              },
            ],
            pendingChoice: {
              choice: {
                case: "chooseMode",
                value: {
                  labels: [
                    {
                      key: "effect.discard",
                      params: [{ name: "count", value: { case: "intValue", value: 1n } }],
                      children: [],
                    },
                  ],
                  player: 0,
                  source: 7,
                },
              },
            },
          },
        },
      },
    });

    expect(frame).toMatchObject({
      frame: "delta",
      auto_actions: [
        {
          key: "auto.sacrificed_forced",
          params: [{ name: "name", string_value: "Goblin" }],
          children: [],
        },
      ],
      state: {
        actions: [
          {
            label: {
              key: "effect.draw_cards",
              params: [{ name: "count", int_value: 2 }],
              children: [],
            },
          },
        ],
        pending_choice: {
          kind: "choose_mode",
          labels: [
            {
              key: "effect.discard",
              params: [{ name: "count", int_value: 1 }],
              children: [],
            },
          ],
        },
      },
    });
  });

  it("decodes Ack.rejectReason into reject_reason MessageRef", () => {
    const ack = fromProtoWire({
      accepted: false,
      rejectReason: {
        key: "reject.illegal_target",
        params: [],
        children: [],
      },
    });

    expect(ack).toEqual({
      accepted: false,
      reject_reason: {
        key: "reject.illegal_target",
        params: [],
        children: [],
      },
    });
  });

  it("decodes may_return_from_graveyard mandatory from proto choice payloads", () => {
    const frame = fromProtoWire<{
      state: { pending_choice: { kind: string; mandatory?: boolean; items: Array<{ id: number; label: string }> } };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "mayReturnFromGraveyard",
            value: {
              player: 0,
              source: 7,
              mandatory: true,
              items: [{ id: 11, label: "Forest" }],
            },
          },
        },
      },
    });

    expect(frame.state.pending_choice).toEqual({
      kind: "may_return_from_graveyard",
      player: 0,
      source: 7,
      mandatory: true,
      items: [{ id: 11, label: "Forest" }],
    });
  });

  it("encodes may_return_from_graveyard as items=3 and mandatory=4", () => {
    const binary = toBinary(
      PendingChoiceViewMayReturnFromGraveyardSchema,
      create(PendingChoiceViewMayReturnFromGraveyardSchema, {
        player: 0,
        source: 7,
        items: [{ id: 11, label: "Forest" }],
        mandatory: true,
      }),
    );

    expect(Array.from(binary)).toEqual([16, 7, 26, 10, 8, 11, 18, 6, 70, 111, 114, 101, 115, 116, 32, 1]);
  });

  it("decodes choose_copy_target counter-primer wording from proto choice payloads", () => {
    const frame = fromProtoWire<{
      state: {
        pending_choice: {
          kind: string;
          put_counter_on_creature?: boolean;
          items: Array<{ id: number; label: string }>;
        };
      };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "chooseCopyTarget",
            value: {
              player: 0,
              source: 7,
              putCounterOnCreature: true,
              items: [{ id: 11, label: "Forest" }],
            },
          },
        },
      },
    });

    expect(frame.state.pending_choice).toEqual({
      kind: "choose_copy_target",
      player: 0,
      source: 7,
      put_counter_on_creature: true,
      items: [{ id: 11, label: "Forest" }],
    });
  });

  it("decodes choose_exiled_dig_to_cast_free hand-pick wording from proto choice payloads", () => {
    const frame = fromProtoWire<{
      state: {
        pending_choice: {
          kind: string;
          from_opponent_hand?: boolean;
          items: Array<{ id: number; label: string }>;
        };
      };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "chooseExiledDigToCastFree",
            value: {
              player: 0,
              source: 7,
              fromOpponentHand: true,
              items: [{ id: 11, label: "Lightning Bolt" }],
            },
          },
        },
      },
    });

    expect(frame.state.pending_choice).toEqual({
      kind: "choose_exiled_dig_to_cast_free",
      player: 0,
      source: 7,
      from_opponent_hand: true,
      items: [{ id: 11, label: "Lightning Bolt" }],
    });
  });

  it("decodes the Raging River pile discriminators from proto choice payloads", () => {
    const partition = fromProtoWire<{
      state: { pending_choice: { kind: string; into_piles?: boolean } };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "partitionRevealed",
            value: { player: 1, source: 7, intoPiles: true, items: [{ id: 11, label: "Grizzly Bears" }] },
          },
        },
      },
    });
    expect(partition.state.pending_choice).toEqual({
      kind: "partition_revealed",
      player: 1,
      source: 7,
      into_piles: true,
      items: [{ id: 11, label: "Grizzly Bears" }],
    });

    const pilePick = fromProtoWire<{
      state: { pending_choice: { kind: string; attacker?: { id: number; label: string } } };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "choosePileForHand",
            value: {
              player: 0,
              source: 7,
              pileA: [{ id: 11, label: "Wall of Stone" }],
              pileB: [],
              attacker: { id: 12, label: "Shivan Dragon" },
            },
          },
        },
      },
    });
    expect(pilePick.state.pending_choice).toEqual({
      kind: "choose_pile_for_hand",
      player: 0,
      source: 7,
      pile_a: [{ id: 11, label: "Wall of Stone" }],
      pile_b: [],
      attacker: { id: 12, label: "Shivan Dragon" },
    });
  });

  it("decodes choose_copy_target block-re-aim wording from proto choice payloads", () => {
    const frame = fromProtoWire<{
      state: {
        pending_choice: {
          kind: string;
          choose_block_target?: boolean;
          items: Array<{ id: number; label: string }>;
        };
      };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "chooseCopyTarget",
            value: {
              player: 0,
              source: 7,
              chooseBlockTarget: true,
              items: [{ id: 11, label: "Grizzly Bears" }],
            },
          },
        },
      },
    });

    expect(frame.state.pending_choice).toEqual({
      kind: "choose_copy_target",
      player: 0,
      source: 7,
      choose_block_target: true,
      items: [{ id: 11, label: "Grizzly Bears" }],
    });
  });

  it("decodes may_exile_discarded_to_play from proto choice payloads", () => {
    const frame = fromProtoWire<{
      state: {
        pending_choice: {
          kind: string;
          player: number;
          source: number;
          items: Array<{ id: number; label: string }>;
        };
      };
    }>({
      state: {
        pendingChoice: {
          choice: {
            case: "mayExileDiscardedToPlay",
            value: {
              player: 0,
              source: 7,
              items: [{ id: 11, label: "Lightning Bolt" }],
            },
          },
        },
      },
    });

    expect(frame.state.pending_choice).toEqual({
      kind: "may_exile_discarded_to_play",
      player: 0,
      source: 7,
      items: [{ id: 11, label: "Lightning Bolt" }],
    });
  });
});

describe("catalogCardsFromProto", () => {
  it("decodes catalog summaries as MessageRef arrays", () => {
    const cards = catalogCardsFromProto([
      {
        id: "card-1",
        defaultPrint: "print-1",
        name: "Summary Card",
        cost: { generic: 1, colored: [0, 0, 0, 0, 0], hasX: false, xSymbols: 0 },
        kind: { kind: { case: "creature", value: { power: 2, toughness: 2 } } },
        keywords: ["ward:2"],
        summary: [
          {
            key: "keyword.ward",
            params: [{ name: "amount", value: { case: "intValue", value: 2n } }],
            children: [],
          },
          {
            key: "effect.sequence",
            params: [],
            children: [
              {
                key: "effect.draw_cards",
                params: [{ name: "count", value: { case: "intValue", value: 1n } }],
                children: [],
              },
            ],
          },
        ],
        legendary: false,
        colorIdentity: [],
        set: "",
        sets: ["tst"],
        subtypes: [],
        otags: [],
      },
    ]);

    expect(cards[0]).toMatchObject({
      summary: [
        {
          key: "keyword.ward",
          params: [{ name: "amount", int_value: 2 }],
          children: [],
        },
        {
          key: "effect.sequence",
          params: [],
          children: [
            {
              key: "effect.draw_cards",
              params: [{ name: "count", int_value: 1 }],
              children: [],
            },
          ],
        },
      ],
      set: "",
      sets: ["tst"],
    } satisfies Partial<CatalogCard>);
  });
});

describe("MessageRef schema", () => {
  it("rejects bare strings", () => {
    expect(() => Schema.decodeUnknownSync(MessageRef)("Scry 1")).toThrow();
  });
});

describe("intentEnvelopeToProto", () => {
  it("coerces take_action id to bigint through protobuf create", () => {
    const envelope: IntentEnvelope = {
      table_id: "T1",
      client_seq: 7,
      intent: {
        kind: "take_action",
        player: 0,
        id: 91,
        target: null,
        x: 0,
        modes: [],
        sacrifice: [],
        discard_cost: [],
        graveyard_exile: [],
      },
    };

    const proto = intentEnvelopeToProto(envelope);
    const intent = proto.intent?.intent;

    expect(proto.clientSeq).toBe(7n);
    expect(intent?.case).toBe("takeAction");
    if (intent?.case !== "takeAction") return;
    expect(intent.value.id).toBe(91n);
    expect(typeof intent.value.id).toBe("bigint");
  });
});
