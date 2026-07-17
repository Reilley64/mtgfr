import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import { humanReason, rejectMessageFor } from "~/controllers/reject";
import { client } from "~/effect/client";
import { buildIntentEnvelope, tableId } from "~/net";
import { setReject } from "~/store";
import type { WireIntent } from "~/wire/types";

// The one ack/failure fold shared by every server call: surface the reason in the reject
// banner, and resolve to whether the server accepted (so callers can undo optimistic UI).
const ackToReject = (ack: { accepted: boolean; reason?: string | null }): boolean => {
  setReject(ack.accepted ? null : ack.reason ? humanReason(ack.reason) : "That didn't work.");
  return ack.accepted;
};
const failureToReject = (failure: unknown): boolean => {
  setReject(rejectMessageFor(failure));
  return false;
};

/** Submit one intent, folding its ack/failure into the reject line. `Effect.match` makes it never
 * fail, so `useAtomSet(..., { mode: "promise" })` always resolves — callers `await` or fire-and-forget. */
export const submitIntentFn = Atom.fn((intent: WireIntent) =>
  client
    .submitIntent(tableId(), buildIntentEnvelope(intent))
    .pipe(Effect.match({ onSuccess: ackToReject, onFailure: failureToReject })),
);

/** Toggle the viewer's "don't care" yield: the server auto-passes their seat while the current
 * stack resolves (and clears the flag once it empties). Resolves to whether it was accepted. */
export const setYieldFn = Atom.fn((p: { enabled: boolean }) =>
  client
    .setYield(tableId(), { enabled: p.enabled })
    .pipe(Effect.match({ onSuccess: ackToReject, onFailure: failureToReject })),
);

/** Toggle turn yield (ADR 0029): auto-pass until this seat's turn / until they act. */
export const setTurnYieldFn = Atom.fn((p: { enabled: boolean }) =>
  client
    .setTurnYield(tableId(), { enabled: p.enabled })
    .pipe(Effect.match({ onSuccess: ackToReject, onFailure: failureToReject })),
);

/** Helpless stack dwell: pause the stack-hold while hovering, if the seat has no meaningful action. */
export const setStackDwellFn = Atom.fn((p: { dwelling: boolean }) =>
  client.setStackDwell(tableId(), { dwelling: p.dwelling }).pipe(
    Effect.match({
      onSuccess: (ack) => ack.accepted,
      onFailure: () => false,
    }),
  ),
);
