// The lobby poll, factored out of the Solid component so it can be unit-tested without dragging in
// `@solidjs/router` (which touches `window` at import). `Lobby.tsx` wires these into the component.

import * as Effect from "effect/Effect";
import * as Schedule from "effect/Schedule";
import * as Stream from "effect/Stream";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import type * as AtomRegistry from "effect/unstable/reactivity/AtomRegistry";
import type { LobbyView } from "~/api/generated";
import { type Client, client } from "~/effect/client";

// Poll the lobby once per second, sequentially, for as long as the atom stays subscribed. A failed
// tick retries (every second) rather than surfacing — the poll must survive transient server
// hiccups. Emits immediately then follows the schedule, so a claimed seat shows up right away.
// Three former poll-race guards are now structural: sequential repetition means requests never
// overlap (no in-flight flag); atom unmount interrupts the fiber so a late response can't touch a
// torn-down component (no disposed flag); a null table parks on a never-stream and the component
// stops subscribing once the game starts (no stop-on-started flag). `wire` is injectable so a test
// can count requests against a stub client (mirrors effect/stream.ts).
export const pollStream = (table: string, wire: Client) =>
  Stream.fromEffectSchedule(
    wire.lobbyState(table, {}).pipe(Effect.retry(Schedule.spaced("1 second"))),
    Schedule.spaced("1 second"),
  );

export const lobbyPollFamily = Atom.family((table: string | null) =>
  Atom.make(table == null ? (Stream.never as Stream.Stream<LobbyView>) : pollStream(table, client)),
);
export type PollAtom = ReturnType<typeof lobbyPollFamily>;

// Subscribe to a poll atom, delivering each successful view to `onView`, until the returned
// unsubscribe is called. `{ immediate: true }` is load-bearing: `registry.subscribe` on its own only
// attaches a listener — it's the immediate *read* of the atom's value that computes it and thereby
// launches the poll stream's fiber. Omit it (as the old `useAtomSubscribe` call did) and the stream
// never starts: the lobby polled exactly once at most and then went silent, so guests never saw
// ready-state changes or the game start. This is the whole `useAtomSubscribe(..., immediate)` in a
// form a test can drive directly against a stub-client atom.
export function startLobbyPoll(
  registry: AtomRegistry.AtomRegistry,
  atom: PollAtom,
  onView: (view: LobbyView) => void,
): () => void {
  return registry.subscribe(
    atom,
    (res) => {
      if (AsyncResult.isSuccess(res)) onView(res.value);
    },
    { immediate: true },
  );
}
