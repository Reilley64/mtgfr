// MTGA-style deck builder over the curated pool. Left: a browsable grid of every pool card
// (art + cost + type), click to add. Right: the commander picker, deck name, and the live
// decklist with per-card counts and a running total toward 99. Full Commander legality is
// enforced by the server on save; we surface its problems and mirror the obvious ones live.
//
// Card identity is the Scryfall oracle id (`CardDef.id` / `CatalogCard.id`); a Printing (Scryfall
// card UUID) is art preference only (ADR 0031). `preferredPrint` is a sticky, session-local choice
// per Card id — once you pick a printing for a card, adding it again reuses that choice.

import { useAtom, useAtomResource, useAtomSet, useAtomValue } from "@effect/atom-solid";
import { useNavigate, useParams } from "@solidjs/router";
import * as Effect from "effect/Effect";
import * as Atom from "effect/unstable/reactivity/Atom";
import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  on,
  onCleanup,
  onMount,
  Show,
  untrack,
} from "solid-js";
import { createStore, produce } from "solid-js/store";
import type { CatalogCard, DeckCardEntry, DeckError, SaveDeckRequest } from "~/api/generated";
import CardPreview from "~/components/molecules/card-preview";
import ConfirmDialog from "~/components/molecules/confirm-dialog";
import { client } from "~/effect/client";
import { useAuthGuard } from "~/guard";
import { cn } from "~/lib/cn";
import { commanderPrintForRow, formatReleasedAt, reconcileEntries } from "~/lib/deckBuilderPrint";
import { lookupCardsByIds } from "~/lib/lookupCards";
import { openModalWhenReady } from "~/lib/modalDialog";
import { imageUrlByPrint, searchPrints } from "~/lib/scryfall";
import { Button, Felt, Field } from "~/components/atoms";

const BASICS = new Set(["Plains", "Island", "Swamp", "Mountain", "Forest"]);
const DECK_SIZE = 99;
const CONTEXT_MENU_PRESS_MS = 500;

/** A card that can be this deck's commander: a legendary creature (the only commanders in pool). */
const canBeCommander = (c: CatalogCard) => c.legendary && c.kind.kind === "creature";

type MenuItem = { label: string; run: () => void };

// One server page. The server caps `limit` at 200 (MAX_LIMIT); we page in PAGE-sized chunks and
// accumulate them in the grid as the user scrolls (see the sentinel/IntersectionObserver below).
const PAGE = 100;

