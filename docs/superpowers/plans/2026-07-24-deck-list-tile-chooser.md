# Deck List Tile Chooser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign `/` (Your decks) into a compact tile grid with name/commander search, customs-then-reverse-release-precons ordering, Play-on-tile-click, and Edit/Delete via right-click context menu.

**Architecture:** Client-only. Pure helpers (`visibleDecks`, identity pip codes, menu eligibility) drive display order/filter. Foldkit list submodel gains `searchQuery` + `contextMenu`; view becomes a responsive tile grid with a search field and a builder-style context-menu overlay. No wire/API changes.

**Tech Stack:** Foldkit (Html / Messages / Mount / Scene), Effect Schema, Vitest, TypeScript, mana-font via `manaFontClass`.

**Spec:** [deck-list-tile-chooser-design](../specs/2026-07-24-deck-list-tile-chooser-design.md)  
**Current-behavior update:** [client-shell-deck-builder-and-observability](../specs/2026-07-20-client-shell-deck-builder-and-observability.md)

## Global Constraints

- No `.proto`, BFF, or `DeckSummary` schema changes.
- Lobby deck `<select>` / Bring strip unchanged.
- Guard-return-first; imports at top of file; exhaustive message matching.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages on branch `cursor/deck-list-redesign-b23c`.
- Scene/outcome tests assert product behavior (tile Play href, search, order, context menu), not migration/parity.
- Exact copy: search placeholder `Search decks…`; no-match `No decks match.`; keep existing empty-library / loading / error strings.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/app/shell/decks/list/visible.ts` | `visibleDecks`, `identityPipCodes`, `deckListContextMenuAllowed` |
| `client/app/shell/decks/list/visible.test.ts` | Unit tests for helpers |
| `client/app/shell/decks/list/submodel.ts` | `searchQuery`, `contextMenu` |
| `client/app/shell/decks/list/messages.ts` | Search + context menu messages |
| `client/app/shell/decks/list/update.ts` | Fold new messages |
| `client/app/shell/decks/messages.ts` | Re-export new list messages |
| `client/app/messages.ts` | Re-export for app Message union |
| `client/app/update.ts` | `tagsExhaustive` arms for new list messages |
| `client/app/shell/decks/list/view.ts` | Search field, tile grid, context menu, hover |
| `client/app/shell/decks/list/story.test.ts` | Focused Scene: search, order, menu, tile href |
| `client/app/shell/surfaces.test.ts` | Update deck-list chrome assertions |
| `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md` | Document current tile-chooser behavior |

---

### Task 1: Pure visibleDecks + identity helpers

**Files:**
- Create: `client/app/shell/decks/list/visible.ts`
- Create: `client/app/shell/decks/list/visible.test.ts`

**Interfaces:**
- Consumes: `DeckSummary` from `~/wire/types`; `CatalogCard` / `CatalogCardSchema.Type` from deck-builder cards
- Produces:
  - `visibleDecks(decks, knownCommanders, query): DeckSummary[]`
  - `identityPipCodes(colorIdentity: readonly number[]): string[]` — WUBRG letter codes for indices in `0..4`
  - `deckListContextMenuAllowed(deckId: number): boolean` — `true` iff `deckId > 0`

- [ ] **Step 1: Write the failing test**

```ts
// client/app/shell/decks/list/visible.test.ts
import { describe, expect, it } from "vitest";
import type { CatalogCard } from "../../../../lib/wire/types";
import { deckListContextMenuAllowed, identityPipCodes, visibleDecks } from "./visible";

const card = (id: string, name: string, color_identity: number[] = []): CatalogCard => ({
  color_identity,
  cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
  default_print: `${id}-print`,
  id,
  keywords: [],
  kind: { kind: "creature", power: 1, toughness: 1 },
  legendary: true,
  name,
  oracle: "",
  otags: [],
  set: "tst",
  subtypes: [],
  summary: "",
});

