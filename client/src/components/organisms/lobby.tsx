// The pre-game lobby: create a table (shareable code), claim a seat with one of your saved
// decks, ready up, and — for the host — start the game. State is polled from the server
// (`GET /tables/{table}/lobby`) until `started`, at which point `onStarted` hands off to the Board.
// Identity is the session cookie; the claimed seat is reflected in the URL.

import { RegistryContext, useAtomResource, useAtomSet } from "@effect/atom-solid";
import { useParams, useSearchParams } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createEffect, createSignal, For, onCleanup, Show, useContext } from "solid-js";
import type { LobbyView } from "~/api/generated";
import { decksAtom } from "~/atoms";
import { client, orNull } from "~/effect/client";
import { cn } from "~/lib/cn";
import { lobbyIsHost } from "~/lib/lobby";
import { lobbyPollFamily, startLobbyPoll } from "~/lobbyPoll";
import { parseTableCode, setTableUrl } from "~/net";
import { Button, Felt, Field, Panel } from "~/components/atoms";

// The three seat-claiming wire actions plus table creation, as function atoms. Logical outcomes
// (TableFull, NotHost, …) come back as a 200 `LobbyView` with an `error` field; a *transport*
// failure (500, network drop) would otherwise reject the promise, so `orNull` folds it to `null`
// here. Every one of these promises resolves — the component branches on the value, and never
// wraps an `await` in a `try`/`catch` (ADR 0019).
const createTableFn = Atom.fn(() => orNull(client.createTable({})));
const joinTableFn = Atom.fn((p: { table_id: string; deck_id: number }) => orNull(client.joinTable({ payload: p })));
const readyUpFn = Atom.fn((p: { table_id: string; ready: boolean }) => orNull(client.readyUp({ payload: p })));
const startGameFn = Atom.fn((p: { table_id: string }) => orNull(client.startGame({ payload: p })));

// Copy the table code. A browser with no Clipboard API (older, or an insecure context) throws
// synchronously on `navigator.clipboard.writeText`; a denied permission rejects. `tryPromise`
// catches both, so the caller sees one boolean and there is no feature-detect branch to keep in
// sync with the failure path. `false` → reveal the manual-copy input.
const copyTextFn = Atom.fn((text: string) =>
  Effect.tryPromise(() => navigator.clipboard.writeText(text)).pipe(
    Effect.as(true),
    Effect.catch(() => Effect.succeed(false)),
  ),
);

// A transport failure, named like every other lobby outcome so it renders through `humanError`.
const UNREACHABLE = "Unreachable";

// A seat row's three columns: seat number, deck, badges.
const SEAT_ROW = cn("grid grid-cols-[70px_1fr_auto] gap-sm rounded-control bg-glass-dim px-md py-xs");

