import { Effect, Schema as S } from "effect";
import { Command } from "foldkit";
import { CardNameSuggestionsFetched, CardTextFetched, InspectCardFetched } from "../board/messages";
import { formatMessage } from "../domain/i18n/message";
import { statusOf } from "../domain/rpc-client";
import type { Ack, IntentEnvelope, WireIntent } from "../domain/wire/types";
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

function ackMessage(ack: Ack) {
  if (ack.accepted) return IntentAcked();
  return IntentRejected({ reason: ack.reject_reason ? formatMessage(ack.reject_reason) : "That didn't work." });
}

export const SubmitIntent = Command.define("SubmitIntent", {
  args: { tableId: S.String, intent: WireIntentSchema },
  messages: [IntentAcked, IntentRejected],
  execute: ({ tableId, intent }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.submitIntent(tableId, nextEnvelope(tableId, intent)).pipe(
        Effect.map(ackMessage),
        Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
      );
    }),
});

export const SetYield = Command.define("SetYield", {
  args: { tableId: S.String, enabled: S.Boolean },
  messages: [IntentAcked, IntentRejected],
  execute: ({ tableId, enabled }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.setYield(tableId, { enabled }).pipe(
        Effect.map(ackMessage),
        Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
      );
    }),
});

export const SetTurnYield = Command.define("SetTurnYield", {
  args: { tableId: S.String, enabled: S.Boolean },
  messages: [IntentAcked, IntentRejected],
  execute: ({ tableId, enabled }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.setTurnYield(tableId, { enabled }).pipe(
        Effect.map(ackMessage),
        Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
      );
    }),
});

export const SetStackDwell = Command.define("SetStackDwell", {
  args: { tableId: S.String, dwelling: S.Boolean },
  messages: [IntentAcked, IntentRejected],
  execute: ({ tableId, dwelling }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.setStackDwell(tableId, { dwelling }).pipe(
        Effect.map(ackMessage),
        Effect.catch((error) => Effect.succeed(IntentRejected({ reason: failureReason(error) }))),
      );
    }),
});

export const FetchCardText = Command.define("FetchCardText", {
  args: { cardIds: S.Array(S.String) },
  messages: [CardTextFetched],
  execute: ({ cardIds }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.lookupCards([...cardIds]).pipe(
        Effect.map((cards) => CardTextFetched({ cards })),
        // ponytail: a failed lookup leaves those faces textless for the table — the ids are already
        // marked asked, so nothing retries. Frame, art and name still draw; add a retry if it bites.
        Effect.catch(() => Effect.succeed(CardTextFetched({ cards: [] }))),
      );
    }),
});

export const FetchInspectCard = Command.define("FetchInspectCard", {
  args: { cardId: S.String },
  messages: [InspectCardFetched],
  execute: ({ cardId }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.lookupCards([cardId]).pipe(
        Effect.map((cards) => InspectCardFetched({ card: cards[0] ?? null })),
        Effect.catch(() => Effect.succeed(InspectCardFetched({ card: null }))),
      );
    }),
});

const CARD_NAME_SUGGEST_LIMIT = 8;

/** Catalog typeahead for `choose_card_name` — suggestions assist; engine still accepts any name.
 *  Interruptible under one key: only the search for what is in the input now is worth finishing,
 *  so each keystroke cancels the one before it. */
export const SearchCardNames = Command.define("SearchCardNames", {
  args: { query: S.String },
  messages: [CardNameSuggestionsFetched],
  interrupt: true,
  execute: ({ query }) =>
    Effect.gen(function* () {
      const rpc = yield* RpcClient;
      return yield* rpc.searchCards({ q: query, limit: CARD_NAME_SUGGEST_LIMIT, offset: 0 }).pipe(
        Effect.map((cards) =>
          CardNameSuggestionsFetched({
            query,
            names: cards.map((c) => c.name),
          }),
        ),
        Effect.catch(() => Effect.succeed(CardNameSuggestionsFetched({ query, names: [] }))),
      );
    }),
});
