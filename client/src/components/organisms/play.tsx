import { useParams } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import Board from "~/components/organisms/board";
import Lobby from "~/components/organisms/lobby";
import { RequireAuth } from "~/guard";
import * as lobbyClient from "~/lib/lobbyClient";
import { tableId } from "~/net";

export default function Play() {
  return <RequireAuth>{() => <PlaySignedIn />}</RequireAuth>;
}

function PlaySignedIn() {
  const params = useParams();
  const [started, setStarted] = createSignal(false);

  createEffect(() => {
    if (!params.table) return;
    let cancelled = false;
    void lobbyClient.lobbyState(tableId()).then((view) => {
      if (!cancelled && view?.started) setStarted(true);
    });
    return () => {
      cancelled = true;
    };
  });

  return (
    <Show when={started()} fallback={<Lobby onStarted={() => setStarted(true)} />}>
      <Board />
    </Show>
  );
}