// A pool tile and a decklist row are both list rows; only their density differs.
const LIST_ROW = "border border-vine-dim bg-glass-dim text-snow hover:bg-white/8";
const POOL_CARD = cn(
  LIST_ROW,
  "flex cursor-pointer flex-col items-center gap-1 rounded-hud p-sm text-caption focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const DECK_ROW = cn(
  LIST_ROW,
  "flex w-full cursor-pointer items-center gap-xs rounded-[5px] px-sm py-1 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const MENU_ITEM =
  "cursor-pointer rounded-[5px] border-none bg-transparent px-md py-xs text-left text-label text-snow hover:bg-white/8 focus-visible:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine";
const PRINT_PICKER_COL = "w-[min(38vw,200px)]";
const PRINT_TILE = cn(
  PRINT_PICKER_COL,
  "flex cursor-pointer flex-col items-center gap-1.5 rounded-hud p-md text-label hover:bg-white/8 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-vine",
);
const PRINT_PICKER_GRID = "grid w-fit grid-cols-2 gap-md";
const PRINT_BADGE =
  "rounded-full border border-vine-dim bg-glass-dim px-[7px] py-px font-semibold text-chip text-lichen";
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
const hydrateCardsFn = Atom.fn((ids: string[]) => lookupCardsByIds(ids));

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
  // superseded query/offset (the reset above bumps both).
  createEffect(() => {
    const page = results();
    if (!page || page.q !== debouncedQuery() || page.offset !== offset()) return;
    untrack(() => {
      const seen = new Set(pool().map((c) => c.id));
      setPool([...pool(), ...page.cards.filter((c) => !seen.has(c.id))]);
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
  // The commander, by Card id, and the Printing UUID chosen for its art.
  const [commander, setCommander] = createStore({ id: "", print: "" });
  // Card id → { count, print }.
  const [entries, setEntries] = createStore<Record<string, { count: number; print: string }>>({});
  // Sticky art preference per Card id, seeded from `default_print` the first time a card is seen
  // and overwritten whenever the player explicitly chooses a printing (session-local, not saved).
  const [preferredPrint, setPreferredPrint] = createStore<Record<string, string>>({});
  const [problems, setProblems] = createStore<{ list: string[] }>({ list: [] });
  // True once the user has changed name/commander/list since the deck loaded. Drives the
  // Cancel button: a clean builder navigates away immediately, a dirty one confirms first.
  const [dirty, setDirty] = createSignal(false);
  const [confirmDiscard, setConfirmDiscard] = createSignal(false);
  // Card data we've seen, keyed by Card id — accumulated from search results and hydrated for a
  // loaded deck. Lets the commander/decklist resolve color identity and a display name without
  // loading the full pool.
  const [known, setKnown] = createStore<Record<string, CatalogCard>>({});
  const remember = (cards: readonly CatalogCard[] | undefined) => {
    if (!cards) return;
    setKnown(
      produce((k) => {
        for (const c of cards) k[c.id] = c;
      }),
    );
    setPreferredPrint(
      produce((p) => {
        for (const c of cards) if (!(c.id in p)) p[c.id] = c.default_print;
      }),
    );
  };
  // The pool card currently hovered, for the read-the-text preview.
  const [hover, setHover] = createSignal<{ id: string; print: string; x: number; y: number } | null>(null);
  // The right-click context menu: a title, its items, and where to show it.
  const [menu, setMenu] = createSignal<{ title: string; items: MenuItem[]; x: number; y: number } | null>(null);
  // The print picker: which Card id it's choosing for, and what to do with the choice.
  const [printPicker, setPrintPicker] = createSignal<{ oracleId: string; onPick: (printId: string) => void } | null>(
    null,
  );
  // Deck id we have already prefilled — blocks atom re-fetch from clobbering in-progress edits.
  const [prefilledDeckId, setPrefilledDeckId] = createSignal<number | null>(null);

  let suppressClick = false;
  let menuPressTimer: ReturnType<typeof setTimeout> | undefined;
  let menuPressOrigin: { x: number; y: number } | null = null;

  const clearMenuPress = () => {
    if (menuPressTimer) clearTimeout(menuPressTimer);
    menuPressTimer = undefined;
    menuPressOrigin = null;
  };

  const openMenuAt = (title: string, items: MenuItem[], x: number, y: number) => {
    setHover(null);
    setMenu({ title, items, x, y });
  };

  const startMenuPress = (title: string, items: MenuItem[], e: PointerEvent) => {
    if (e.button !== 0) return;
    clearMenuPress();
    const x = e.clientX;
    const y = e.clientY;
    menuPressOrigin = { x, y };
    menuPressTimer = setTimeout(() => {
      menuPressTimer = undefined;
      suppressClick = true;
      openMenuAt(title, items, x, y);
    }, CONTEXT_MENU_PRESS_MS);
  };

  const moveMenuPress = (e: PointerEvent) => {
    if (!menuPressTimer || !menuPressOrigin) return;
    const dx = e.clientX - menuPressOrigin.x;
    const dy = e.clientY - menuPressOrigin.y;
    if (dx * dx + dy * dy > 100) clearMenuPress();
  };

  createEffect(() => {
    if (!menu() || printPicker()) return;
    const dismiss = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenu(null);
    };
    document.addEventListener("keydown", dismiss);
    onCleanup(() => document.removeEventListener("keydown", dismiss));
  });

  createEffect(() => remember(results()?.cards));

  // Reset on route change; prefill once per loaded deck id.
  createEffect(
    on(editingId, (id, prevId) => {
      if (id === null) {
        setName("value", "New deck");
        setCommander({ id: "", print: "" });
        setEntries({});
        setPreferredPrint({});
        setPrefilledDeckId(null);
        setDirty(false);
        return;
      }
      if (prevId !== undefined && id !== prevId) {
        setPrefilledDeckId(null);
      }
    }),
  );

  createEffect(() => {
    const id = editingId();
    const deck = existing();
    if (id === null || !deck || prefilledDeckId() === id) return;
    setName("value", deck.name);
    setCommander({ id: deck.commander, print: deck.commander_print });
    setEntries(reconcileEntries(deck.cards));
    setPreferredPrint(
      produce((p) => {
        if (deck.commander && deck.commander_print) p[deck.commander] = deck.commander_print;
        for (const c of deck.cards) if (c.print) p[c.id] = c.print;
      }),
    );
    const ids = deck.cards.map((c) => c.id);
    if (deck.commander) ids.push(deck.commander);
    void hydrateCards(ids).then(remember);
    setPrefilledDeckId(id);
    setDirty(false);
  });

  const commanderIdentity = () => known[commander.id]?.color_identity ?? [];
  const offIdentity = (c: CatalogCard) => c.color_identity.some((i) => !commanderIdentity().includes(i));

  const total = createMemo(() => Object.values(entries).reduce((sum, e) => sum + e.count, 0));

  /** The print a fresh add of this card should use: the sticky preference, else its default. */
  const printFor = (card: CatalogCard) => preferredPrint[card.id] ?? card.default_print;

  const setCount = (card: CatalogCard, count: number) => {
    setDirty(true);
    const id = card.id;
    if (count <= 0) {
      setEntries(produce((e) => delete e[id]));
      return;
    }
    setEntries(id, (e) => ({ count: BASICS.has(card.name) ? count : 1, print: e?.print ?? printFor(card) }));
  };
  const addN = (card: CatalogCard, count: number) => setCount(card, (entries[card.id]?.count ?? 0) + count);
  const add = (card: CatalogCard) => addN(card, 1);
  /** Remove `count` copies (deletes the entry at zero). */
  const removeN = (card: CatalogCard, count: number) => setCount(card, (entries[card.id]?.count ?? 0) - count);
  /** Add one copy at an explicitly chosen printing (the pool's "Choose print" flow), ignoring the
   * sticky preference for this one add — the caller updates the preference itself. */
  const addOneWithPrint = (card: CatalogCard, printId: string) => {
    setDirty(true);
    const id = card.id;
    setEntries(id, (e) => ({ count: BASICS.has(card.name) ? (e?.count ?? 0) + 1 : 1, print: printId }));
  };

  const setCommanderDirty = (card: CatalogCard | null) => {
    setDirty(true);
    setCommander({ id: card?.id ?? "", print: card ? printFor(card) : "" });
  };
  const setCommanderPrint = (printId: string) => {
    setDirty(true);
    setCommander("print", printId);
    if (commander.id) {
      setPreferredPrint(commander.id, printId);
      if (commander.id in entries) setEntries(commander.id, "print", printId);
    }
  };

  const openPrintPicker = (oracleId: string, onPick: (printId: string) => void) => setPrintPicker({ oracleId, onPick });

  const removeItems = (c: CatalogCard): MenuItem[] => [
    { label: "Fill deck", run: () => addN(c, Math.max(0, DECK_SIZE - total())) },
    ...[1, 2, 5].map((k) => ({ label: `Remove ${k}`, run: () => removeN(c, k) })),
  ];

  /** Right-click menu items for a pool card: basics add in bulk (and can fill the deck out to
   * the card limit), commanders can be set as the commander, everything else just adds one.
   * "Choose print" only offers when this Card id isn't already in the deck — once it's in, the
   * deck row's own menu is where the print changes. */
  const menuItems = (c: CatalogCard): MenuItem[] => {
    const items: MenuItem[] = BASICS.has(c.name)
      ? [
          { label: "Add One", run: () => addN(c, 1) },
          { label: "Add Two", run: () => addN(c, 2) },
          { label: "Add Five", run: () => addN(c, 5) },
          { label: "Fill deck", run: () => addN(c, Math.max(0, DECK_SIZE - total())) },
        ]
      : canBeCommander(c)
        ? [
            { label: "Add One", run: () => add(c) },
            { label: "Set As Commander", run: () => setCommanderDirty(c) },
          ]
        : [{ label: "Add One", run: () => add(c) }];
    if (!(c.id in entries)) {
      items.push({
        label: "Choose print",
        run: () =>
          openPrintPicker(c.id, (printId) => {
            setPreferredPrint(c.id, printId);
            addOneWithPrint(c, printId);
          }),
      });
    }
    return items;
  };

  /** Deck row menu: bulk remove for basics (as before), plus "Choose print" for every row —
   * changing an already-added card's print does not touch its count. */
  const rowMenuItems = (row: { id: string; count: number }): MenuItem[] => {
    const card = known[row.id];
    const items: MenuItem[] = card && BASICS.has(card.name) ? removeItems(card) : [];
    items.push({
      label: "Choose print",
      run: () =>
        openPrintPicker(row.id, (printId) => {
          setDirty(true);
          setEntries(row.id, "print", printId);
          setPreferredPrint(row.id, printId);
          const cmdPrint = commanderPrintForRow(commander.id, row.id, printId);
          if (cmdPrint) setCommander("print", cmdPrint);
        }),
    });
    return items;
  };

  const deckList = createMemo(() =>
    Object.entries(entries)
      .map(([id, e]) => ({ id, count: e.count, print: e.print, name: known[id]?.name ?? id }))
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
      commander: commander.id,
      commander_print: commander.print,
      cards: Object.entries(entries).map(([id, e]): DeckCardEntry => ({ id, count: e.count, print: e.print })),
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
        <div class="text-label text-lichen">
          Click to add. Right-click or long-press for print and other options. Only basics may exceed one copy.
        </div>
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
                title="Right-click or long-press for more options"
                onClick={() => {
                  if (suppressClick) {
                    suppressClick = false;
                    return;
                  }
                  add(c);
                }}
                onPointerDown={(e) => startMenuPress(c.name, menuItems(c), e)}
                onPointerMove={moveMenuPress}
                onPointerUp={clearMenuPress}
                onPointerCancel={clearMenuPress}
                onPointerLeave={clearMenuPress}
                onContextMenu={(e) => {
                  e.preventDefault();
                  clearMenuPress();
                  openMenuAt(c.name, menuItems(c), e.clientX, e.clientY);
                }}
                onMouseMove={(e) => setHover({ id: c.id, print: printFor(c), x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
                class={cn(POOL_CARD, commander.id && offIdentity(c) && "opacity-40")}
              >
                <img src={imageUrlByPrint(printFor(c))} alt={c.name} loading="lazy" class={CARD_ART} />
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
          when={commander.id}
          fallback={
            <div class="text-label text-lichen">
              Right-click or long-press a legendary creature to set commander or choose its art.
            </div>
          }
        >
          <button
            type="button"
            title="Click to remove · right-click or long-press to change art"
            onClick={() => {
              if (suppressClick) {
                suppressClick = false;
                return;
              }
              setCommanderDirty(null);
            }}
            onPointerDown={(e) => {
              const title = known[commander.id]?.name ?? commander.id;
              startMenuPress(title, [{ label: "Choose print", run: () => openPrintPicker(commander.id, setCommanderPrint) }], e);
            }}
            onPointerMove={moveMenuPress}
            onPointerUp={clearMenuPress}
            onPointerCancel={clearMenuPress}
            onPointerLeave={clearMenuPress}
            onContextMenu={(e) => {
              e.preventDefault();
              clearMenuPress();
              openMenuAt(known[commander.id]?.name ?? commander.id, [
                { label: "Choose print", run: () => openPrintPicker(commander.id, setCommanderPrint) },
              ], e.clientX, e.clientY);
            }}
            onMouseMove={(e) => setHover({ id: commander.id, print: commander.print, x: e.clientX, y: e.clientY })}
            onMouseLeave={() => setHover(null)}
            class="flex w-full cursor-pointer items-center gap-sm rounded-control border border-vine bg-glass-dim px-sm py-xs text-left"
          >
            <img
              src={imageUrlByPrint(commander.print)}
              alt={known[commander.id]?.name ?? commander.id}
              class="aspect-[0.72] w-10 rounded-focus object-cover"
            />
            <span class="min-w-0 flex-1 truncate font-semibold">★ {known[commander.id]?.name ?? commander.id}</span>
          </button>
        </Show>

        <div class="flex items-center justify-between gap-sm">
          <b>Cards</b>
          <span class={cn("shrink-0 text-caution-amber", total() === DECK_SIZE && "text-vine")}>
            {total()}/{DECK_SIZE}
            {commander.id ? " + commander" : ""}
          </span>
        </div>

        <div class="flex max-h-[40vh] min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          <For each={deckList()}>
            {(row) => (
              <button
                type="button"
                title="Click to remove one · right-click or long-press for print"
                onClick={() => {
                  if (suppressClick) {
                    suppressClick = false;
                    return;
                  }
                  const card = known[row.id];
                  if (card) removeN(card, 1);
                  setHover(null);
                }}
                onPointerDown={(e) => startMenuPress(row.name, rowMenuItems(row), e)}
                onPointerMove={moveMenuPress}
                onPointerUp={clearMenuPress}
                onPointerCancel={clearMenuPress}
                onPointerLeave={clearMenuPress}
                onMouseMove={(e) => setHover({ id: row.id, print: row.print, x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  clearMenuPress();
                  openMenuAt(row.name, rowMenuItems(row), e.clientX, e.clientY);
                }}
                class={DECK_ROW}
              >
                <img
                  src={imageUrlByPrint(row.print)}
                  alt=""
                  aria-hidden
                  class="aspect-[0.72] w-7 shrink-0 rounded-[3px] object-cover"
                />
                <span class="min-w-0 flex-1 truncate">
                  {known[row.id]?.legendary ? "★ " : ""}
                  {row.name}
                  <Show when={row.id === commander.id}>
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

      <CardPreview id={hover()?.id ?? null} print={hover()?.print} x={hover()?.x ?? 0} y={hover()?.y ?? 0} />

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
                    class={MENU_ITEM}
                  >
                    {item.label}
                  </button>
                )}
              </For>
            </div>
          </>
        )}
      </Show>

      <Show when={printPicker()}>
        {(pp) => (
          <PrintPicker
            oracleId={pp().oracleId}
            onPick={(printId) => {
              pp().onPick(printId);
              setPrintPicker(null);
            }}
            onClose={() => setPrintPicker(null)}
          />
        )}
      </Show>
    </Felt>
  );
}

/** Scryfall print picker; `createResource` so a failed fetch retries on reopen. */
function PrintPicker(props: { oracleId: string; onPick: (printId: string) => void; onClose: () => void }) {
  let dialog!: HTMLDialogElement;
  const [prints] = createResource(
    () => props.oracleId,
    (id) => (id ? searchPrints(id) : Promise.resolve([])),
  );

  onMount(() => onCleanup(openModalWhenReady(dialog)));
  onCleanup(() => {
    if (dialog?.open) dialog.close();
  });

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: backdrop click's keyboard equivalent is Escape, which showModal() wires natively into onClose.
    <dialog
      ref={dialog}
      onClose={() => props.onClose()}
      onClick={(e) => e.target === dialog && props.onClose()}
      class={cn(
        "m-auto w-fit max-w-[90vw] rounded-modal border border-vine bg-forest-surface p-xl text-body text-snow shadow-table",
        "backdrop:bg-black/60",
      )}
    >
      <div class="flex w-fit max-w-full flex-col gap-md">
        <div class="flex items-center justify-between gap-lg">
          <div class="font-semibold text-body">Choose printing</div>
          <Button type="button" autofocus onClick={props.onClose} variant="ghost" hitQuiet>
            Close
          </Button>
        </div>
        <div class={cn(PRINT_PICKER_GRID, "max-h-[min(60vh,720px)] overflow-y-auto")}>
          <Show when={prints.loading}>
            <For each={Array.from({ length: 4 })}>
              {() => (
                <div class={cn(PRINT_TILE, "pointer-events-none cursor-default")}>
                  <div class={cn(CARD_ART, "animate-skeleton bg-white/8")} />
                  <div class="flex flex-wrap justify-center gap-1">
                    <div class="h-[18px] w-10 animate-skeleton rounded-full bg-white/8" />
                    <div class="h-[18px] w-8 animate-skeleton rounded-full bg-white/8" />
                    <div class="h-[18px] w-16 animate-skeleton rounded-full bg-white/8" />
                  </div>
                </div>
              )}
            </For>
          </Show>
          <Show when={prints.error}>
            <div class="col-span-2 text-burn-red text-label">Could not load printings. Close and try again.</div>
          </Show>
          <Show when={!prints.loading && !prints.error}>
            <For each={prints() ?? []}>
              {(p) => (
                <button type="button" onClick={() => props.onPick(p.id)} class={PRINT_TILE}>
                  <img src={imageUrlByPrint(p.id)} alt={`${p.set_name} #${p.collector_number}`} class={CARD_ART} />
                  <div class="flex w-full flex-wrap items-center justify-center gap-1">
                    <span class={PRINT_BADGE} title={p.set_name}>
                      {p.set.toUpperCase()}
                    </span>
                    <span class={PRINT_BADGE}>#{p.collector_number}</span>
                    <span class={PRINT_BADGE}>{formatReleasedAt(p.released_at)}</span>
                  </div>
                </button>
              )}
            </For>
            <Show when={(prints() ?? []).length === 0}>
              <div class="col-span-2 text-label text-lichen">No printings found.</div>
            </Show>
          </Show>
        </div>
      </div>
    </dialog>
  );
}
