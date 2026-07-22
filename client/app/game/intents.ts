import { Effect, Schema as S } from "effect";
import { Command } from "foldkit";
import { humanReason } from "../../lib/reject";
import { statusOf } from "../../lib/rpc-client";
import type { IntentEnvelope, WireIntent } from "../../lib/wire/types";
import { InspectCardFetched } from "../board/messages";
import { RpcClient } from "../resources";
import { IntentAcked, IntentRejected } from "./messages";

const WireIntentSchema: S.Schema<WireIntent> = S.Any;

let clientSeq = 0;

function nextEnvelope(tableId: string, intent: WireIntent): IntentEnvelope {
  return { table_id: tableId, client_seq: ++clientSeq, intent };
}

function failureReason(error: unknown): string {
  if (statusOf(error) === 401) return "Session expired — sign in again.";
  return "Couldn't reach the table.";
}

function ackMessage(ack: { accepted: boolean; reason?: string | null }) {
  if (ack.accepted) return IntentAcked();
  return IntentRejected({ reason: ack.reason ? humanReason(ack.reason) : "That didn't work." });
}

export const SubmitIntent = Command.define(
  "SubmitIntent",
  { tableId: S.String, intent: WireIntentSchema },
  IntentAcked,
  IntentRejected,
)(({ tableId, intent }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.submitIntent(tableId, nextEnvelope(tableId, intent)).pipe(
      Effect.map(ackMessage),
      Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
    );
  }),
);

export const SetYield = Command.define(
  "SetYield",
  { tableId: S.String, enabled: S.Boolean },
  IntentAcked,
  IntentRejected,
)(({ tableId, enabled }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.setYield(tableId, { enabled }).pipe(
      Effect.map(ackMessage),
      Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
    );
  }),
);

export const SetTurnYield = Command.define(
  "SetTurnYield",
  { tableId: S.String, enabled: S.Boolean },
  IntentAcked,
  IntentRejected,
)(({ tableId, enabled }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.setTurnYield(tableId, { enabled }).pipe(
      Effect.map(ackMessage),
      Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
    );
  }),
);

export const SetStackDwell = Command.define(
  "SetStackDwell",
  { tableId: S.String, dwelling: S.Boolean },
  IntentAcked,
  IntentRejected,
)(({ tableId, dwelling }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.setStackDwell(tableId, { dwelling }).pipe(
      Effect.map(ackMessage),
      Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
    );
  }),
);

export const FetchInspectCard = Command.define(
  "FetchInspectCard",
  { cardId: S.String },
  InspectCardFetched,
)(({ cardId }) =>
  Effect.gen(function* () {
    const rpc = yield* RpcClient;
    return yield* rpc.lookupCards([cardId]).pipe(
      Effect.map((cards) => InspectCardFetched({ card: cards[0] ?? null })),
      Effect.catch(() => Effect.succeed(InspectCardFetched({ card: null }))),
    );
  }),
);
