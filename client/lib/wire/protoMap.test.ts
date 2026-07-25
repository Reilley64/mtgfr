import { describe, expect, it } from "vitest";
import * as Schema from "effect/Schema";
import { fromProtoWire, intentEnvelopeToProto } from "./protoMap";
import { MessageRef } from "./types";
import type { ActionView, IntentEnvelope } from "./types";

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
