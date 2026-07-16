// MTGA-style deck builder over the curated pool. Left: a browsable grid of every pool card
// (art + cost + type), click to add. Right: the commander picker, deck name, and the live
// decklist with per-card counts and a running total toward 99. Full Commander legality is
// enforced by the server on save; we surface its problems and mirror the obvious ones live.

import { useAtom, useAtomResource, useAtomSet, useAtomValue } from "@effect/atom-solid";
import { useNavigate, useParams } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import { createEffect, createMemo, createSignal, For, on, onCleanup, onMount, Show, untrack } from "solid-js";
import { createStore, produce } from "solid-js/store";
import type { CatalogCard, DeckError, SaveDeckRequest } from "~/api/generated";
import CardPreview from "~/CardPreview";
import ConfirmDialog from "~/ConfirmDialog";
import { client } from "~/effect/client";
import { useAuthGuard } from "~/guard";
import { cn } from "~/lib/cn";
import { imageUrlByName } from "~/lib/scryfall";
import { Button, Felt, Field } from "~/ui";

const BASICS = new Set(["Plains", "Island", "Swamp", "Mountain", "Forest"]);
const DECK_SIZE = 99;

/** A card that can be this deck's commander: a legendary creature (the only commanders in pool). */
const canBeCommander = (c: CatalogCard) => c.legendary && c.kind.kind === "creature";

type MenuItem = { label: string; run: () => void };

// One server page. The server caps `limit` at 200 (MAX_LIMIT); we page in PAGE-sized chunks and
// accumulate them in the grid as the user scrolls (see the sentinel/IntersectionObserver below).
const PAGE = 100;

// A pool tile and a decklist row are both list rows; only their density differs.
const LIST_ROW = "border border-vine-dim bg-glass-dim text-snow hover:bg-white/8";
const POOL_CARD = cn(LIST_ROW, "flex cursor-pointer flex-col items-center gap-1 rounded-hud p-sm text-caption");
const DECK_ROW = cn(LIST_ROW, "flex w-full cursor-pointer items-center gap-xs rounded-[5px] px-sm py-1 text-left");
const CARD_ART = cn("aspect-[0.72] w-full rounded-[5px] object-cover");

// Search: the box writes `queryAtom`; a 200ms debounce feeds the derived query atom so the pool
// grid fetches only after typing settles. An empty query is a real value here (fetches the first
// page of the pool) — atoms have no createResource-style falsy-source quirk to work around.
const queryAtom = Atom.make("");
const debouncedQueryAtom = Atom.debounce(queryAtom, "200 millis");
// How far into the current query's results we've paged; the scroll sentinel bumps it by PAGE.
const offsetAtom = Atom.make(0);
// Fetch one page and tag it with the (query, offset) it was fetched for, so the accumulator can
// reject a page that resolves for a stale query or a superseded offset (see the reset/fold below).
const searchResultsAtom = Atom.make((get) => {
  const q = get(debouncedQueryAtom);
  const offset = get(offsetAtom);
  return client.searchCards({ params: { q, limit: PAGE, offset } }).pipe(Effect.map((cards) => ({ q, offset, cards })));
});

// Existing-deck load, one atom per id (memoized). `/decks/new` passes a null id and resolves to
// null, so the prefill effect no-ops — matching createResource's "don't fetch on a null source".
const deckAtomFamily = Atom.family((id: number | null) =>
  Atom.make(id === null ? Effect.succeed(null) : client.getDeck(String(id), {}).pipe(Effect.map((d) => d ?? null))),
);

// Hydrate a loaded deck's cards' data (the commander/decklist need color identity). Invoked once
// per loaded deck from the prefill effect; the caller folds the result into `known` via `remember`.
const hydrateCardsFn = Atom.fn((names: string[]) => client.lookupCards({ params: { names } }));

