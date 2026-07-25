# Lobby Deck Card Path + View Transitions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Require a deck id in play path params, show that deck as a commander card on host/join (no `<select>`), and morph the Your decks tile into the lobby card with CSS View Transitions when the API is available.

**Architecture:** Foldkit routes become `/play/:deckId` and `/play/:deckId/:table`. Lobby reads `selectedDeckId` from the route, renders a shared deck-card chrome (same content as list tiles), and drops Bring/select. Internal navigations between `/` and `/play/:deckId` optionally wrap `Navigation.pushUrl` in `document.startViewTransition`, with matching `view-transition-name` on the card roots. Missing/malformed/not-in-library decks surface as not-found.

**Tech Stack:** Foldkit (Html / route / Navigation / Scene / Command), Effect Schema, Vitest, CSS View Transitions API, TypeScript.

**Spec:** [lobby-deck-card-path-and-view-transitions-design](../specs/2026-07-24-lobby-deck-card-path-and-view-transitions-design.md)  
**Current-behavior updates:** [client-shell-deck-builder-and-observability](../specs/2026-07-20-client-shell-deck-builder-and-observability.md), [lobby-table-routing-and-live-game](../specs/2026-07-20-lobby-table-routing-and-live-game.md) (path / share-link mentions)

## Global Constraints

- No `.proto`, BFF, or `DeckSummary` schema changes; host/join/ready still send deck id as today.
- No lobby `<select>`, no Bring name strip, no seat-row art, no app-wide VT on every route.
- No soft redirects for legacy `/play`, `/play/:table`, or `?deck=` — hard not-found.
- Guard-return-first; imports at top of file; exhaustive message/route matching.
- TDD: failing test → implement → pass → commit per task.
- Angular commit messages on branch `cursor/lobby-deck-card-view-transitions-f353`.
- Scene/outcome tests assert product behavior (card visible, href path, not-found), not migration/parity.
- Exact testids: `lobby-deck-card`, `lobby-deck-card-{id}`, keep `lobby-back` / `lobby-host` / `lobby-join`; remove `lobby-deck` / `lobby-bring`.
- View-transition name format: `deck-card-{id}` (CSS ident-safe; negative precon ids → `deck-card--1`).

---

## File map

| File | Responsibility |
|------|----------------|
| `client/app/routes.ts` | `PlayRoute({ deckId })`, `TableRoute({ deckId, table })`; path build/parse |
| `client/app/routes.test.ts` | Route parse/build + login-next + lobby selectedDeckId from path |
| `client/app/deck-id.ts` | `parseDeckIdParam`, `playDeckAccess`, `deckCardViewTransitionName` |
| `client/app/deck-id.test.ts` | Unit tests for those helpers |
| `client/app/update.ts` | selectedDeckId from route; host redirect path; not-in-library → NotFound; VT PushUrl |
| `client/app/view.ts` | Pass knownCommanders into lobby; not-found when access missing; fix shell Play link → `/` |
| `client/app/shell/decks/deck-card.ts` | Shared non-link / link deck card Html helper |
| `client/app/shell/lobby/view.ts` | Deck card + Back; delete select/Bring/picker |
| `client/app/shell/lobby/update.ts` | Stop defaulting to first deck when none selected; host redirect uses path deck |
| `client/app/shell/lobby/messages.ts` | Remove `ChangedLobbyDeck` if unused |
| `client/app/shell/lobby/entry.test.ts` | Card / no select / host redirect path |
| `client/app/shell/decks/list/view.ts` | Tile href `/play/{id}` + VT name on tile root |
| `client/app/shell/decks/list/story.test.ts` | Tile href assertion |
| `client/app/shell/surfaces.test.ts` | Lobby card Scene; PlayRoute/TableRoute constructors |
| `client/lib/lobby/code.ts` | Parse `/play/:deckId/:table` pasted links; stop advertising bare `/play/:table` share URL |
| `client/lib/lobby/code.test.ts` | Create if missing; paste-link cases |
| `client/app/view-transition.ts` | `shouldAnimateDeckCardNav`, `pushUrlMaybeViewTransition` |
| `client/app/view-transition.test.ts` | Helper + seam tests with mocked `startViewTransition` |
| `client/styles/global.css` | Optional VT + reduced-motion notes for `::view-transition-*` |
| Feature specs above | Document current path-param + lobby card behavior |