describe("visibleDecks", () => {
  const decks = [
    { id: 2, name: "Beta", commander: "b", commander_print: "" },
    { id: -1, name: "Silverquill Influence", commander: "s", commander_print: "" },
    { id: 1, name: "Alpha", commander: "a", commander_print: "" },
    { id: -9, name: "Mirror Mastery", commander: "m", commander_print: "" },
    { id: -5, name: "Quandrix Unlimited", commander: "q", commander_print: "" },
  ];
  const known = {
    a: card("a", "Atraxa, Praetors' Voice"),
    b: card("b", "Beledros Witherbloom"),
    m: card("m", "Riku of Two Reflections"),
    s: card("s", "Breena, the Demagogue"),
    q: card("q", "Adrix and Nev, Timelocked"),
  };

  it("puts customs first preserving relative order, then precons by ascending id", () => {
    const ids = visibleDecks(decks, known, "").map((d) => d.id);
    expect(ids).toEqual([2, 1, -9, -5, -1]);
  });

  it("filters by deck name case-insensitively", () => {
    expect(visibleDecks(decks, known, "mirror").map((d) => d.id)).toEqual([-9]);
  });

  it("filters by commander display name", () => {
    expect(visibleDecks(decks, known, "atraxa").map((d) => d.id)).toEqual([1]);
  });

  it("falls back to commander id when unknown", () => {
    const orphan = [{ id: 9, name: "Orphan", commander: "mystery-id", commander_print: "" }];
    expect(visibleDecks(orphan, {}, "mystery").map((d) => d.id)).toEqual([9]);
  });

  it("returns empty when nothing matches a non-empty library filter", () => {
    expect(visibleDecks(decks, known, "zzzz").map((d) => d.id)).toEqual([]);
  });
});

describe("identityPipCodes", () => {
  it("maps WUBRG indices in order given", () => {
    expect(identityPipCodes([0, 2, 4])).toEqual(["W", "B", "G"]);
  });
  it("skips out-of-range indices", () => {
    expect(identityPipCodes([-1, 5, 1])).toEqual(["U"]);
  });
});

