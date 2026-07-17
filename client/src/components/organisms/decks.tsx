// The deck manager: list your saved decks, build a new one, edit or delete, or take one to a
// table to play. Auth-gated (redirects to /login if not signed in).
//
// Deck list is the shared `decksAtom` (per ADR 0019); delete/logout are function atoms consumed
// via `useAtomSet` in promise mode, so error folding lives in the Effect pipeline, not here.

import { useAtomRefresh, useAtomSet, useAtomValue } from "@effect/atom-solid";
import { useNavigate } from "@solidjs/router";
import * as Option from "effect/Option";
import * as AsyncResult from "effect/unstable/reactivity/AsyncResult";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createEffect, createSignal, For, Show } from "solid-js";
import { decksAtom } from "~/atoms";
import { Button, Felt, ListRow } from "~/components/atoms";
import CardPreview from "~/components/molecules/card-preview";
import ConfirmDialog from "~/components/molecules/confirm-dialog";
import { client, succeeded } from "~/effect/client";
import { RequireAuth } from "~/guard";
import { lookupCardsByIds } from "~/lib/lookupCards";
import type { CatalogCard, Me } from "~/wire/types";

const deleteDeckFn = Atom.fn((id: number) => succeeded(client.deleteDeck(String(id))));
const logoutFn = Atom.fn(() => succeeded(client.logout()));
// Commander catalog lookup by Card id; hover art uses `commander_print` on the summary.
const lookupCommandersFn = Atom.fn((ids: string[]) => lookupCardsByIds(ids));

export default function Decks() {
  return <RequireAuth>{(user) => <DecksSignedIn user={user()} />}</RequireAuth>;
}

