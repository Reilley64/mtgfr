# Lobby Host/Join Entry Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the pre-table `/play/:deckId` entry so Host and Join are equal destination cards, with Join opening a focused code panel (Bringing strip + Cancel), without changing seated lobby or wire contracts.

**Architecture:** Add client-only `entryMode: "choose" | "join"` on the lobby slice. `choose` renders twin destination CTAs (Host wraps deck-card chrome; Join opens join mode). `join` replaces that row with a focused form. Host/join RPC, path-param deck routing, FLIP keyed by deck id, and seated `tableLobby` stay as today.

**Tech Stack:** Foldkit (Html / Scene / Command / messages), Effect Schema, Vitest, Tailwind token utilities, TypeScript.

**Spec:** [lobby-entry-ui](../specs/2026-07-20-lobby-entry-ui.md)  
**Current-behavior updates:** [shell-routes-and-auth](../specs/2026-07-20-shell-routes-and-auth.md)

## Global Constraints

- Entry surface only (`/play/:deckId`). Do not redesign seated lobby (code copy, seats, Ready, Start) or claim-seat beyond leaving it working.
- No `.proto`, BFF, route shape, or `DeckSummary` changes.
- Host creates immediately on click (no confirm). Join destination does not network until **Join table** submit.
- Guard-return-first; imports at top of file; exhaustive `M.tagsExhaustive` on lobby messages.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages on branch `cursor/lobby-host-join-entry-redesign-1c5b`.
- Scene/outcome tests assert product behavior (destinations visible, join mode swap, Cancel clears code), not migration/parity.
- Exact testids:
  - `lobby-host` — Host destination CTA
  - `lobby-open-join` — Join destination CTA (`choose` mode)
  - `lobby-join-code` — table code field (`join` mode)
  - `lobby-join` — Join table submit (`join` mode)
  - `lobby-join-cancel` — Cancel back to `choose`
  - `lobby-bringing` — compact Bringing strip
  - `lobby-deck-card` / `lobby-deck-card-{id}` — deck chrome inside Host card (and claim-seat unchanged)
  - `lobby-back` — Back to Your decks
  - `lobby-entry-choose` / `lobby-entry-join` — mode wrappers for Scene assertions
- FLIP target remains `data-deck-card-flip="{id}"` on deck chrome inside the Host destination.
- Skip mode-swap animation when `prefers-reduced-motion: reduce`.
- Panel may widen slightly for twin cards (e.g. `max-w` ~640px); keep landscape-first, no new nav chrome.

---

## File map

| File | Responsibility |
|------|----------------|
| `client/app/shell/lobby/submodel.ts` | `entryMode` on `LobbySlice`; default `"choose"`; reset via `initialLobbySlice` / `enterLobby` |
| `client/app/shell/lobby/messages.ts` | `RequestedLobbyOpenJoin`, `RequestedLobbyCancelJoin` |
| `client/app/shell/lobby/update.ts` | Mode transitions; Cancel clears `code` + join error; host/join RPC unchanged |
| `client/app/shell/lobby/update.test.ts` | Unit tests for mode + Cancel + host still hosts |
| `client/app/shell/lobby/view.ts` | Twin destinations + focused join panel; seated lobby untouched |
| `client/app/shell/lobby/entry.test.ts` | Entry Scene outcomes for choose/join/Cancel |
| `client/app/shell/surfaces.test.ts` | Shell surface coverage for choose (and join if needed) |
| `client/styles/global.css` (only if needed) | Optional `@media (prefers-reduced-motion)` note for entry swap class |
| `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md` | Document entry as current behavior once shipped |
| `docs/superpowers/specs/2026-07-20-lobby-entry-ui.md` | Flip Status to Implemented when done |

No changes to `deck-card-nav.ts` unless FLIP resolution breaks because the Host wrapper changed Mount ancestry — prefer keeping `renderDeckCard(..., { mode: "static" })` inside the Host button so existing Mount/FLIP continues to work.