// Save a deck. Every branch resolves to a problem list or `null` ("saved") — a 422 arrives as a
// generated `MtgfrError` tag (`{Create,Update}Deck422`) carrying the `DeckError` — so the promise
// from `useAtomSet(..., { mode: "promise" })` never rejects.
const saveDeckFn = Atom.fn((req: { id: number | null; body: SaveDeckRequest }) => {
  // Widen the create/update union (their 422 error tags differ) to one Effect type for `.pipe`.
  const attempt: Effect.Effect<unknown, unknown> =
    req.id !== null
      ? client.updateDeck(String(req.id), { payload: req.body })
      : client.createDeck({ payload: req.body });
  return attempt.pipe(
    Effect.as(null as string[] | null),
    Effect.catch((err) => {
      const tag = (err as { _tag?: string })._tag;
      if (tag === "CreateDeck422" || tag === "UpdateDeck422") {
        return Effect.succeed([...(err as { cause: DeckError }).cause.problems]);
      }
      return Effect.succeed(["Could not save the deck."]);
    }),
  );
});

export default function DeckBuilder() {
  useAuthGuard();
  const params = useParams();
  const navigate = useNavigate();
  const editingId = () => (params.id ? Number(params.id) : null);

  const [existing] = useAtomResource(() => deckAtomFamily(editingId()));
  const hydrateCards = useAtomSet(() => hydrateCardsFn, { mode: "promise" });
  const saveDeck = useAtomSet(() => saveDeckFn, { mode: "promise" });

  // The single search box drives a debounced server query (via `queryAtom`); the pool grid renders
  // its results rather than the whole pool (which stays server-side and scales past a few hundred
  // cards). `queryAtom` is module-level, so reset it on mount — otherwise the builder would reopen
  // with the last query still typed instead of the first page.
  const [query, setQuery] = useAtom(() => queryAtom);
  const debouncedQuery = useAtomValue(() => debouncedQueryAtom);
  const [offset, setOffset] = useAtom(() => offsetAtom);
  onMount(() => setQuery(""));
  const [results] = useAtomResource(() => searchResultsAtom);

  // The grid renders an accumulated list of every page fetched so far — infinite scroll appends,
  // it isn't the single current page. `atEnd` latches once a short page proves we've hit the pool's
  // end, so the scroll sentinel stops asking for more.
  // ponytail: plain DOM scroll + append, no virtualization — the pool is a few hundred cards; add
  // windowing only if it grows past a few thousand.
  const [pool, setPool] = createSignal<CatalogCard[]>([]);
  const [atEnd, setAtEnd] = createSignal(false);

  // A new query starts the grid over: clear it (so skeletons show, not stale hits) and page from 0.
  createEffect(
    on(debouncedQuery, () => {
      setPool([]);
      setAtEnd(false);
      setOffset(0);
    }),
  );

  // Fold each arriving page into the grid. Accept it only if it's for the current query AND the
  // offset it was fetched for is still the one we want — this drops pages that resolve late for a
  // superseded query/offset (the reset above bumps both). Dedup by name is belt-and-suspenders.
  createEffect(() => {
    const page = results();
    if (!page || page.q !== debouncedQuery() || page.offset !== offset()) return;
    untrack(() => {
      const seen = new Set(pool().map((c) => c.name));
      setPool([...pool(), ...page.cards.filter((c) => !seen.has(c.name))]);
      if (page.cards.length < PAGE) setAtEnd(true);
    });
  });

  // Infinite scroll: when the bottom sentinel scrolls into the pool's own scroll area, fetch the
  // next page — unless we're already loading or have hit the end (never loop on empty pages).
  let gridEl: HTMLDivElement | undefined;
  let sentinel: HTMLDivElement | undefined;
  onMount(() => {
    if (!sentinel) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries[0].isIntersecting || atEnd() || results.loading) return;
        setOffset((o) => o + PAGE);
      },
      { root: gridEl, rootMargin: "300px" },
    );
    io.observe(sentinel);
    onCleanup(() => io.disconnect());
  });

  const [name, setName] = createStore({ value: "New deck" });
  const [commander, setCommander] = createStore({ value: "" });
  // name → count.
  const [entries, setEntries] = createStore<Record<string, number>>({});
  const [problems, setProblems] = createStore<{ list: string[] }>({ list: [] });
  // True once the user has changed name/commander/list since the deck loaded. Drives the
  // Cancel button: a clean builder navigates away immediately, a dirty one confirms first.
  const [dirty, setDirty] = createSignal(false);
  const [confirmDiscard, setConfirmDiscard] = createSignal(false);
  // Card data we've seen, keyed by name — accumulated from search results and hydrated for a
  // loaded deck. Lets the commander/decklist resolve color identity without loading the full pool.
  const [known, setKnown] = createStore<Record<string, CatalogCard>>({});
  const remember = (cards: readonly CatalogCard[] | undefined) =>
    cards &&
    setKnown(
      produce((k) => {
        for (const c of cards) k[c.name] = c;
      }),
    );
  // The pool card currently hovered, for the read-the-text preview.
  const [hover, setHover] = createSignal<{ name: string; x: number; y: number } | null>(null);
  // The right-click context menu: a title, its items, and where to show it.
  const [menu, setMenu] = createSignal<{ title: string; items: MenuItem[]; x: number; y: number } | null>(null);

  createEffect(() => remember(results()?.cards));

  // Prefill when editing an existing deck, and hydrate its cards' data for identity/preview.
  createEffect(() => {
    const deck = existing();
    if (!deck) return;
    setName("value", deck.name);
    setCommander("value", deck.commander);
    setEntries(reconcileEntries(deck.cards));
    const names = deck.cards.map((c) => c.name);
    if (deck.commander) names.push(deck.commander);
    void hydrateCards(names).then(remember);
  });

  const commanderIdentity = () => known[commander.value]?.color_identity ?? [];
  const offIdentity = (c: CatalogCard) => c.color_identity.some((i) => !commanderIdentity().includes(i));

  const total = createMemo(() => Object.values(entries).reduce((sum, n) => sum + n, 0));

  const add = (n: string) => addN(n, 1);
  /** Add `n` copies. Non-basics are capped at one regardless of `n`. */
  const addN = (n: string, count: number) => {
    setDirty(true);
    setEntries(n, (c) => (BASICS.has(n) ? (c ?? 0) + count : 1));
  };
  const setCount = (n: string, count: number) => {
    setDirty(true);
    if (count <= 0) {
      setEntries(produce((e) => delete e[n]));
      return;
    }
    setEntries(n, BASICS.has(n) ? count : 1);
  };
  /** Remove `count` copies (deletes the entry at zero). */
  const removeN = (n: string, count: number) => setCount(n, (entries[n] ?? 0) - count);
  const setCommanderDirty = (v: string) => {
    setDirty(true);
    setCommander("value", v);
  };
  const removeItems = (n: string): MenuItem[] => [
    { label: "Fill deck", run: () => addN(n, Math.max(0, DECK_SIZE - total())) },
    ...[1, 2, 5].map((k) => ({ label: `Remove ${k}`, run: () => removeN(n, k) })),
  ];

  /** Right-click menu items for a pool card: basics add in bulk (and can fill the deck out to
   * the card limit), commanders can be set as the commander, everything else just adds one. */
  const menuItems = (c: CatalogCard): MenuItem[] => {
    if (BASICS.has(c.name))
      return [
        { label: "Add One", run: () => addN(c.name, 1) },
        { label: "Add Two", run: () => addN(c.name, 2) },
        { label: "Add Five", run: () => addN(c.name, 5) },
        { label: "Fill deck", run: () => addN(c.name, Math.max(0, DECK_SIZE - total())) },
      ];
    if (canBeCommander(c))
      return [
        { label: "Add One", run: () => add(c.name) },
        { label: "Set As Commander", run: () => setCommanderDirty(c.name) },
      ];
    return [{ label: "Add One", run: () => add(c.name) }];
  };

  const deckList = createMemo(() =>
    Object.entries(entries)
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => a.name.localeCompare(b.name)),
  );

  // A save is in flight. On the create path a second click would POST a second `createDeck` before
  // the first one's `navigate` runs, leaving the user with two identical decks.
  const [saving, setSaving] = createSignal(false);

  const save = async () => {
    if (saving()) return;
    setSaving(true);
    setProblems("list", []);
    const body: SaveDeckRequest = {
      name: name.value,
      commander: commander.value,
      cards: Object.entries(entries).map(([name, count]) => ({ name, count })),
    };
    // `saveDeckFn` folds every outcome to a problem list or `null` ("saved"), so this never rejects.
    const problems = await saveDeck({ id: editingId(), body });
    setSaving(false);
    if (problems === null) {
      navigate("/", { replace: true });
      return;
    }
    setProblems("list", problems);
  };

  return (
    // Landscape-first: pool | deck stay side-by-side. Portrait phones hit the rotate gate — we do
    // not stack into a vertical builder. Short landscape phones shrink the deck column, not the axis.
    <Felt
      as="main"
      class="grid h-full grid-cols-[minmax(0,1fr)_minmax(220px,min(32vw,360px))] gap-5 overflow-hidden p-xxl pt-[max(1.5rem,env(safe-area-inset-top))] pr-[max(1.5rem,env(safe-area-inset-right))] pb-[max(1.5rem,env(safe-area-inset-bottom))] pl-[max(1.5rem,env(safe-area-inset-left))]"
    >
      {/* Pool grid — only this column scrolls; its scrollbar sits beside the cards. */}
      <div class="flex min-h-0 min-w-0 flex-col">
        <h1 class="m-0 text-title">Card pool</h1>
        <div class="text-label text-lichen">Click a card to add it. Only basics may exceed one copy.</div>
        <label for="pool-search" class="sr-only">
          Search card pool
        </label>
        <Field
          id="pool-search"
          type="search"
          placeholder="Search name, type, subtype, color, set, tag…"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          class="mt-sm"
        />
        {/* `content-start` packs rows at their natural height from the top — without it the scroll
            area's height stretches a lone row of cards into tall boxes. */}
        <div
          ref={gridEl}
          class="mt-3 grid min-h-0 flex-1 grid-cols-[repeat(auto-fill,minmax(120px,1fr))] content-start gap-md overflow-y-auto"
        >
          <For each={pool()}>
            {(c) => (
              <button
                type="button"
                onClick={() => add(c.name)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setHover(null);
                  setMenu({ title: c.name, items: menuItems(c), x: e.clientX, y: e.clientY });
                }}
                onMouseMove={(e) => setHover({ name: c.name, x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
                class={cn(POOL_CARD, commander.value && offIdentity(c) && "opacity-40")}
              >
                <img src={imageUrlByName(c.name, "small")} alt={c.name} loading="lazy" class={CARD_ART} />
                <span class="text-center leading-[1.1]">
                  {c.legendary ? "★ " : ""}
                  {c.name}
                </span>
              </button>
            )}
          </For>
          {/* Skeletons while a page loads — same footprint as a pool card, so the grid doesn't jump
              when the real cards land. */}
          <Show when={results.loading}>
            <For each={Array.from({ length: 10 })}>
              {() => (
                <div class={cn(POOL_CARD, "pointer-events-none cursor-default")}>
                  <div class={cn(CARD_ART, "animate-skeleton bg-white/8")} />
                  <div class="h-2.5 w-[70%] animate-skeleton rounded-[3px] bg-white/8" />
                </div>
              )}
            </For>
          </Show>
          <Show when={!results.loading && pool().length === 0}>
            <div class="col-span-full text-label text-lichen">No cards match.</div>
          </Show>
          {/* Bottom sentinel: the IntersectionObserver above fetches the next page when it appears. */}
          <div ref={sentinel} class="col-span-full h-px" />
        </div>
      </div>

      {/* Deck panel */}
      <div class="flex min-w-0 flex-col gap-3">
        <h2 class="m-0 text-title">{editingId() ? "Edit deck" : "New deck"}</h2>
        <label for="deck-name" class="sr-only">
          Deck name
        </label>
        <Field
          id="deck-name"
          value={name.value}
          onInput={(e) => {
            setDirty(true);
            setName("value", e.currentTarget.value);
          }}
        />

        <div class="text-label text-lichen">Commander</div>
        <Show
          when={commander.value}
          fallback={
            <div class="text-label text-lichen">Right-click a legendary creature and choose “Set As Commander”.</div>
          }
        >
          <button
            type="button"
            title="Click to remove"
            onClick={() => setCommanderDirty("")}
            onMouseMove={(e) => setHover({ name: commander.value, x: e.clientX, y: e.clientY })}
            onMouseLeave={() => setHover(null)}
            class="flex w-full cursor-pointer items-center gap-sm rounded-control border border-vine bg-glass-dim px-sm py-xs text-left"
          >
            <img
              src={imageUrlByName(commander.value, "small")}
              alt={commander.value}
              class="aspect-[0.72] w-10 rounded-focus object-cover"
            />
            <span class="min-w-0 flex-1 truncate font-semibold">★ {commander.value}</span>
          </button>
        </Show>

        <div class="flex items-center justify-between gap-sm">
          <b>Cards</b>
          <span class={cn("shrink-0 text-caution-amber", total() === DECK_SIZE && "text-vine")}>
            {total()}/{DECK_SIZE}
            {commander.value ? " + commander" : ""}
          </span>
        </div>

        <div class="flex max-h-[40vh] min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          <For each={deckList()}>
            {(row) => (
              <button
                type="button"
                title="Click to remove one"
                onClick={() => {
                  removeN(row.name, 1);
                  setHover(null);
                }}
                onMouseMove={(e) => setHover({ name: row.name, x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setHover(null);
                  if (!BASICS.has(row.name)) return;
                  setMenu({ title: row.name, items: removeItems(row.name), x: e.clientX, y: e.clientY });
                }}
                class={DECK_ROW}
              >
                <span class="min-w-0 flex-1 truncate">
                  {known[row.name]?.legendary ? "★ " : ""}
                  {row.name}
                  <Show when={row.name === commander.value}>
                    <span class="text-label text-lichen"> (commander)</span>
                  </Show>
                </span>
                <span class="shrink-0 text-label text-lichen">×{row.count}</span>
              </button>
            )}
          </For>
        </div>

        <Button type="button" onClick={save} disabled={saving()}>
          {saving() ? "Saving…" : "Save deck"}
        </Button>
        <Button type="button" onClick={() => (dirty() ? setConfirmDiscard(true) : navigate("/"))} variant="ghost">
          Cancel
        </Button>
        <ConfirmDialog
          open={confirmDiscard()}
          title="Discard changes?"
          body="Everything you've edited since the deck loaded will be lost."
          confirmLabel="Discard"
          danger
          onConfirm={() => navigate("/")}
          onCancel={() => setConfirmDiscard(false)}
        />

        <Show when={problems.list.length > 0}>
          <div role="alert" class="flex flex-col gap-[3px]">
            <For each={problems.list}>{(p) => <div class="text-burn-red text-caption">{p}</div>}</For>
          </div>
        </Show>
      </div>

      <CardPreview name={hover()?.name ?? null} x={hover()?.x ?? 0} y={hover()?.y ?? 0} />

      <Show when={menu()}>
        {(m) => (
          <>
            {/* Full-screen catcher: any click (or another right-click) dismisses the menu. */}
            {/* biome-ignore lint/a11y/noStaticElementInteractions: the menu only opens on
                right-click, so its dismissal is mouse-only by construction. */}
            {/* biome-ignore lint/a11y/useKeyWithClickEvents: same */}
            <div
              onClick={() => setMenu(null)}
              onContextMenu={(e) => {
                e.preventDefault();
                setMenu(null);
              }}
              class="fixed inset-0 z-[2500]"
            />
            <div
              style={{
                "--x": `${Math.min(m().x, window.innerWidth - 180)}px`,
                "--y": `${Math.min(m().y, window.innerHeight - 140)}px`,
              }}
              class="fixed top-(--y) left-(--x) z-[2501] flex min-w-[160px] flex-col rounded-hud border border-vine bg-forest-surface p-xs shadow-table"
            >
              <div class="border-[#223344] border-b px-md pt-0.5 pb-1.5 text-label text-lichen">{m().title}</div>
              <For each={m().items}>
                {(item) => (
                  <button
                    type="button"
                    onClick={() => {
                      item.run();
                      setMenu(null);
                    }}
                    class="cursor-pointer rounded-[5px] border-none bg-transparent px-md py-xs text-left text-label text-snow"
                  >
                    {item.label}
                  </button>
                )}
              </For>
            </div>
          </>
        )}
      </Show>
    </Felt>
  );
}

/** Turn a loaded decklist into the store's name→count record. */
function reconcileEntries(cards: { name: string; count: number }[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const c of cards) out[c.name] = c.count;
  return out;
}
