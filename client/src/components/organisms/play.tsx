import { useParams } from "@solidjs/router";
import { createEffect, createSignal, Show } from "solid-js";
import Board from "~/components/organisms/board";
import Lobby from "~/components/organisms/lobby";
import { useAuthGuard } from "~/guard";
import * as lobbyClient from "~/lib/lobbyClient";
import { tableId } from "~/net";

export default function Play() {
  const user = useAuthGuard();
  const params = useParams();
  const [started, setStarted] = createSignal(false);

  createEffect(() => {
    if (!user() || !params.table) return;
    let cancelled = false;
    void lobbyClient.lobbyState(tableId()).then((view) => {
      if (!cancelled && view?.started) setStarted(true);
    });
    return () => {
      cancelled = true;
    };
  });

  // Same race as Decks: Lobby mounts `decksAtom` — wait for a session first.
  return (
    <Show when={user()}>
      <Show when={started()} fallback={<Lobby onStarted={() => setStarted(true)} />}>
        <Board />
      </Show>
    </Show>
  );
}