---

### Task 1: `entryMode` state + open/cancel messages

**Files:**
- Modify: `client/app/shell/lobby/submodel.ts`
- Modify: `client/app/shell/lobby/messages.ts`
- Modify: `client/app/shell/lobby/update.ts`
- Modify: `client/app/shell/lobby/update.test.ts`

**Interfaces:**
- Consumes: Effect `S.Literal` / `S.Union` (same pattern as `AuthMode`)
- Produces:
  - `LobbyEntryMode = S.Union([S.Literal("choose"), S.Literal("join")])`
  - `LobbySlice.entryMode: "choose" | "join"`
  - `RequestedLobbyOpenJoin` — no payload
  - `RequestedLobbyCancelJoin` — no payload
  - `update` on OpenJoin → `{ ...model, entryMode: "join" }`
  - `update` on CancelJoin → `{ ...model, entryMode: "choose", code: "", error: null }` (clear join-attempt UI error; do not clear seated-lobby state because Cancel is entry-only)
  - `initialLobbySlice()` / table-id reset via `enterLobby` → `entryMode: "choose"`

- [ ] **Step 1: Write the failing tests**

```ts
// client/app/shell/lobby/update.test.ts — add describe block
import {
  RequestedLobbyCancelJoin,
  RequestedLobbyHost,
  RequestedLobbyOpenJoin,
} from "./messages";
import { CreateLobbyTable, update } from "./update";
import { initialLobbySlice } from "./submodel";

describe("lobby entryMode", () => {
  it("defaults to choose", () => {
    expect(initialLobbySlice().entryMode).toBe("choose");
  });

  it("open join switches to join mode without submitting", () => {
    const [next, commands] = update(initialLobbySlice(), RequestedLobbyOpenJoin(), []);
    expect(next.entryMode).toBe("join");
    expect(commands).toHaveLength(0);
  });

  it("cancel join returns to choose and clears code + error", () => {
    const model = {
      ...initialLobbySlice(),
      entryMode: "join" as const,
      code: "ABC123",
      error: "UnknownTable",
    };
    const [next, commands] = update(model, RequestedLobbyCancelJoin(), []);
    expect(next.entryMode).toBe("choose");
    expect(next.code).toBe("");
    expect(next.error).toBeNull();
    expect(commands).toHaveLength(0);
  });

  it("host still creates a table when a deck is selected", () => {
    const model = { ...initialLobbySlice(), selectedDeckId: 7, entryMode: "choose" as const };
    const [next, commands] = update(model, RequestedLobbyHost(), [7]);
    expect(next.submitting).toBe(true);
    expect(commands).toHaveLength(1);
    expect(commands[0]?.name).toBe(CreateLobbyTable.name);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/shell/lobby/update.test.ts`

Expected: FAIL (missing `entryMode` / unknown messages)

- [ ] **Step 3: Minimal implementation**

```ts
// client/app/shell/lobby/submodel.ts
export const LobbyEntryMode = S.Union([S.Literal("choose"), S.Literal("join")]);
export type LobbyEntryMode = typeof LobbyEntryMode.Type;

export const LobbySlice = S.Struct({
  tableId: S.NullOr(S.String),
  selectedDeckId: S.NullOr(S.Number),
  code: S.String,
  entryMode: LobbyEntryMode,
  view: S.NullOr(LobbyView),
  started: S.Boolean,
  error: S.NullOr(S.String),
  copied: S.Boolean,
  clipboardFallback: S.Boolean,
  submitting: S.Boolean,
});

export function initialLobbySlice(): LobbySlice {
  return {
    tableId: null,
    selectedDeckId: null,
    code: "",
    entryMode: "choose",
    view: null,
    started: false,
    error: null,
    copied: false,
    clipboardFallback: false,
    submitting: false,
  };
}
// enterLobby unchanged — spreading initialLobbySlice() on tableId change already resets entryMode
```