---

### Task 1: Play routes require deckId path param

**Files:**
- Modify: `client/app/routes.ts`
- Modify: `client/app/routes.test.ts`
- Create: `client/app/deck-id.ts`
- Create: `client/app/deck-id.test.ts`
- Modify: `client/app/smoke.test.ts` (PlayRoute constructor)
- Modify: any other compile breaks that construct `PlayRoute()` / `TableRoute({ table })` without `deckId` (fix with `rg 'PlayRoute\\(|TableRoute\\(' client/app`)

**Interfaces:**
- Consumes: Foldkit `literal` / `slash` / `string` / `r`
- Produces:
  - `PlayRoute({ deckId: string })` → `/play/:deckId`
  - `TableRoute({ deckId: string, table: string })` → `/play/:deckId/:table`
  - `parseDeckIdParam(raw: string): number | null` — `Number.isInteger` after `Number(raw)`; rejects `""`, `"abc"`, `"1.5"`
  - `deckCardViewTransitionName(deckId: number): string` — `` `deck-card-${deckId}` ``

- [ ] **Step 1: Write the failing tests**

```ts
// client/app/deck-id.test.ts
import { expect, test } from "vitest";
import { deckCardViewTransitionName, parseDeckIdParam } from "./deck-id";

test("parseDeckIdParam accepts integers including negative precons", () => {
  expect(parseDeckIdParam("7")).toBe(7);
  expect(parseDeckIdParam("-1")).toBe(-1);
  expect(parseDeckIdParam("0")).toBe(0);
});

test("parseDeckIdParam rejects non-integers", () => {
  expect(parseDeckIdParam("")).toBeNull();
  expect(parseDeckIdParam("abc")).toBeNull();
  expect(parseDeckIdParam("1.5")).toBeNull();
  expect(parseDeckIdParam("01")).toBe(1); // Number("01") === 1; integer OK
});

test("deckCardViewTransitionName is keyed by id", () => {
  expect(deckCardViewTransitionName(7)).toBe("deck-card-7");
  expect(deckCardViewTransitionName(-1)).toBe("deck-card--1");
});
```

```ts
// client/app/routes.test.ts — replace play/table expectations
test("parses play routes with required deckId", () => {
  expect(routeFromUrl(url("/play/7"))).toEqual(PlayRoute({ deckId: "7" }));
  expect(routeFromUrl(url("/play/-1/ABC123"))).toEqual(TableRoute({ deckId: "-1", table: "ABC123" }));
});

test("bare /play and legacy /play/:table are not found", () => {
  expect(routeFromUrl(url("/play"))._tag).toBe("NotFoundRoute");
  expect(routeFromUrl(url("/play/table-1"))._tag).toBe("NotFoundRoute"); // non-integer deckId normalized in Task 2, OR: parses as PlayRoute({deckId:"table-1"}) then Task 2 maps to NotFound — prefer Task 1 router only matches when next segment is present for table; bare /play is NotFound because playRouter requires deckId
});

test("builds typed play paths", () => {
  expect(routePath(PlayRoute({ deckId: "7" }))).toBe("/play/7");
  expect(routePath(TableRoute({ deckId: "7", table: "ABC123" }))).toBe("/play/7/ABC123");
});
```

Note on legacy `/play/table-1`: with `string("deckId")`, Foldkit will parse it as `PlayRoute({ deckId: "table-1" })`. Task 2’s `normalizePlayRoute` turns non-integer deckIds into `NotFoundRoute`. Keep the bare-`/play` not-found assertion in Task 1; assert legacy non-integer → NotFound in Task 2.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/deck-id.test.ts app/routes.test.ts`

Expected: FAIL (module missing / PlayRoute shape wrong)

- [ ] **Step 3: Minimal implementation**

```ts
// client/app/deck-id.ts
export function parseDeckIdParam(raw: string): number | null {
  if (raw.trim() === "") return null;
  const id = Number(raw);
  if (!Number.isInteger(id)) return null;
  return id;
}

