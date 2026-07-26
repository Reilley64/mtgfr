import * as Schema from "effect/Schema";
import { describe, expect, it } from "vitest";
import { catalogCardsFromProto, fromProtoWire, intentEnvelopeToProto, seedRequestToProto, streamFrameFromProto } from "./protoMap";
import type { ActionView, CatalogCard, IntentEnvelope, StreamFrame } from "./types";
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

  it("defaults missing commander_damage_enabled on snapshots to true", () => {
    const frame = streamFrameFromProto({
      frame: {
        case: "snapshot",
        value: {
          seq: 1n,
          state: {},
        },
      },
    });

    expect(frame).toMatchObject({
      frame: "snapshot",
      state: {
        commander_damage_enabled: true,
      },
    });
  });

  it("decodes commander damage toggle from visible snapshots", () => {
    const frame = streamFrameFromProto({
      frame: {
        case: "snapshot",
        value: {
          seq: 1n,
          state: {
            commanderDamageEnabled: false,
          },
        },
      },
    });

    expect(frame).toMatchObject({
      frame: "snapshot",
      state: {
        commander_damage_enabled: false,
      },
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
        set: "tst",
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

describe("seedRequestToProto", () => {
  it("maps commander_damage_enabled to the proto seed flag", () => {
    const proto = seedRequestToProto({
      table_id: "T1",
      host_user_id: 17,
      seats: [],
      commander_damage_enabled: false,
    });

    expect(proto.tableId).toBe("T1");
    expect(proto.hostUserId).toBe(17n);
    expect(proto.commanderDamageEnabled).toBe(false);
  });
});