function DecksSignedIn(props: { user: Me }) {
  const navigate = useNavigate();
  // Non-suspending — `useAtomResource` blanks the whole page under the app-root Suspense.
  const decksResult = useAtomValue(() => decksAtom);
  const decks = () => Option.getOrUndefined(AsyncResult.value(decksResult()));
  const decksLoading = () => {
    const r = decksResult();
    return AsyncResult.isInitial(r) || r.waiting;
  };
  // Refreshing the atom is what actually re-fetches (as in guard).
  const refreshDecks = useAtomRefresh(() => decksAtom);
  const deleteDeck = useAtomSet(() => deleteDeckFn, { mode: "promise" });
  const logout = useAtomSet(() => logoutFn, { mode: "promise" });
  const lookupCommanders = useAtomSet(() => lookupCommandersFn, { mode: "promise" });

  // The deck whose delete confirmation is up, or null. One at a time — the dialog is modal.
  const [confirmingId, setConfirmingId] = createSignal<number | null>(null);
  const confirming = () => decks()?.find((d) => d.id === confirmingId());

  // Commander catalog data by Card id, hydrated once the deck list resolves — the list shows each
  // commander's id until its name arrives.
  const [commanders, setCommanders] = createSignal<Record<string, CatalogCard>>({});
  createEffect(() => {
    const ids = [...new Set((decks() ?? []).map((d) => d.commander).filter(Boolean))];
    if (ids.length === 0) return;
    void lookupCommanders(ids).then((cards) => {
      setCommanders(Object.fromEntries(cards.map((c) => [c.id, c])));
    });
  });
  const commanderName = (id: string) => commanders()[id]?.name ?? id;

  // The commander currently hovered, for the shared read-the-card preview.
  const [hover, setHover] = createSignal<{ id: string; print?: string; x: number; y: number } | null>(null);

  // The last request that didn't land. Cleared on the next attempt.
  const [failed, setFailed] = createSignal<string | null>(null);

  const onDelete = async (id: number) => {
    setFailed(null);
    setConfirmingId(null); // the dialog has answered its question; a failure surfaces on the page
    if (!(await deleteDeck(id))) return setFailed("Couldn't delete that deck — try again.");
    refreshDecks();
  };
  const onLogout = async () => {
    setFailed(null);
    // The session cookie is the server's to clear; if the request didn't land we're still signed in,
    // so stay put rather than navigating to /login and bouncing straight back through the guard.
    if (!(await logout())) return setFailed("Couldn't sign out — try again.");
    navigate("/login", { replace: true });
  };

  return (
    // `#app` is fixed to the viewport with `body { overflow: hidden }` (for the board), so page
    // scroll is off — this screen must scroll itself, hence `h-full` + `overflow-y-auto`.
    // Landscape-first: keep the horizontal list layout; portrait phones hit the rotate gate.
    <Felt
      as="main"
      data-testid="decks-page"
      class="h-full overflow-y-auto p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]"
    >
      <div class="mx-auto mb-5 flex max-w-[720px] flex-wrap items-center justify-between gap-md">
        <h1 class="m-0 text-title">Your decks</h1>
        <div class="flex flex-wrap items-center gap-md">
          <span class="text-label text-lichen">{props.user.email}</span>
          <Button type="button" onClick={onLogout} variant="ghost">
            Sign out
          </Button>
          <Button type="button" onClick={() => navigate("/decks/new")}>
            New deck
          </Button>
        </div>
      </div>

      <div class="mx-auto flex max-w-[720px] flex-col gap-md">
        <Show when={failed()}>
          <div role="alert" class="text-label text-reconnect-rust">
            {failed()}
          </div>
        </Show>
        <Show when={decksLoading()}>
          <div class="text-label text-lichen">Loading decks…</div>
        </Show>
        <Show when={!decksLoading()}>
          <Show
            when={(decks() ?? []).length > 0}
            fallback={<div class="text-label text-lichen">No decks yet — build one to get started.</div>}
          >
            <For each={decks()}>
              {(d) => (
                <ListRow class="flex flex-wrap items-center justify-between gap-md rounded-hud px-xl py-3">
                  <div class="flex min-w-0 flex-col">
                    <span class="font-semibold">
                      {d.name}
                      {/* Precons (negative id) are read-only — everyone has them, nobody edits them.
                        Commander Gold is reserved for commanders (DESIGN.md §2/§6), so the precon
                        chip uses the Lichen (muted-label) family instead, same shape and size. */}
                      <Show when={d.id < 0}>
                        <span class="ml-sm rounded-full bg-lichen/14 px-[7px] py-px align-middle font-semibold text-chip text-lichen">
                          Precon
                        </span>
                      </Show>
                    </span>
                    {/* biome-ignore lint/a11y/noStaticElementInteractions: hover only reveals the
                      commander's art; its name is right here as text. */}
                    <span
                      onMouseMove={(e) =>
                        setHover({
                          id: d.commander,
                          print: d.commander_print || commanders()[d.commander]?.default_print,
                          x: e.clientX,
                          y: e.clientY,
                        })
                      }
                      onMouseLeave={() => setHover(null)}
                      class="text-label text-lichen"
                    >
                      {commanderName(d.commander)}
                    </span>
                  </div>
                  <div class="flex flex-wrap gap-sm">
                    <Button
                      type="button"
                      data-testid={`deck-play-${d.id}`}
                      onClick={() => navigate(`/play?deck=${d.id}`)}
                    >
                      Play
                    </Button>
                    <Show when={d.id >= 0}>
                      <Button type="button" onClick={() => navigate(`/decks/${d.id}`)} variant="ghost">
                        Edit
                      </Button>
                      <Button type="button" onClick={() => setConfirmingId(d.id)} variant="ghost">
                        Delete
                      </Button>
                    </Show>
                  </div>
                </ListRow>
              )}
            </For>
          </Show>
        </Show>
      </div>

      <ConfirmDialog
        open={confirmingId() !== null}
        title={`Delete “${confirming()?.name ?? ""}”?`}
        body="This deck and its card list are gone for good."
        confirmLabel="Delete deck"
        danger
        onConfirm={() => {
          const id = confirmingId();
          if (id !== null) onDelete(id);
        }}
        onCancel={() => setConfirmingId(null)}
      />

      <CardPreview id={hover()?.id ?? null} print={hover()?.print} x={hover()?.x ?? 0} y={hover()?.y ?? 0} />
    </Felt>
  );
}