export function deckCardViewTransitionName(deckId: number): string {
  return `deck-card-${deckId}`;
}
```

```ts
// client/app/routes.ts — replace Play/Table routers
export const PlayRoute = r("PlayRoute", { deckId: S.String });
export const TableRoute = r("TableRoute", { deckId: S.String, table: S.String });

const playRouter = pipe(literal("play"), slash(string("deckId")), mapTo(PlayRoute));
const tableRouter = pipe(
  literal("play"),
  slash(string("deckId")),
  slash(string("table")),
  mapTo(TableRoute),
);
// oneOf: put tableRouter BEFORE playRouter so /play/a/b matches table
const appRouter = oneOf(homeRouter, loginRouter, newDeckRouter, deckRouter, tableRouter, playRouter);
// routePath arms:
PlayRoute: ({ deckId }) => playRouter({ deckId }),
TableRoute: ({ deckId, table }) => tableRouter({ deckId, table }),
```

Update `smoke.test.ts` and every `PlayRoute()` / `TableRoute({ table })` call site to include `deckId`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/deck-id.test.ts app/routes.test.ts app/smoke.test.ts`

Expected: PASS (adjust routes.test login-next test in Task 2 if still on `?deck=`)

- [ ] **Step 5: Commit**

```bash
git add client/app/deck-id.ts client/app/deck-id.test.ts client/app/routes.ts client/app/routes.test.ts client/app/smoke.test.ts $(rg -l 'PlayRoute\(|TableRoute\(' client/app)
git commit -m "feat(client): require deckId path param on play routes"
```

---

### Task 2: selectedDeckId from route + not-found normalization + host redirect

**Files:**
- Modify: `client/app/update.ts`
- Modify: `client/app/routes.ts` (export `normalizeAppRoute` if kept next to parsers) **or** keep normalize in `deck-id.ts` / `update.ts`
- Modify: `client/app/routes.test.ts`
- Modify: `client/app/shell/lobby/entry.test.ts` (host redirect expectation)
- Modify: `client/app/shell/lobby/update.ts` (`selectedDeckId` must not fall back to `deckIds[0]`)
- Modify: `client/app/view.ts` (shell Play link → `routePath(HomeRoute())`)

**Interfaces:**
- Consumes: `parseDeckIdParam`, new route shapes
- Produces:
  - `normalizeAppRoute(route, path): AppRoute` — if Play/Table and `parseDeckIdParam(deckId) == null` → `NotFoundRoute({ path })`
  - `selectedDeckId` on lobby enter = `parseDeckIdParam(route.deckId)`
  - Host redirect path = `routePath(TableRoute({ deckId: String(selectedDeckId), table }))` with **no** `?deck=`
  - Remove `deckFromCurrentPath`

- [ ] **Step 1: Write the failing tests**

```ts
// client/app/routes.test.ts
test("non-integer play deckId becomes NotFound after normalize", () => {
  const raw = routeFromUrl(url("/play/table-1"));
  // After UrlChanged/init normalize:
  const [base] = init(url("/play/table-1"));
  expect(base.route._tag).toBe("NotFoundRoute");
});

test("PlayRoute /play/-1 sets lobby.selectedDeckId to -1", () => {
  const [base] = init(url("/play/-1"));
  const [model] = update(base, ReceivedMe({ me: { id: 1, email: "a@b.c", username: "alice" } }));
  expect(model.route).toEqual(PlayRoute({ deckId: "-1" }));
  expect(model.lobby.selectedDeckId).toBe(-1);
});

test("redirects unsigned protected play routes with path deck", () => {
  const [model] = init(url("/play/7"));
  const redirect = {
    name: "Redirect",
    args: { path: "/login?next=%2Fplay%2F7" },
    effect: Effect.succeed(NavigationCompleted()),
  };
  Story.story(
    update,
    Story.with(model),
    Story.message(ReceivedMe({ me: null })),
    Story.Command.expectExact(redirect),
    Story.Command.resolve(redirect, NavigationCompleted()),
  );
});
```

