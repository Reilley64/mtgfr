// The lobby poll, factored out of the Solid component so it can be unit-tested without dragging in
// `@solidjs/router` (which touches `window` at import). `Lobby.tsx` wires these into the component.

import * as Effect from "effect/Effect";
import * as Schedule from "effect/Schedule";
import * as Stream from "effect/Stream";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import type * as AtomRegistry from "effect/unstable/reactivity/AtomRegistry";
import * as lobbyClient from "~/lib/lobbyClient";
import type { LobbyView } from "~/lib/lobbyTypes";

export const pollStream = (table: string) =>
  Stream.fromEffectSchedule(
    Effect.tryPromise(() => lobbyClient.lobbyState(table)).pipe(
      Effect.flatMap((v) => (v == null ? Effect.fail(new Error("lobby poll failed")) : Effect.succeed(v))),
      Effect.retry(Schedule.spaced("1 second")),
    ),
    Schedule.spaced("1 second"),
  );

export const lobbyPollFamily = Atom.family((table: string | null) =>
  Atom.make(table == null ? (Stream.never as Stream.Stream<LobbyView>) : pollStream(table)),
);
export type PollAtom = ReturnType<typeof lobbyPollFamily>;

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