```ts
// client/app/shell/lobby/messages.ts — add + include in Message union
export const RequestedLobbyOpenJoin = m("RequestedLobbyOpenJoin");
export const RequestedLobbyCancelJoin = m("RequestedLobbyCancelJoin");
```

```ts
// client/app/shell/lobby/update.ts — inside M.tagsExhaustive
RequestedLobbyOpenJoin: () => [{ ...model, entryMode: "join" }, []],
RequestedLobbyCancelJoin: () => [
  { ...model, entryMode: "choose", code: "", error: null },
  [],
],
```

Fix any TypeScript breaks where `LobbySlice` literals are constructed without `entryMode` (spread `initialLobbySlice()` in tests/helpers).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/shell/lobby/update.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/submodel.ts client/app/shell/lobby/messages.ts \
  client/app/shell/lobby/update.ts client/app/shell/lobby/update.test.ts
git commit -m "feat(client): add lobby entryMode choose/join messages"
```

---

### Task 2: Choose mode — twin destination cards

**Files:**
- Modify: `client/app/shell/lobby/view.ts` (`entry` + helpers; leave `tableLobby` / `claimSeat` behavior)
- Modify: `client/app/shell/lobby/entry.test.ts`
- Modify: `client/app/shell/surfaces.test.ts`

**Interfaces:**
- Consumes: `renderDeckCard`, `RequestedLobbyHost`, `RequestedLobbyOpenJoin`, `entryMode`
- Produces: `choose` UI with testids above; code field **absent** in `choose`

- [ ] **Step 1: Write the failing Scene tests**

Replace / extend the entry assertions that currently expect `lobby-join-code` on `/play/:deckId` without opening join.

```ts
// client/app/shell/lobby/entry.test.ts
test("entry choose mode shows Host and Join destinations with deck on Host", () => {
  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(
      playLobbyModel({
        route: PlayRoute({ deckId: "9" }),
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: {
            ...init()[0].decks.list,
            decks: [deck, other],
            knownCommanders: { rhys: card({ id: "rhys", name: "Rhys the Redeemed" }) },
            loading: false,
          },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-entry-choose")).toExist(),
    Scene.expect(Scene.testId("lobby-host")).toExist(),
    Scene.expect(Scene.testId("lobby-open-join")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card-9")).toExist(),
    Scene.expect(Scene.text("Tokens")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.testId("lobby-join-code")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-join")).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 9 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});
```

```ts
// client/app/shell/surfaces.test.ts — update "renders lobby entry join surfaces with decks"
it("renders lobby entry choose destinations with decks", () => {
  Scene.scene(
    { update, view },
    Scene.with(
      authedModel(PlayRoute({ deckId: "1" }), {
        decks: {
          ...init()[0].decks,
          list: { ...init()[0].decks.list, decks: [deck], knownCommanders: { atraxa }, loading: false },
        },
        lobby: { ...initialLobbySlice(), selectedDeckId: 1 },
      }),
    ),
    Scene.expect(Scene.selector('[data-testid="lobby-entry-choose"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-host"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-open-join"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck-card"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck-card-1"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-join-code"]')).toBeAbsent(),
    Scene.expect(Scene.text("Lobby")).toExist(),
    Scene.expect(Scene.text("edh.reilley.dev")).toExist(),
    Scene.Mount.resolve(BindDeckCardFlip({ deckId: 1 }), DeckCardFlipTick()),
    Scene.Mount.resolve(BindCardArt, CardArtTick()),
  );
});
```

Update `keeps entry visible while decks load when a deck is selected` to still expect `lobby-host` + `lobby-deck-card` (loading placeholder inside Host is OK).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts`

Expected: FAIL (`lobby-open-join` / `lobby-entry-choose` missing; join-code still present)

- [ ] **Step 3: Implement choose-mode view**

In `entry()`, when deck is resolved and `model.entryMode === "choose"` (default path before Task 3 wires join UI):

```ts
function destinationCardClass(): string {
  return cn(
    "flex flex-col gap-sm rounded-hud border border-vine bg-glass-dim p-md text-left",
    "hover:bg-white/8 disabled:opacity-60",
  );
}

// choose row — sketch; match DESIGN tokens / existing helpers
h.div(
  [h.DataAttribute("testid", "lobby-entry-choose"), h.Class("grid grid-cols-2 gap-md")],
  [
    h.button(
      [
        h.Type("button"),
        h.DataAttribute("testid", "lobby-host"),
        h.Disabled(model.submitting),
        h.OnClick(RequestedLobbyHost()),
        h.Class(destinationCardClass()),
      ],
      [
        h.div(
          [h.Class("max-w-[240px]"), h.DataAttribute("testid", "lobby-deck-card")],
          [
            renderDeckCard(h, deckCardModel(deck, knownCommanders), {
              mode: "static",
              testId: `lobby-deck-card-${deck.id}`,
            }),
          ],
        ),
        h.div([h.Class("font-semibold")], ["Host a table"]),
        h.div([h.Class("text-label text-lichen")], ["with this deck"]),
      ],
    ),
    h.button(
      [
        h.Type("button"),
        h.DataAttribute("testid", "lobby-open-join"),
        h.Disabled(model.submitting),
        h.OnClick(RequestedLobbyOpenJoin()),
        h.Class(destinationCardClass()),
      ],
      [
        // code motif: dashed glass placeholder block, not deck art
        h.div(
          [
            h.Class(
              "flex aspect-[137/100] w-full items-center justify-center rounded-hud border border-dashed border-vine-dim bg-glass text-display text-lichen",
            ),
          ],
          ["#"],
        ),
        h.div([h.Class("font-semibold")], ["Join a table"]),
        h.div([h.Class("text-label text-lichen")], ["enter a code"]),
      ],
    ),
  ],
);
// Back ghost link below (reuse lobby-back)
```

Widen the lobby panel override if twin cards clip on landscape phones, e.g. `panelClass("max-w-[min(100%-2rem,640px)]")`.

Remove the old stacked Host button + always-visible join code row from `entry()`.

Do **not** change `claimSeat` / `tableLobby` in this task beyond compiling.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts app/shell/lobby/update.test.ts`

Expected: PASS (join-mode Scene may still be absent until Task 3)

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/view.ts client/app/shell/lobby/entry.test.ts \
  client/app/shell/surfaces.test.ts
git commit -m "feat(client): twin Host/Join destination cards on lobby entry"
```

---

### Task 3: Join mode — focused panel + Cancel

**Files:**
- Modify: `client/app/shell/lobby/view.ts`
- Modify: `client/app/shell/lobby/entry.test.ts`
- Modify: `client/app/shell/lobby/update.test.ts` (optional Scene via message in entry tests)

**Interfaces:**
- Consumes: `RequestedLobbyCancelJoin`, `RequestedLobbyJoin`, `ChangedLobbyCode`, `entryMode === "join"`
- Produces: focused panel with Bringing strip; twin row absent

- [ ] **Step 1: Write the failing tests**

```ts
// client/app/shell/lobby/entry.test.ts
import { RequestedLobbyCancelJoin, RequestedLobbyOpenJoin } from "./messages";

test("opening Join shows focused panel with Bringing strip and hides destinations", () => {
  const base = playLobbyModel({
    lobby: { ...initialLobbySlice(), selectedDeckId: 7 },
    decks: {
      ...init()[0].decks,
      list: {
        ...init()[0].decks.list,
        decks: [deck],
        knownCommanders: { atraxa: card({ id: "atraxa", name: "Atraxa" }) },
        loading: false,
      },
    },
  });

  const [joined] = update(base, RequestedLobbyOpenJoin());
  expect(joined.lobby.entryMode).toBe("join");

  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(joined),
    Scene.expect(Scene.testId("lobby-entry-join")).toExist(),
    Scene.expect(Scene.testId("lobby-bringing")).toExist(),
    Scene.expect(Scene.text("Superfriends")).toExist(),
    Scene.expect(Scene.testId("lobby-join-code")).toExist(),
    Scene.expect(Scene.testId("lobby-join")).toExist(),
    Scene.expect(Scene.testId("lobby-join-cancel")).toExist(),
    Scene.expect(Scene.testId("lobby-entry-choose")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-open-join")).toBeAbsent(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
  );
});

test("Cancel returns to choose and clears the table code", () => {
  const open = playLobbyModel({
    lobby: {
      ...initialLobbySlice(),
      selectedDeckId: 7,
      entryMode: "join",
      code: "ABC123",
      error: "UnknownTable",
    },
    decks: {
      ...init()[0].decks,
      list: { ...init()[0].decks.list, decks: [deck], loading: false },
    },
  });

  const [next] = update(open, RequestedLobbyCancelJoin());
  expect(next.lobby.entryMode).toBe("choose");
  expect(next.lobby.code).toBe("");
  expect(next.lobby.error).toBeNull();

  Scene.scene(
    { update, view: lobbyAppView },
    Scene.with(next),
    Scene.expect(Scene.testId("lobby-entry-choose")).toExist(),
    Scene.expect(Scene.testId("lobby-entry-join")).toBeAbsent(),
  );
});
```

Use app `update` from `main-exports` (same as other entry tests) so `RequestedLobbyOpenJoin` is handled through the real message union.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/shell/lobby/entry.test.ts`

Expected: FAIL (`lobby-entry-join` / `lobby-bringing` missing)

- [ ] **Step 3: Implement join-mode panel**

In `entry()`, branch on `model.entryMode`:

```ts
if (model.entryMode === "join") {
  return h.div(
    [h.DataAttribute("testid", "lobby-entry-join"), h.Class("flex flex-col gap-md")],
    [
      h.div(
        [
          h.DataAttribute("testid", "lobby-bringing"),
          h.Class("flex items-center gap-sm border-b border-vine-dim pb-sm"),
        ],
        [
          // small art: reuse cardArt or a tiny renderDeckCard crop; keep compact
          h.div([h.Class("w-10 shrink-0 overflow-hidden rounded-control")], [/* art_crop or glass */]),
          h.div(
            [h.Class("min-w-0")],
            [
              h.div([h.Class("text-label text-lichen")], ["Bringing"]),
              h.div([h.Class("truncate font-semibold")], [deck.name]),
            ],
          ),
        ],
      ),
      h.div([h.Class("font-semibold text-title")], ["Join a table"]),
      h.div([h.Class("text-label text-lichen")], ["Paste the code your host shared"]),
      h.label([h.For("table-code"), h.Class("sr-only")], ["Table code"]),
      h.input([
        h.Id("table-code"),
        h.DataAttribute("testid", "lobby-join-code"),
        h.Placeholder("Table code"),
        h.Value(model.code),
        h.OnInput((code) => ChangedLobbyCode({ code })),
        h.Autocomplete("off"),
        h.Spellcheck(false),
        h.Class(fieldClass("w-full")),
      ]),
      h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "lobby-join"),
          h.Disabled(model.submitting),
          h.OnClick(RequestedLobbyJoin()),
          h.Class(buttonClass("primary")),
        ],
        ["Join table"],
      ),
      h.button(
        [
          h.Type("button"),
          h.DataAttribute("testid", "lobby-join-cancel"),
          h.Disabled(model.submitting),
          h.OnClick(RequestedLobbyCancelJoin()),
          h.Class(buttonClass("ghost")),
        ],
        ["Cancel"],
      ),
      // Back ghost to `/` (lobby-back)
    ],
  );
}
```

Keep loading / empty / pick-a-deck gates **before** the choose/join branch (same as today).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts app/shell/lobby/update.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby/view.ts client/app/shell/lobby/entry.test.ts \
  client/app/shell/lobby/update.test.ts
git commit -m "feat(client): focused join panel with Bringing strip on lobby entry"
```

---

### Task 4: Light choose↔join motion + reduced-motion

**Files:**
- Modify: `client/app/shell/lobby/view.ts` (CSS classes on mode wrappers)
- Modify: `client/styles/global.css` only if a named keyframe is cleaner than Tailwind animate utilities already in the project

**Interfaces:**
- Produces: short opacity/translate on `lobby-entry-choose` / `lobby-entry-join`; no animation when `prefers-reduced-motion: reduce`

- [ ] **Step 1: Confirm available motion utilities**

Run: `rg -n "prefers-reduced-motion|animate-|@keyframes" client/styles client/app/shell --glob '*.{css,ts}' | head -40`

Prefer an existing utility. If none fits, add a tiny CSS block:

```css
@media (prefers-reduced-motion: no-preference) {
  [data-lobby-entry-motion] {
    animation: lobby-entry-swap 180ms ease-out;
  }
}

@keyframes lobby-entry-swap {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

- [ ] **Step 2: Attach motion attribute to mode wrappers**

```ts
h.DataAttribute("lobby-entry-motion", "1"),
```

on both `lobby-entry-choose` and `lobby-entry-join` roots.

No new Vitest for animation frames (same posture as deck-card FLIP Scene tests).

- [ ] **Step 3: Smoke the related tests**

Run: `cd client && bun test app/shell/lobby/entry.test.ts app/deck-card-nav.test.ts`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add client/app/shell/lobby/view.ts client/styles/global.css
git commit -m "feat(client): light lobby entry mode-swap motion"
```

---

### Task 5: Living specs + verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md` (lobby entry paragraph)
- Modify: `docs/superpowers/specs/2026-07-20-lobby-entry-ui.md` (`Status: Implemented`)

- [ ] **Step 1: Update client-shell lobby paragraph**

Replace the sentence that says the lobby renders a non-interactive commander card plus Host/Join with current behavior, for example:

> On `/play/:deckId`, the lobby entry uses `entryMode` `choose` | `join`. **Choose** shows twin destination cards: Host wraps the deck-card chrome (`lobby-deck-card` / `lobby-deck-card-{id}`) and hosts immediately; Join (`lobby-open-join`) opens **join** mode. **Join** replaces the twin row with a focused panel (`lobby-bringing`, `lobby-join-code`, `lobby-join`, `lobby-join-cancel`). Claim-seat and seated lobby still use the deck card + Ready/Start chrome as before. Malformed / not-in-library deck ids still 404.

Keep claim-seat “no select / Back” language accurate.

- [ ] **Step 2: Mark design Status Implemented**

Set `**Status:** Implemented` on the design doc header.

- [ ] **Step 3: Full client verification**

Run: `just client-check`

Expected: format + lint + typecheck + Vitest all green.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md \
  docs/superpowers/specs/2026-07-20-lobby-entry-ui.md
git commit -m "docs: record lobby host/join entry redesign as current behavior"
```

---

## Spec coverage checklist (author self-review)

| Spec requirement | Task |
|------------------|------|
| Twin Host/Join destinations, equal weight | Task 2 |
| Host immediate create | Task 1 + 2 |
| Join → focused panel | Task 3 |
| Bringing strip in join mode | Task 3 |
| Cancel → choose, clear code | Task 1 + 3 |
| Path/404/RPC unchanged | All (no wire tasks) |
| FLIP on Host deck chrome | Task 2 |
| Light motion + reduced-motion | Task 4 |
| Scene/outcome tests | Tasks 2–3 |
| Living shell spec update | Task 5 |
| Seated lobby out of scope | Explicit non-touch in Tasks 2–3 |

## Placeholder / consistency notes

- Message names are fixed: `RequestedLobbyOpenJoin`, `RequestedLobbyCancelJoin`.
- Testids are fixed in Global Constraints; do not invent alternates in later tasks.
- `lobby-join` means **submit** in join mode only; choose mode uses `lobby-open-join`.