```ts
// client/app/shell/lobby/entry.test.ts — replace host redirect test
test("host redirect uses /play/:deckId/:table", () => {
  // ... same setup with PlayRoute({ deckId: "7" }), selectedDeckId: 7
  expect(redirect?.args?.path).toBe("/play/7/XYZ789");
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/routes.test.ts app/shell/lobby/entry.test.ts`

Expected: FAIL on selectedDeckId / redirect path / NotFound

- [ ] **Step 3: Minimal implementation**

In `update.ts`:

```ts
function normalizeAppRoute(route: AppRoute, path: string): AppRoute {
  if (route._tag === "PlayRoute" || route._tag === "TableRoute") {
    if (parseDeckIdParam(route.deckId) == null) return NotFoundRoute({ path });
  }
  return route;
}

// UrlChanged / init path:
route: normalizeAppRoute(routeFromUrl(url), pathWithSearch(url)),

// PlayRoute enter:
lobby: enterLobby(model.lobby, {
  tableId: null,
  selectedDeckId: parseDeckIdParam(model.route.deckId),
}),

// TableRoute enter:
lobby: enterLobby(model.lobby, {
  tableId: model.route.table,
  selectedDeckId: parseDeckIdParam(model.route.deckId),
}),

// foldLobby redirect:
Redirect({
  path: routePath(
    TableRoute({
      deckId: String(lobby.selectedDeckId),
      table: lobby.tableId,
    }),
  ),
}),
```

Only emit redirect when `lobby.selectedDeckId != null` (always true for valid play routes).

In `lobby/update.ts`, change `selectedDeckId(model, deckIds)` to return `model.selectedDeckId` only — **do not** default to `deckIds[0]`. Host/Join without a deck id should error (should be unreachable once routes require deck).

In `view.ts` shell nav, change Play href from `routePath(PlayRoute())` to `routePath(HomeRoute())` (or remove Play if redundant with brand home — prefer Home).

Delete `deckFromCurrentPath`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/routes.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/update.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/update.ts client/app/view.ts client/app/routes.test.ts client/app/shell/lobby/update.ts client/app/shell/lobby/entry.test.ts
git commit -m "feat(client): bind lobby deck from play path and drop ?deck="
```

---

### Task 3: Shared deck card helper

**Files:**
- Create: `client/app/shell/decks/deck-card.ts`
- Create: `client/app/shell/decks/deck-card.test.ts` (Scene or pure Html assertions via Scene on a tiny view)

**Interfaces:**
- Consumes: `cardArt` from `client/lib/ui/card-art.ts`, `identityPipCodes` from `list/visible.ts`, `manaFontClass`, `listRowClass` / glass chrome classes from list view, `deckCardViewTransitionName`
- Produces:

```ts
export type DeckCardModel = {
  id: number;
  name: string;
  commander: string;
  commanderName: string;
  print: string; // "" → placeholder
  colorIdentity: readonly number[];
};

