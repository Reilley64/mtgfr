import { Schema as S } from "effect";
import { m } from "foldkit/message";
import {
  type CardTextView,
  type MessageRef,
  MessageRef as MessageRefSchema,
  type VisibleEvent,
  type VisibleState,
} from "../domain/wire/types";

const VisibleStateSchema: S.Schema<VisibleState> = S.Any;
const VisibleEventsSchema: S.Schema<ReadonlyArray<VisibleEvent>> = S.Array(S.Any);
const CardTextSchema: S.Schema<ReadonlyArray<CardTextView>> = S.Array(S.Any);
const OptionalCardTextSchema: S.Schema<ReadonlyArray<CardTextView> | undefined> = S.optional(S.Array(S.Any));
const AutoActionsSchema: S.Schema<ReadonlyArray<MessageRef> | undefined> = S.optional(S.Array(MessageRefSchema));

export const ReceivedSnapshot = m("ReceivedSnapshot", {
  seq: S.Number,
  state: VisibleStateSchema,
  /** The viewer's own deck, plus any other seat's card this snapshot already shows them. */
  card_text: CardTextSchema,
});

export const ReceivedDelta = m("ReceivedDelta", {
  seq: S.Number,
  state: VisibleStateSchema,
  events: VisibleEventsSchema,
  auto_actions: AutoActionsSchema,
  /** Words for cards this frame made visible that aren't in the viewer's own deck — an
   * opponent's spell arriving on the stack. Merged into the book, never replacing it. */
  card_text: OptionalCardTextSchema,
});

export const StreamStatus = m("StreamStatus", { connected: S.Boolean });
export const StreamTerminalError = m("StreamTerminalError", { status: S.Number });
export const IntentAcked = m("IntentAcked");
export const IntentRejected = m("IntentRejected", { reason: S.String });

export const Message = S.Union([
  ReceivedSnapshot,
  ReceivedDelta,
  StreamStatus,
  StreamTerminalError,
  IntentAcked,
  IntentRejected,
]);
export type Message = typeof Message.Type;
