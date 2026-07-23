import { describe, expect, it } from "vitest";
import { fromProtoWire, intentEnvelopeToProto } from "./protoMap";
import type { ActionView, IntentEnvelope } from "./types";

describe("fromProtoWire", () => {
  it("coerces proto bigint action ids to browser numbers", () => {
    const state = fromProtoWire<{ actions: ActionView[] }>({
      actions: [{ id: 123n, kind: "activate", label: "Scry 1", needsTarget: false, section: "battlefield" }],
    });

    expect(state.actions[0]?.id).toBe(123);
    expect(typeof state.actions[0]?.id).toBe("number");
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
