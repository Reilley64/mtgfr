/** The play surface: show the lobby until the game starts, then hand off to the Board. A reload
 * mid-game (table in the URL, already started) skips straight to the board. */

import { useAtomSubscribe } from "@effect/atom-solid";
import { useParams } from "@solidjs/router";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createSignal, Show } from "solid-js";
import Board from "~/Board";
import { client } from "~/effect/client";
import { useAuthGuard } from "~/guard";
import Lobby from "~/Lobby";
import { tableId } from "~/net";

// One-shot "did this table already start?" check. Any failure folds to "not started" — we fall
// through to the lobby, whose poll will recover the real state (before, this was an unhandled
// rejection; folding to not-started is the intended behavior).
const startedCheckFamily = Atom.family((table: string) => Atom.make(client.lobbyState(table, {})));

export default function Play() {
  useAuthGuard();
  const params = useParams();
  const [started, setStarted] = createSignal(false);

  // Only worth checking when arriving with a table in the path, exactly as before.
  if (params.table) {
    // `immediate: true` is required, not cosmetic: subscribing alone never reads (computes) the
    // atom, so without it the one-shot lobbyState effect never runs and this fast path is dead.
    useAtomSubscribe(
      () => startedCheckFamily(tableId()),
      (res) => {
        if (AsyncResult.isSuccess(res) && res.value.started) setStarted(true);
      },
      { immediate: true },
    );
  }

  return (
    <Show when={started()} fallback={<Lobby onStarted={() => setStarted(true)} />}>
      <Board />
    </Show>
  );
}