export default function Lobby(props: { onStarted: () => void }) {
  const routeParams = useParams();
  // A table id in the path means we're joining an existing table; otherwise we're at the landing.
  const [table, setTable] = createSignal<string | null>(routeParams.table ?? null);
  const [lobby, setLobby] = createSignal<LobbyView | null>(null);
  const [decks] = useAtomResource(() => decksAtom);
  // Deck to bring: the one passed from the deck manager (?deck=ID). Read reactively from the router
  // (raw location.search can be stale right after an SPA navigation). null only when truly absent.
  const [params] = useSearchParams();
  const urlDeck = () => {
    const n = Number(params.deck);
    return params.deck != null && Number.isInteger(n) ? n : null;
  };
  // Manual override only for the share-link claim path (no ?deck to inherit there).
  const [override, setOverride] = createSignal<number | null>(null);
  const deckId = () => override() ?? urlDeck();
  const setDeckId = setOverride;
  const [code, setCode] = createSignal(""); // the table code typed on the Join path
  const [error, setError] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal(false); // "Copy code" feedback flip
  // Only rendered when the Clipboard API is unavailable/denied — a readonly input the guest can
  // select-and-Ctrl+C by hand, in place of the one-click copy.
  const [clipboardFallback, setClipboardFallback] = createSignal(false);
  let shareInputRef: HTMLInputElement | undefined;
  createEffect(() => {
    if (clipboardFallback()) shareInputRef?.select();
  });

  const you = () => lobby()?.you ?? null;
  const joined = () => you() != null;

  // Fold a lobby view into local state: adopt it, mirror the claimed seat into the URL, hand off
  // to the board when the game starts. Actions additionally surface the view's error line.
  const reflect = (view: LobbyView) => {
    setLobby(view);
    if (view.you != null) setTableUrl(view.table_id);
    if (view.started) props.onStarted();
  };
  const apply = (view: LobbyView) => {
    reflect(view);
    setError(view.error ?? null);
  };
  // Poll ticks fold through here: adopt the view, and surface a died-table poll (e.g. UnknownTable)
  // rather than silently degrading to an empty "claim a seat" screen. A healthy tick carries no
  // error, so this never stomps an action error with a stale clear.
  const reflectPoll = (view: LobbyView) => {
    reflect(view);
    if (view.error != null) setError(view.error);
  };

  // Drive `reflectPoll` off the poll stream while a table is set. Re-keys on `table()` (a null table
  // parks on a no-op stream); the subscription is torn down on cleanup — including when `onStarted`
  // flips the parent `Show` to the board.
  const registry = useContext(RegistryContext);
  createEffect(() => {
    onCleanup(startLobbyPoll(registry, lobbyPollFamily(table()), reflectPoll));
  });

  const createTable = useAtomSet(() => createTableFn, { mode: "promise" });
  const joinTable = useAtomSet(() => joinTableFn, { mode: "promise" });
  const readyUp = useAtomSet(() => readyUpFn, { mode: "promise" });
  const startGame = useAtomSet(() => startGameFn, { mode: "promise" });
  const copyText = useAtomSet(() => copyTextFn, { mode: "promise" });

  // `null` means the request never landed (folded in the atom, above); anything else is a 200
  // whose `error` field carries the logical outcome. One path, no rejected promise to catch.
  const applyOrFail = (view: LobbyView | null): void => {
    if (!view) {
      setError(UNREACHABLE);
      return;
    }
    apply(view);
  };

  // Host and Join both claim a seat with the already-picked deck: Host mints a fresh table, Join
  // uses the code typed by the guest. Share-link arrivals (table already in the URL) reuse this too.
  const joinWith = async (t: string) => {
    const d = deckId() ?? decks()?.[0]?.id ?? null;
    if (d == null) {
      setError("Pick a deck to bring first.");
      return;
    }
    setTable(t);
    applyOrFail(await joinTable({ table_id: t, deck_id: d }));
  };
  const onHost = async () => {
    const created = await createTable();
    if (!created) {
      setError(UNREACHABLE);
      return;
    }
    await joinWith(created.table_id);
  };
  const onJoinCode = async () => {
    const t = parseTableCode(code()); // bare code or legacy share link
    if (t == null) {
      setError("Enter the table code your host shared.");
      return;
    }
    await joinWith(t);
  };
  const onJoin = () => {
    const t = table();
    if (t) joinWith(t);
  };
  const onReady = async (ready: boolean) => {
    const t = table();
    if (!t) return;
    applyOrFail(await readyUp({ table_id: t, ready }));
  };
  const onStart = async () => {
    const t = table();
    if (!t) return;
    applyOrFail(await startGame({ table_id: t }));
  };

  const onCopyCode = async () => {
    const code = table();
    if (!code) return;
    if (!(await copyText(code))) {
      setClipboardFallback(true);
      return;
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1500); // a view-layer flip, not async work (ADR 0019)
  };

  const myReady = () => {
    const y = you();
    return y != null && (lobby()?.seats[y]?.ready ?? false);
  };
  const isHost = () => lobbyIsHost(you(), lobby()?.seats);

  /** Why Start is blocked, or null when it isn't. The server's own gate, verbatim. */
  const startError = () => lobby()?.start_error ?? null;

  // Deck picker shared by the entry screen and the share-link claim path.
  const DeckPicker = () => (
    <>
      <label for="lobby-deck" class="sr-only">
        Choose deck
      </label>
      <select
        id="lobby-deck"
        value={deckId() ?? decks()?.[0]?.id ?? ""}
        onInput={(e) => setDeckId(Number(e.currentTarget.value))}
        class="rounded-control border border-vine bg-glass px-md py-sm text-body text-snow"
      >
        <For each={decks()}>{(d) => <option value={d.id}>{d.name}</option>}</For>
      </select>
    </>
  );

  // The deck carried from the Decks page (?deck=ID) — the one they hit Play on.
  const pickedDeck = () => decks()?.find((d) => d.id === deckId()) ?? null;

  // Landing: bring the picked deck, then Host (mint a code) or Join (enter a host's code). Gate on
  // *having* a deck id, not on resolving its name — decks() loads async and shouldn't block hosting.
  const Entry = () => (
    <Show
      when={deckId() != null}
      fallback={<div class="text-caution-amber text-label">Pick a deck to play first (Your decks → Play).</div>}
    >
      <div class="flex items-center gap-sm">
        <span class="text-label text-lichen">
          Bring: <b>{pickedDeck()?.name ?? "your deck"}</b>
        </span>
      </div>
      <div class="flex items-center gap-sm">
        <Button type="button" onClick={onHost}>
          Host a table
        </Button>
      </div>
      <div class="flex flex-wrap items-center gap-sm">
        <label for="table-code" class="sr-only">
          Table code
        </label>
        <Field
          id="table-code"
          placeholder="Table code"
          value={code()}
          onInput={(e) => setCode(e.currentTarget.value)}
          class="min-w-0 flex-1"
          autocomplete="off"
          spellcheck={false}
        />
        <Button type="button" onClick={onJoinCode}>
          Join
        </Button>
      </div>
    </Show>
  );

  return (
    <Felt class="fixed inset-0 overflow-y-auto">
      <div class="flex min-h-full items-center justify-center p-xxl">
        <Panel as="main">
          <h1 class="m-0 text-title">mtgfr — Lobby</h1>

          <Show when={table()} fallback={<Entry />}>
            <div class="flex flex-wrap items-center gap-md">
              <span class="text-label text-lichen">Table code</span>
              <span class="select-text font-bold text-display tracking-[0.06em]">{table()}</span>
              <Button type="button" onClick={onCopyCode}>
                {copied() ? "Copied" : "Copy code"}
              </Button>
            </div>
            <Show when={clipboardFallback()}>
              <label for="share-code" class="sr-only">
                Table code (manual copy)
              </label>
              <Field
                id="share-code"
                readOnly
                value={table() ?? ""}
                ref={shareInputRef}
                onFocus={(e) => e.currentTarget.select()}
                class="w-[120px] px-1.5 py-1 text-chip tracking-[0.06em]"
              />
            </Show>

            <div class="flex flex-col gap-xs">
              <For each={lobby()?.seats ?? []}>
                {(s) => (
                  <div class={SEAT_ROW}>
                    {/* Open-seat ink: dimmer than the claimed-row text but still ≥4.5:1 against the
                        row background (glass-dim over Forest Floor ≈ #171f1c) — measured ~9.4:1. */}
                    <span class={cn(!s.claimed && "text-lichen")}>
                      {s.claimed ? (s.username ?? `Seat ${s.player + 1}`) : `Seat ${s.player + 1}`}
                    </span>
                    <span class={cn("min-w-0 truncate text-lichen", s.claimed && "text-mist")}>
                      {s.claimed ? (s.deck_name ?? "—") : "open"}
                    </span>
                    <span class="flex items-center justify-end gap-xs">
                      <Show when={s.is_host}>
                        <span class="text-label text-lichen">Host</span>
                      </Show>
                      <Show when={s.claimed}>
                        <Show when={s.ready} fallback={<span class="text-label text-lichen">Waiting…</span>}>
                          {/* Llanowar-tinted chip on the seat row (~#1d3727) — measured ~8.4:1. */}
                          <span class="inline-block rounded-full bg-llanowar/25 px-sm py-0.5 font-semibold text-caption text-ready-sprout">
                            Ready
                          </span>
                        </Show>
                      </Show>
                      <Show when={s.is_you}>
                        <span class="text-label text-lichen">(you)</span>
                      </Show>
                    </span>
                  </div>
                )}
              </For>
            </div>

            <Show
              when={joined()}
              fallback={
                <Show
                  when={(decks() ?? []).length > 0}
                  fallback={
                    <div class="text-caution-amber text-label">Build a deck first (Your decks → New deck).</div>
                  }
                >
                  <div class="flex flex-wrap items-center gap-sm">
                    <DeckPicker />
                    <Button type="button" onClick={onJoin}>
                      Claim a seat
                    </Button>
                  </div>
                </Show>
              }
            >
              <div class="flex flex-wrap items-center gap-sm">
                <Button type="button" onClick={() => onReady(!myReady())}>
                  {myReady() ? "Unready" : "Ready up"}
                </Button>
                <Show when={isHost()}>
                  {/* A disabled button can't be clicked, so it can never surface its own rejection —
                      the reason has to be shown next to it or the host is left guessing. */}
                  <Button type="button" disabled={startError() !== null} onClick={onStart}>
                    Start game
                  </Button>
                  <Show when={startError()}>
                    {(e) => <span class="text-caption text-lichen">{humanError(e())}</span>}
                  </Show>
                </Show>
              </div>
            </Show>
          </Show>

          <Show when={error()}>
            {(e) => (
              <div role="alert" class="text-burn-red text-caption">
                {humanError(e())}
              </div>
            )}
          </Show>
        </Panel>
      </div>
    </Felt>
  );
}

function humanError(code: string): string {
  const map: Record<string, string> = {
    TableFull: "That table is full.",
    AlreadyStarted: "The game already started.",
    NotHost: "Only the host can start.",
    NeedTwoPlayers: "Need at least two players.",
    NotAllReady: "Everyone must ready up first.",
    UnknownTable: "No such table.",
    NotSeated: "Claim a seat first.",
    UnknownDeck: "That deck no longer exists.",
    [UNREACHABLE]: "Couldn't reach the table — try again.",
  };
  return map[code] ?? code;
}
