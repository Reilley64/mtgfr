import { useAtomSubscribe } from "@effect/atom-solid";
import { useParams } from "@solidjs/router";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createSignal, Show } from "solid-js";
import Board from "~/components/organisms/board";
import { client } from "~/effect/client";
import { useAuthGuard } from "~/guard";
import Lobby from "~/components/organisms/lobby";
import { tableId } from "~/net";

// Failures fold to "not started"; lobby poll recovers. `immediate` is required or the atom never runs.
const startedCheckFamily = Atom.family((table: string) => Atom.make(client.lobbyState(table, {})));

export default function Play() {
  useAuthGuard();
  const params = useParams();
  const [started, setStarted] = createSignal(false);

  if (params.table) {
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