export function renderDeckCard<Msg>(
  h: ReturnType<typeof html<Msg>>,
  card: DeckCardModel,
  opts: {
    mode: "link" | "static";
    href?: string; // required when mode === "link"
    testId: string; // e.g. deck-tile-7 or lobby-deck-card-7
  },
): Html;
```

- Root element (`a` or `div`) sets inline style / attribute for view transition:
  - Prefer `h.Style(\`view-transition-name: ${deckCardViewTransitionName(card.id)}\`)` if Foldkit Style accepts it; otherwise a wrapper `div` with that style around the chrome.
- Contents match list tile: art_crop (or glass placeholder), name, Precon chip if `id < 0`, commander name, pips.
- `mode: "link"` → `h.a` with href; `mode: "static"` → non-interactive `h.div` (no button role).

- [ ] **Step 1: Write the failing Scene test**

```ts
// client/app/shell/decks/deck-card.test.ts
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { renderDeckCard } from "./deck-card";

const h = html<never>();
const view = () =>
  renderDeckCard(
    h,
    {
      id: 7,
      name: "Superfriends",
      commander: "atraxa",
      commanderName: "Atraxa, Praetors' Voice",
      print: "atraxa-print",
      colorIdentity: [0, 1, 2, 3, 4],
    },
    { mode: "static", testId: "lobby-deck-card-7" },
  );

test("static deck card exposes testid, name, commander, precon chip absent", () => {
  Scene.scene(
    { update: (m) => [m, []], view: () => ({ title: "t", body: view() }) },
    Scene.with(null),
    Scene.expect(Scene.testId("lobby-deck-card-7")).toExist(),
    Scene.expect(Scene.text("Superfriends")).toExist(),
    Scene.expect(Scene.text("Atraxa, Praetors' Voice")).toExist(),
    Scene.expect(Scene.text("Precon")).not.toExist(),
  );
});
```

(Adapt Scene harness to how other pure-view tests are structured in this repo if `view: () => Document` differs — mirror `entry.test.ts` pattern: pass Html through a thin Document wrapper like app `view`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cd client && bun test app/shell/decks/deck-card.test.ts`

Expected: FAIL module not found

- [ ] **Step 3: Implement `renderDeckCard`**

Copy chrome structure from `shell/decks/list/view.ts` tile body (art / name / pips / precon). Do not include context-menu mount in the helper.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd client && bun test app/shell/decks/deck-card.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/deck-card.ts client/app/shell/decks/deck-card.test.ts
git commit -m "feat(client): add shared deck card render helper"
```

---

### Task 4: Lobby shows deck card; remove select/Bring; not-in-library 404

**Files:**
- Modify: `client/app/shell/lobby/view.ts`
- Modify: `client/app/view.ts` (pass `knownCommanders` + loading; render not-found when access missing)
- Modify: `client/app/deck-id.ts` — add `playDeckAccess`
- Modify: `client/app/deck-id.test.ts`
- Modify: `client/app/update.ts` — on `ReceivedDecks`, if Play/Table and id not in library → set `route` to `NotFoundRoute({ path: currentPath })`
- Modify: `client/app/shell/lobby/entry.test.ts`
- Modify: `client/app/shell/surfaces.test.ts`
- Modify: `client/app/shell/lobby/messages.ts` + app message wiring — remove `ChangedLobbyDeck` if unused
- Modify: `client/app/messages.ts`, `client/app/update.ts` arms

**Interfaces:**
- Consumes: `renderDeckCard`, `parseDeckIdParam`, decks list + knownCommanders
- Produces:
  - `playDeckAccess(deckId: number | null, decks: readonly { id: number }[], loading: boolean): "loading" | "ok" | "missing"`
  - Lobby view signature includes `knownCommanders` and uses card instead of select/Bring
  - `data-testid="lobby-deck-card"` on wrapper **and** `lobby-deck-card-{id}` on card root

```ts
export function playDeckAccess(
  deckId: number | null,
  decks: ReadonlyArray<{ id: number }>,
  loading: boolean,
): "loading" | "ok" | "missing" {
  if (deckId == null) return "missing";
  if (loading && decks.length === 0) return "loading";
  if (decks.some((d) => d.id === deckId)) return "ok";
  if (loading) return "loading";
  return "missing";
}
```

- [ ] **Step 1: Write failing tests**

```ts
// deck-id.test.ts
test("playDeckAccess", () => {
  expect(playDeckAccess(7, [], true)).toBe("loading");
  expect(playDeckAccess(7, [{ id: 7 }], false)).toBe("ok");
  expect(playDeckAccess(7, [{ id: 1 }], false)).toBe("missing");
  expect(playDeckAccess(null, [{ id: 1 }], false)).toBe("missing");
});
```

```ts
// entry.test.ts — replace select/Bring tests
test("entry shows deck card and Back, never a select", () => {
  Scene.scene(
    { update, view: lobbyAppView }, // lobbyAppView must pass knownCommanders
    Scene.with(
      playLobbyModel({
        route: PlayRoute({ deckId: "9" }),
        lobby: { ...initialLobbySlice(), selectedDeckId: 9 },
        decks: {
          ...init()[0].decks,
          list: {
            ...init()[0].decks.list,
            decks: [deck, other],
            knownCommanders: { rhys: { /* minimal CatalogCard */ } },
            loading: false,
          },
        },
      }),
    ),
    Scene.expect(Scene.testId("lobby-deck-card")).toExist(),
    Scene.expect(Scene.testId("lobby-deck-card-9")).toExist(),
    Scene.expect(Scene.testId("lobby-back")).toExist(),
    Scene.expect(Scene.selector('[data-testid="lobby-deck"]')).toBeAbsent(),
    Scene.expect(Scene.selector('[data-testid="lobby-bring"]')).toBeAbsent(),
  );
});

test("unknown deck after load shows not-found, not lobby", () => {
  // Prefer full app `view` from main-exports:
  // PlayRoute deckId 99, decks loaded without 99 → text "Not found"
});
```

Update `surfaces.test.ts` lobby entry case: `PlayRoute({ deckId: "1" })`, `selectedDeckId: 1`, expect `lobby-deck-card`, join controls; table case includes `deckId` in `TableRoute`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd client && bun test app/deck-id.test.ts app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts`

Expected: FAIL on card testids / not-found

- [ ] **Step 3: Implement lobby UI + access gating**

`lobby/view.ts`:
- Delete `deckPicker`, `bringAndBack`, `ChangedLobbyDeck` import.
- Add `deckCardAndBack(model, decks, knownCommanders)` using `renderDeckCard` + Back link to `HomeRoute`.
- While `playDeckAccess === "loading"`, show “Loading decks…” in the card area (still show Host/Join only when access is ok — or show Host disabled during load; match today’s “keep entry visible while loading” by showing Host/Join with loading card placeholder).
- Claim-seat: card + Claim button only (no picker branch).

`view.ts` `routeBody` for Play/Table when not active game:

```ts
const deckId = parseDeckIdParam(model.route.deckId);
const access = playDeckAccess(deckId, model.decks.list.decks, model.decks.list.loading);
if (access === "missing") {
  return shell(model, "Not found", `No Foldkit route for ${model.currentPath}.`);
}
return lobbyView(
  model.lobby,
  model.decks.list.decks,
  model.decks.list.loading,
  model.decks.list.knownCommanders,
  model.apiVersion,
);
```

Also fold `ReceivedDecks` in `update.ts` so `model.route` becomes `NotFoundRoute` when missing (keeps model aligned with view). Either view-only or model+view is fine if Scene asserts “Not found” text; prefer updating `model.route` for consistency.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd client && bun test app/shell/lobby app/shell/surfaces.test.ts app/deck-id.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/lobby client/app/view.ts client/app/update.ts client/app/deck-id.ts client/app/deck-id.test.ts client/app/messages.ts client/app/shell/surfaces.test.ts
git commit -m "feat(client): show lobby deck card and 404 unknown play decks"
```

---

### Task 5: Your decks tile → `/play/:deckId` + shared card + VT name

**Files:**
- Modify: `client/app/shell/decks/list/view.ts`
- Modify: `client/app/shell/decks/list/story.test.ts`
- Modify: `client/app/shell/surfaces.test.ts` if it asserts tile href

**Interfaces:**
- Consumes: `renderDeckCard`, `routePath(PlayRoute({ deckId: String(deck.id) }))`
- Produces: tile is `renderDeckCard(..., { mode: "link", href, testId: \`deck-tile-${id}\` })` plus existing context-menu `OnMount` on the link root (keep BindDeckListContextMenu on the card root)

- [ ] **Step 1: Write failing test**

```ts
// story.test.ts — update existing href test
Scene.expect(Scene.selector('[data-testid="deck-tile-1"][href="/play/1"]')).toExist(),
```

- [ ] **Step 2: Run to verify fail**

Run: `cd client && bun test app/shell/decks/list/story.test.ts`

Expected: FAIL on `?deck=` vs `/play/1`

- [ ] **Step 3: Implement list tiles via `renderDeckCard`**

Replace inline tile markup with helper; pass print/commanderName/pips inputs from existing `commanderPrint` / `knownCommanders` lookups. Preserve right-click mount.

- [ ] **Step 4: Run to verify pass**

Run: `cd client && bun test app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/shell/decks/list/view.ts client/app/shell/decks/list/story.test.ts
git commit -m "feat(client): link deck tiles to /play/:deckId with shared card"
```

---

### Task 6: View transition navigation seam + paste-link parse

**Files:**
- Create: `client/app/view-transition.ts`
- Create: `client/app/view-transition.test.ts`
- Modify: `client/app/update.ts` (`UrlRequested` Internal → VT-aware push)
- Modify: `client/lib/lobby/code.ts`
- Create or modify: `client/lib/lobby/code.test.ts`
- Modify: `client/styles/global.css` (reduced-motion for view transitions)

**Interfaces:**
- Consumes: `document.startViewTransition`, `matchMedia('(prefers-reduced-motion: reduce)')`, Foldkit `Navigation.pushUrl`
- Produces:

```ts
export function shouldAnimateDeckCardNav(fromPathname: string, toPathname: string): boolean;

export function pushUrlMaybeViewTransition(
  url: string,
  fromPathname: string,
  opts?: {
    startViewTransition?: typeof document.startViewTransition;
    prefersReducedMotion?: boolean;
    pushUrl?: (url: string) => Effect.Effect<void>;
  },
): Effect.Effect<void>;
```

`shouldAnimateDeckCardNav` is true only for:
- `/` ↔ `/play/:deckId` (exactly one path segment after play; not table routes)
- Use pathname only (ignore search/hash)

`pushUrlMaybeViewTransition`:
- If `!shouldAnimate` OR reduced motion OR no `startViewTransition` → `pushUrl(url)`
- Else call `startViewTransition(() => { void Effect.runPromise(pushUrl(url)); })` and still complete the Effect (do not block forever on `finished` if that flakes tests — prefer fire-and-forget VT around push, returning after push completes)

Wire in `UrlRequested` Internal arm:

```ts
Internal: ({ url }) => [
  model,
  [
    Command.define(...)( // or reuse PushUrl with richer effect
      pushUrlMaybeViewTransition(urlToString(url), /* pathname from model.currentPath */, {
        pushUrl: Navigation.pushUrl,
      }),
    ),
  ],
],
```

Prefer extending the existing `PushUrl` command effect to accept optional fromPathname from the closure in `UrlRequested` rather than a second command name.

`parseTableCode`: also match `/play/:deckId/:table` and return the **table** segment. `lobbyShareLink`: change to return table code only or deprecate — if unused, delete or make it return `${origin}/` with a comment that invites use the code; simplest: update `lobbyShareLink` to throw/remove if unused (rg shows only definition) — **delete `lobbyShareLink`** if unused, or rewrite to not emit `/play/:table`.

- [ ] **Step 1: Write failing tests**

```ts
// view-transition.test.ts
test("shouldAnimateDeckCardNav only for home ↔ play deck entry", () => {
  expect(shouldAnimateDeckCardNav("/", "/play/7")).toBe(true);
  expect(shouldAnimateDeckCardNav("/play/7", "/")).toBe(true);
  expect(shouldAnimateDeckCardNav("/", "/play/7/ABC")).toBe(false);
  expect(shouldAnimateDeckCardNav("/decks/1", "/play/7")).toBe(false);
  expect(shouldAnimateDeckCardNav("/play/7", "/play/8")).toBe(false);
});

test("pushUrlMaybeViewTransition uses startViewTransition when animating", async () => {
  let started = false;
  const pushUrl = () => Effect.sync(() => undefined);
  await Effect.runPromise(
    pushUrlMaybeViewTransition("/play/7", "/", {
      prefersReducedMotion: false,
      startViewTransition: (cb) => {
        started = true;
        cb();
        return { finished: Promise.resolve(), ready: Promise.resolve(), updateCallbackDone: Promise.resolve(), skipTransition() {} } as ViewTransition;
      },
      pushUrl,
    }),
  );
  expect(started).toBe(true);
});

test("skips VT when reduced motion", async () => {
  let started = false;
  await Effect.runPromise(
    pushUrlMaybeViewTransition("/play/7", "/", {
      prefersReducedMotion: true,
      startViewTransition: (cb) => {
        started = true;
        cb();
        return { finished: Promise.resolve(), ready: Promise.resolve(), updateCallbackDone: Promise.resolve(), skipTransition() {} } as ViewTransition;
      },
      pushUrl: () => Effect.void,
    }),
  );
  expect(started).toBe(false);
});
```

```ts
// lib/lobby/code.test.ts
test("parseTableCode reads table from /play/:deckId/:table", () => {
  expect(parseTableCode("http://localhost/play/7/ABC123")).toBe("ABC123");
});
test("parseTableCode still accepts bare codes", () => {
  expect(parseTableCode("abc123")).toBe("ABC123");
});
```

- [ ] **Step 2: Run to verify fail**

Run: `cd client && bun test app/view-transition.test.ts lib/lobby/code.test.ts`

Expected: FAIL

- [ ] **Step 3: Implement seam + parse + CSS**

```css
/* client/styles/global.css — inside prefers-reduced-motion block, add: */
::view-transition-group(*),
::view-transition-old(*),
::view-transition-new(*) {
  animation-duration: 0.01ms !important;
}
```

(Only if needed; existing `*` animation override may already cover pseudo-elements — verify; if not, add.)

- [ ] **Step 4: Run to verify pass**

Run: `cd client && bun test app/view-transition.test.ts lib/lobby/code.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add client/app/view-transition.ts client/app/view-transition.test.ts client/app/update.ts client/lib/lobby/code.ts client/lib/lobby/code.test.ts client/styles/global.css
git commit -m "feat(client): view-transition home tile to lobby deck card"
```

---

### Task 7: Feature spec updates + verification

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md`
- Modify: `docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md` (share link `/play/{table_id}` → table code; path shapes)
- Modify: `docs/superpowers/specs/2026-07-24-lobby-deck-card-path-and-view-transitions-design.md` — Status: Implemented
- Modify: tile-chooser / layout-polish design non-goals — one-line pointer that lobby Bring/select is superseded by the lobby-deck-card design

**Docs content to write (current behavior):**
- Routes table: `/play/:deckId`, `/play/:deckId/:table`
- Query params: `?next=` only for login; no `?deck=`
- Lobby: deck card + Back; no select/Bring; unknown deck → not found
- Your decks tile href `/play/{id}`; view-transition-name `deck-card-{id}`
- Invite via copied table code

- [ ] **Step 1: Update specs to match shipped behavior** (no separate failing test)

- [ ] **Step 2: Run client verification**

Run: `cd client && bun test app/routes.test.ts app/deck-id.test.ts app/view-transition.test.ts app/shell/lobby app/shell/decks/list/story.test.ts app/shell/decks/deck-card.test.ts app/shell/surfaces.test.ts`

Expected: PASS

Optional fuller: `just client-check` if environment is ready.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs
git commit -m "docs: record lobby deck card path and view transitions behavior"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| `/play/:deckId`, `/play/:deckId/:table` | 1 |
| Bare/legacy/`?deck=` hard cut; non-integer 404 | 1–2 |
| selectedDeckId from path; host keeps deck in path | 2 |
| Lobby card same chrome as tile; no select/Bring | 3–4 |
| Back to change decks | 4 |
| Not-in-library 404 after load; loading gate | 4 |
| Seat rows text-only | 4 (unchanged) |
| Tile href `/play/{id}` | 5 |
| `view-transition-name` + `startViewTransition` home↔entry; reduced motion; no polyfill | 5–6 |
| Invite via table code; parse new paste URLs | 6 |
| Feature spec updates | 7 |
| No wire/BFF changes | all |

No TBD placeholders remain. Types (`PlayRoute.deckId: string`, `parseDeckIdParam → number | null`, `renderDeckCard` opts) are consistent across tasks.
