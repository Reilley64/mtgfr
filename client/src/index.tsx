// App shell + routing. The deck manager is home; auth is public; the builder deep-links by id;
// /play runs the lobby → board handoff. Protected screens redirect to /login via useAuthGuard.

import { useAtomSubscribe } from "@effect/atom-solid";
import { Route, Router, useParams } from "@solidjs/router";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { render } from "solid-js/web";
import Auth from "~/Auth";
import Board from "~/Board";
import DeckBuilder from "~/DeckBuilder";
import Decks from "~/Decks";
import { client } from "~/effect/client";
import "~/global.css";
import { useAuthGuard } from "~/guard";
import Lobby from "~/Lobby";
import { openModalWhenReady } from "~/lib/modalDialog";
import { tableId } from "~/net";

// One-shot "did this table already start?" check. Any failure folds to "not started" — we fall
// through to the lobby, whose poll will recover the real state (before, this was an unhandled
// rejection; folding to not-started is the intended behavior).
const startedCheckFamily = Atom.family((table: string) => Atom.make(client.lobbyState(table, {})));

/** The play surface: show the lobby until the game starts, then hand off to the Board. A reload
 * mid-game (table in the URL, already started) skips straight to the board. */
function Play() {
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

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing");

/** Landscape-first: portrait phones get a native modal so the app underneath is inert + focus-trapped.
 * Escape is swallowed — rotating is the only dismiss. Matches DESIGN.md The Landscape Rule. */
function PortraitGate() {
  let dialog!: HTMLDialogElement;
  let cancelOpen: (() => void) | undefined;

  onMount(() => {
    const mq = window.matchMedia("(orientation: portrait) and (max-width: 900px)");
    const sync = () => {
      cancelOpen?.();
      cancelOpen = undefined;
      if (mq.matches) {
        if (dialog.open) return;
        cancelOpen = openModalWhenReady(dialog);
        return;
      }
      if (dialog.open) dialog.close();
    };
    sync();
    mq.addEventListener("change", sync);
    onCleanup(() => {
      cancelOpen?.();
      mq.removeEventListener("change", sync);
    });
  });

  return (
    <dialog
      ref={dialog}
      class="portrait-gate bg-forest-floor font-sans text-body text-snow"
      aria-labelledby="portrait-gate-title"
      onCancel={(e) => e.preventDefault()}
    >
      <div id="portrait-gate-title" class="text-title">
        Rotate to landscape
      </div>
      <div class="max-w-[28ch] text-label text-lichen">
        The table and deck builder are built for horizontal screens. Turn your device sideways to continue.
      </div>
    </dialog>
  );
}

render(
  () => (
    <>
      <PortraitGate />
      <Router>
        <Route path="/" component={Decks} />
        <Route path="/login" component={Auth} />
        <Route path="/decks/new" component={DeckBuilder} />
        <Route path="/decks/:id" component={DeckBuilder} />
        <Route path="/play" component={Play} />
        <Route path="/play/:table" component={Play} />
      </Router>
    </>
  ),
  root,
);