describe("deckListContextMenuAllowed", () => {
  it("allows owned decks only", () => {
    expect(deckListContextMenuAllowed(1)).toBe(true);
    expect(deckListContextMenuAllowed(-1)).toBe(false);
    expect(deckListContextMenuAllowed(0)).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/shell/decks/list/visible.test.ts`

Expected: FAIL (module not found)

- [ ] **Step 3: Write minimal implementation**

```ts
// client/app/shell/decks/list/visible.ts
import type { CatalogCard } from "../../../../lib/wire/types";
import type { DeckSummary } from "../../../../lib/wire/types";

const COLOR_PIP = ["W", "U", "B", "R", "G"] as const;

export function identityPipCodes(colorIdentity: readonly number[]): string[] {
  const out: string[] = [];
  for (const i of colorIdentity) {
    if (i < 0 || i > 4) continue;
    out.push(COLOR_PIP[i]);
  }
  return out;
}

export function deckListContextMenuAllowed(deckId: number): boolean {
  return deckId > 0;
}

export function visibleDecks(
  decks: readonly DeckSummary[],
  knownCommanders: Readonly<Record<string, CatalogCard>>,
  query: string,
): DeckSummary[] {
  const q = query.trim().toLowerCase();
  const matched =
    q === ""
      ? [...decks]
      : decks.filter((deck) => {
          if (deck.name.toLowerCase().includes(q)) return true;
          const commander = knownCommanders[deck.commander];
          const commanderLabel = (commander?.name ?? deck.commander).toLowerCase();
          return commanderLabel.includes(q);
        });

  const customs = matched.filter((d) => d.id > 0);
  const precons = matched.filter((d) => d.id < 0).sort((a, b) => a.id - b.id);
  return [...customs, ...precons];
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bunx vitest run app/shell/decks/list/visible.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/visible.ts client/app/shell/decks/list/visible.test.ts
git commit -m "feat(client): add deck list visibleDecks ordering and search helpers"
```

---

### Task 2: Submodel + messages + update wiring

**Files:**
- Modify: `client/app/shell/decks/list/submodel.ts`
- Modify: `client/app/shell/decks/list/messages.ts`
- Modify: `client/app/shell/decks/list/update.ts`
- Modify: `client/app/shell/decks/messages.ts`
- Modify: `client/app/messages.ts`
- Modify: `client/app/update.ts` (add `tagsExhaustive` arms that call `foldDeckList`)
- Test: extend `client/app/shell/decks/list/visible.test.ts` is done; add a small update unit test file OR assert via story in Task 3 — for this task add `client/app/shell/decks/list/update.search.test.ts`

**Interfaces:**
- Consumes: Task 1 helpers (not required in update itself)
- Produces messages:
  - `ChangedDeckListSearch({ query: string })`
  - `OpenedDeckListMenu({ deckId: number, x: number, y: number })`
  - `ClosedDeckListMenu()`
- Submodel fields:
  - `searchQuery: string` (default `""`)
  - `contextMenu: null | { deckId: number, x: number, y: number }`

- [ ] **Step 1: Write the failing update test**

```ts
// client/app/shell/decks/list/update.search.test.ts
import { describe, expect, it } from "vitest";
import {
  ChangedDeckListSearch,
  ClosedDeckListMenu,
  OpenedDeckListMenu,
} from "./messages";
import { initialDeckListSubmodel } from "./submodel";
import { update } from "./update";

describe("deck list search and menu update", () => {
  it("stores search query", () => {
    const [next] = update(initialDeckListSubmodel(), ChangedDeckListSearch({ query: "atraxa" }));
    expect(next.searchQuery).toBe("atraxa");
  });

  it("opens and closes context menu", () => {
    const [open] = update(
      initialDeckListSubmodel(),
      OpenedDeckListMenu({ deckId: 7, x: 10, y: 20 }),
    );
    expect(open.contextMenu).toEqual({ deckId: 7, x: 10, y: 20 });
    const [closed] = update(open, ClosedDeckListMenu());
    expect(closed.contextMenu).toBeNull();
  });

  it("ignores OpenedDeckListMenu for precon ids", () => {
    const [next] = update(
      initialDeckListSubmodel(),
      OpenedDeckListMenu({ deckId: -1, x: 1, y: 2 }),
    );
    expect(next.contextMenu).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/shell/decks/list/update.search.test.ts`

Expected: FAIL (messages / fields missing)

- [ ] **Step 3: Implement submodel, messages, update, re-exports, app fold**

`submodel.ts` — add to struct + initial:

```ts
searchQuery: S.String,
contextMenu: S.NullOr(S.Struct({ deckId: S.Number, x: S.Number, y: S.Number })),
// initial: searchQuery: "", contextMenu: null
```

`messages.ts` — add:

```ts
export const ChangedDeckListSearch = m("ChangedDeckListSearch", { query: S.String });
export const OpenedDeckListMenu = m("OpenedDeckListMenu", {
  deckId: S.Number,
  x: S.Number,
  y: S.Number,
});
export const ClosedDeckListMenu = m("ClosedDeckListMenu");
// include in Message union
```

`update.ts` — import `deckListContextMenuAllowed` from `./visible`; add arms:

```ts
ChangedDeckListSearch: ({ query }) => [{ ...model, searchQuery: query }, []],
OpenedDeckListMenu: ({ deckId, x, y }) => {
  if (!deckListContextMenuAllowed(deckId)) return [model, []];
  return [{ ...model, contextMenu: { deckId, x, y } }, []];
},
ClosedDeckListMenu: () => [{ ...model, contextMenu: null }, []],
```

Also clear `contextMenu` when opening delete confirm / after delete if convenient:

```ts
AskedDeckDelete: ({ id }) => [
  { ...model, confirmingDeleteId: id, error: null, contextMenu: null },
  [],
],
```

`decks/messages.ts` + `app/messages.ts`: re-export the three new messages.

`app/update.ts`: add three `tagsExhaustive` arms forwarding to `foldDeckList` (same pattern as `MovedDeckListHover`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bunx vitest run app/shell/decks/list/update.search.test.ts`

Expected: PASS

Also run typecheck arm quickly: `cd client && bunx tsc --noEmit` — fix any missing exhaustive arms.

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/submodel.ts client/app/shell/decks/list/messages.ts \
  client/app/shell/decks/list/update.ts client/app/shell/decks/list/update.search.test.ts \
  client/app/shell/decks/messages.ts client/app/messages.ts client/app/update.ts
git commit -m "feat(client): wire deck list search and context menu state"
```

---

### Task 3: Tile grid + search UI

**Files:**
- Modify: `client/app/shell/decks/list/view.ts`
- Modify: `client/app/shell/surfaces.test.ts`
- Modify: `client/app/shell/decks/list/story.test.ts`

**Interfaces:**
- Consumes: `visibleDecks`, `identityPipCodes` from `./visible`
- Consumes: `ChangedDeckListSearch` from `./messages`
- Produces: tiles with `data-testid={`deck-tile-${deck.id}`}` and `href` `/play?deck={id}`; search input `data-testid="deck-list-search"`

- [ ] **Step 1: Write / update failing Scene tests**

In `surfaces.test.ts`, replace the delete-button assertion in `"renders deck list chrome…"`:

```ts
Scene.expect(Scene.selector('[data-testid="decks-page"]')).toExist(),
Scene.expect(Scene.selector('[data-testid="deck-list-search"]')).toExist(),
Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).toExist(),
Scene.expect(Scene.selector('[data-testid="delete-deck-1"]')).not.toExist(),
Scene.expect(Scene.text("Your decks")).toExist(),
Scene.expect(Scene.text("Superfriends")).toExist(),
// keep Mount.resolve for hover + card art
```

In `story.test.ts`, use `Story` + `Scene` like the builder stories (`Story.message` folds through `update`):

```ts
import { Story } from "foldkit/test";
import { ChangedDeckListSearch } from "./messages";
import { update } from "./update";

const listProgram = { update, view: listView };

test("tile Play href uses ?deck= and search filters tiles", () => {
  const knownCommanders = {
    atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice", color_identity: [0, 1, 2, 4] }),
    breena: card({ id: "breena", name: "Breena, the Demagogue" }),
    riku: card({ id: "riku", name: "Riku of Two Reflections" }),
  };
  const decks = [
    { id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" },
    { id: -1, name: "Silverquill Influence", commander: "breena", commander_print: "" },
    { id: -9, name: "Mirror Mastery", commander: "riku", commander_print: "" },
  ];

  Scene.scene(
    listProgram,
    Scene.with({ ...initialDeckListSubmodel(), decks, knownCommanders }),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"][href="/play?deck=1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--1"]')).toExist(),
    Story.message(ChangedDeckListSearch({ query: "mirror" })),
    Scene.expect(Scene.selector('[data-testid="deck-tile-1"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-tile--9"]')).toExist(),
    Story.message(ChangedDeckListSearch({ query: "zzzz" })),
    Scene.expect(Scene.text("No decks match.")).toExist(),
    Scene.Mount.resolveAll([BindCardArt, CardArtTick()], [BindCardArt, CardArtTick()]),
  );
});
```

Add a local `card()` helper (copy from `surfaces.test.ts`) at the top of `story.test.ts`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bunx vitest run app/shell/surfaces.test.ts app/shell/decks/list/story.test.ts`

Expected: FAIL on missing `deck-list-search` / `deck-tile-*`

- [ ] **Step 3: Implement view**

Rewrite the decks section in `view.ts`:

1. Import `fieldClass`, `ChangedDeckListSearch`, `visibleDecks`, `identityPipCodes`, `manaFontClass` from `~/oracleText`, `PlayRoute` / `routePath` (already have PlayRoute usage).
2. Above the grid, when `!loading && decks.length > 0`, render:

```ts
h.input([
  h.Type("search"),
  h.DataAttribute("testid", "deck-list-search"),
  h.Placeholder("Search decks…"),
  h.Value(model.searchQuery),
  h.OnInput((value) => ChangedDeckListSearch({ query: value })),
  h.Class(fieldClass("mb-md w-full max-w-[720px]")),
], [])
```

3. Replace `...model.decks.map` with `visibleDecks(model.decks, model.knownCommanders, model.searchQuery).map`.
4. Section layout: `mx-auto grid max-w-[960px] grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-md` (tune minmax to feel dense on landscape).
5. Each tile is an `<a>`:

```ts
h.a(
  [
    h.Href(`${routePath(PlayRoute())}?deck=${deck.id}`),
    h.DataAttribute("testid", `deck-tile-${deck.id}`),
    h.Class(
      listRowClass(
        "relative flex flex-col overflow-hidden rounded-hud no-underline text-snow",
      ),
    ),
    h.OnMount(
      BindDeckListCommanderHover({
        cardId: deck.commander,
        print: commanderPrint(model, deck),
      }),
    ),
  ],
  [
    // art_crop full-bleed top (~110px) or empty glass
    // name (font-semibold text-label, truncate)
    // Precon chip if id < 0
    // identity pips: for each code in identityPipCodes(...),
    //   h.i([h.Class(`ms ms-cost ms-${manaFontClass(code)}`)], [])
  ],
);
```

6. Remove always-visible Play / Edit / Delete buttons from the tile.
7. Empty states:
   - `!loading && decks.length === 0` → existing build-first copy
   - `!loading && decks.length > 0 && visible.length === 0` → `No decks match.`
8. Keep delete confirm dialog + hover preview + header chrome.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/shell/surfaces.test.ts app/shell/decks/list/story.test.ts`

Expected: PASS (adjust attribute/DOM-order assertions to match available Scene API if needed)

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/view.ts client/app/shell/decks/list/story.test.ts \
  client/app/shell/surfaces.test.ts
git commit -m "feat(client): render Your decks as searchable tile grid"
```

---

### Task 4: Right-click context menu

**Files:**
- Modify: `client/app/shell/decks/list/view.ts`
- Modify: `client/app/shell/decks/list/story.test.ts`
- Modify: `client/app/shell/surfaces.test.ts` (delete dialog test still valid; ensure menu delete path covered in story)

**Interfaces:**
- Consumes: `OpenedDeckListMenu`, `ClosedDeckListMenu`, `AskedDeckDelete`, `deckListContextMenuAllowed`
- Produces: Mount `BindDeckListContextMenu({ deckId })`; overlay `data-testid="deck-list-context-menu"`; items `deck-list-menu-edit`, `deck-list-menu-delete`

- [ ] **Step 1: Write failing Scene / Mount tests**

```ts
test("owned deck context menu offers Edit and Delete", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      decks: [
        { id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" },
        { id: -1, name: "Silverquill Influence", commander: "breena", commander_print: "" },
      ],
      knownCommanders: {
        atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }),
        breena: card({ id: "breena", name: "Breena, the Demagogue" }),
      },
      contextMenu: { deckId: 1, x: 40, y: 50 },
    }),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-menu-edit"][href="/decks/1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-menu-delete"]')).toExist(),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: 1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(BindDeckListContextMenu({ deckId: -1 }), ClosedDeckListMenu()),
    Scene.Mount.resolve(
      BindDeckListCommanderHover({ cardId: "atraxa", print: "atraxa-print" }),
      ClearedDeckListHover(),
    ),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});

test("menu Delete opens the confirm dialog", () => {
  Scene.scene(
    listProgram,
    Scene.with({
      ...initialDeckListSubmodel(),
      decks: [{ id: 1, name: "Superfriends", commander: "atraxa", commander_print: "atraxa-print" }],
      knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa, Praetors' Voice" }) },
      contextMenu: { deckId: 1, x: 40, y: 50 },
    }),
    Story.message(AskedDeckDelete({ id: 1 })),
    Scene.expect(Scene.selector('[data-testid="confirm-delete-dialog"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="deck-list-context-menu"]')).not.toExist(),
    Scene.Mount.resolve(OpenDialogAsModal(), ModalOpened()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});
```

Precon “no menu” is already covered by `update.search.test.ts` (`OpenedDeckListMenu` with `deckId: -1` leaves `contextMenu` null).

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts`

Expected: FAIL missing context menu testids / Mount

- [ ] **Step 3: Implement Mount + menu overlay**

Mirror builder pattern (`BindBuilderCardPointer` contextmenu + `contextMenu` view):

```ts
export const BindDeckListContextMenu = Mount.defineStream(
  "BindDeckListContextMenu",
  { deckId: S.Number },
  OpenedDeckListMenu,
  ClosedDeckListMenu,
)(
  (args) => (element) =>
    Stream.callback<typeof OpenedDeckListMenu.Type | typeof ClosedDeckListMenu.Type>((queue) =>
      Effect.gen(function* () {
        yield* Effect.acquireRelease(
          Effect.sync(() => {
            const onContextMenu = (event: Event) => {
              event.preventDefault();
              event.stopPropagation();
              if (!(event instanceof MouseEvent)) return;
              if (!deckListContextMenuAllowed(args.deckId)) return;
              Queue.offerUnsafe(
                queue,
                OpenedDeckListMenu({ deckId: args.deckId, x: event.clientX, y: event.clientY }),
              );
            };
            element.addEventListener("contextmenu", onContextMenu);
            return () => element.removeEventListener("contextmenu", onContextMenu);
          }),
          (teardown) => Effect.sync(teardown),
        );
        return yield* Effect.never;
      }),
    ),
);
```

Attach `OnMount(BindDeckListContextMenu({ deckId: deck.id }))` on every tile (precons still preventDefault).

Render menu when `model.contextMenu != null` (clamp x/y like builder):

- Catcher: fixed inset-0, click / contextmenu / Escape → `ClosedDeckListMenu`
- Panel: **Edit** as `<a href={routePath(DeckRoute({ id: String(deckId) }))} data-testid="deck-list-menu-edit">` with click also closing menu (`OnClick(ClosedDeckListMenu())` if Foldkit allows both — otherwise navigate-only and close via catcher unmount)
- **Delete** button `data-testid="deck-list-menu-delete"` → `AskedDeckDelete({ id: deckId })`
- Root `data-testid="deck-list-context-menu-root"`; panel `data-testid="deck-list-context-menu"`

Use the same `MENU_ITEM` / vine panel classes as the builder menu for visual consistency (copy the class string; do not extract a shared module unless one already exists).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts app/shell/decks/list/update.search.test.ts app/shell/decks/list/visible.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/view.ts client/app/shell/decks/list/story.test.ts
git commit -m "feat(client): add Edit/Delete context menu on deck tiles"
```

---

### Task 5: Shell spec + verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`
- Optionally mark design status in `docs/superpowers/specs/2026-07-24-deck-list-tile-chooser-design.md` as implemented (one-line status bump)

- [ ] **Step 1: Update current-behavior shell spec**

Replace the deck-list paragraph (~line 91) with current behavior:

```markdown
**Deck list** (`/`) shows saved decks from the deck list submodel as a compact tile grid.
Each tile uses commander `art_crop`, deck name, color-identity pips, and a Precon chip when
`id < 0`. The whole tile links to `/play?deck={id}`. A **Search decks…** field filters by
deck name and commander display name (client-only). Display order: owned decks first
(API relative order), then precons by ascending id (newest release first). Right-click on
an owned deck opens Edit (`/decks/{id}`) and Delete (confirm dialog); precons do not open
a context menu. A New Deck button navigates to `/decks/new`.
```

Add a user story if missing:

```markdown
- As a returning player on `/`, I scan commander tiles, search by name, click a tile to play,
  and right-click an owned deck to edit or delete it.
```

- [ ] **Step 2: Full client verification**

Run: `cd client && bun run typecheck && bunx vitest run`

Expected: typecheck exit 0; all tests PASS

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md \
  docs/superpowers/specs/2026-07-24-deck-list-tile-chooser-design.md
git commit -m "docs(client): record Your decks tile chooser as current behavior"
```

- [ ] **Step 4: Push and open/update PR**

```bash
git push -u origin cursor/deck-list-redesign-b23c
```

PR title: `feat(client): redesign Your decks as searchable tile grid`  
Body: summarize tiles, search, precon order, context menu; link design spec; Interaction/UI checklist checked; test plan lists typecheck + vitest + Scene outcomes.

---

## Spec coverage self-check

| Spec requirement | Task |
|------------------|------|
| Compact tile grid + art/name/pips/Precon chip | Task 3 |
| Whole tile → `/play?deck=` | Task 3 |
| No always-visible Play/Edit/Delete | Task 3 |
| Search by name + commander | Tasks 1, 2, 3 |
| No-match copy | Task 3 |
| Customs first, precons ascending id | Task 1, 3 |
| Context menu Edit/Delete owned only | Tasks 2, 4 |
| Precon right-click no menu | Tasks 2, 4 |
| Hover preview retained | Task 3 (keep existing Mount) |
| Lobby unchanged | (no lobby files) |
| Shell spec updated | Task 5 |
| Scene + unit tests | Tasks 1–4 |

## Placeholder / consistency check

- Message names are consistent: `ChangedDeckListSearch`, `OpenedDeckListMenu`, `ClosedDeckListMenu`.
- Testids: `deck-list-search`, `deck-tile-{id}`, `deck-list-context-menu`, `deck-list-menu-edit`, `deck-list-menu-delete`.
- Scene steps use in-repo `Story.message` + attribute selectors (`[href="…"]`), matching builder/lobby stories.
