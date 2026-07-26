# Foldkit submodels, domain layout, play routes, and installable PWA (design)

**Status:** Design input pending user review (2026-07-26).
**Surfaces (update at implement time):** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md), [lobby-entry-ui](2026-07-20-lobby-entry-ui.md), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md), [deck-list-and-builder](2026-07-20-deck-list-and-builder.md), [board-composition](2026-07-20-board-composition.md); favicon / icon notes in [favicon-dragon-silhouette-design](2026-07-25-favicon-dragon-silhouette-design.md).
**Upstream patterns:** [Foldkit project organization](https://foldkit.dev/patterns/project-organization), [Informing submodels](https://foldkit.dev/patterns/informing-submodels), [Submodels](https://foldkit.dev/patterns/submodels).

---

## Problem Statement

The client is a single Foldkit SPA with feature folders, but it drifts from Foldkit’s documented organization and submodel protocol: child messages are flattened into the parent `Message` union, route entry uses ad hoc helpers instead of `inform*`, there are no `index.ts` namespace re-exports, and shared code lives in `client/lib/` rather than an app-local `domain/`. Play URLs still require a deck id path segment even after the seat’s deck is stored in `mtgfr_web` on join/seed. Progressive Web App installability (manifest / service worker / offline) is explicitly out of scope in shell-routes today; friends want a native-feeling install without offline play.

## Goal

1. Align the client with Foldkit project-organization and informing-submodels conventions (`Got*Message`, `Command.mapMessages`, `inform*`, feature `index.ts` namespaces, `domain/`).
2. Reshape play routes so deck picking is on `/play`, pregame lobby keeps deck+table in the path, and the live game is table-only.
3. Ship an **installable-only** PWA via `vite-plugin-pwa` (browser-native install; network-only service worker; no precache / offline mode).

## Locked decisions

| Decision | Choice |
|---|---|
| Micro-frontends / Module Federation | **No** — one Foldkit reactor, one Vite/Nitro app |
| Submodel protocol | **Full Foldkit alignment (C)** — `Got*Message` wrappers, `Command.mapMessages`, `inform*` helpers that run child `update` |
| Shared code location | Rename/move `client/lib/` → `client/app/domain/` (Foldkit `domain/` sibling to features); keep `client/server/` and `client/styles/` outside the app TEA tree |
| Feature namespaces | Add `index.ts` re-exports per feature (`export * as Lobby from './lobby'`, etc.) |
| `page/` folder rename | **Not required** — keep product folders `shell/`, `board/`, `game/` as the feature roots (equivalent role to docs’ `page/`) |
| Play entry | `/play` — deck picker + Host/Join |
| Pregame (seated lobby) | `/play/:deckId/:tableId` |
| Live game | `/play/:tableId` only (after start); deck already on seats in DB / seed |
| Legacy play URLs | Hard cut Not found: bare `/play/:deckId`, `?deck=`, and other obsolete shapes |
| PWA ambition | Installable only (A) — not offline |
| PWA tooling | **Approach 2** — `vite-plugin-pwa` with **injectManifest** (or equivalent) and a hand-authored **network-only** SW (no precache, no runtime caching) |
| Install UX | Browser-native only — no in-app Install button / `beforeinstallprompt` chrome |
| Route lazy-load / workspace packages | Out of scope for this design |

## Approaches considered

### Client modularity

1. **Docs-only + light seam fixes** — Rejected; leaves flat `Message` union and ad hoc route helpers.
2. **`inform*` without `Got*Message`** — Partial alignment; still fights Submodels docs.
3. **Full Foldkit protocol + `domain/` layout (chosen)** — Matches upstream patterns; larger migration, phased with PWA and routes.

### PWA

1. Hand-rolled manifest + minimal SW — Works, more lifecycle boilerplate.
2. **`vite-plugin-pwa` network-only (chosen)** — Standard Vite path; config must forbid accidental precache.
3. Workbox precache / offline shell — Rejected for v1 (authoritative engine is online-only).

### Play routes

1. Drop deck from all play URLs immediately (`/play` + `/play/:table`) — Loses explicit pregame “bringing this deck” in the path.
2. **Picker on `/play`; pregame `/play/:deckId/:tableId`; in-game `/play/:tableId` (chosen)** — Deck id remains a required path param while claiming/readying; stripped once seats are seeded and the board mounts.
3. Keep current `/play/:deckId` and `/play/:deckId/:table` forever — Rejected; table share links should not require a deck segment after start.

## Design

### Wave plan

Implement as **three waves** (separate PRs or stacked commits; each wave updates living surface specs in the same change):

| Wave | Deliverable |
|---|---|
| **1** | Foldkit submodel protocol + `client/app/domain/` move + feature `index.ts` namespaces |
| **2** | Play route reshape (picker / pregame / in-game) + lobby/parse/share updates |
| **3** | Installable PWA (`vite-plugin-pwa`, icons, network-only SW, `entry` registration) |

Waves 1–2 may merge if review prefers one PR; Wave 3 must not enable precaching.

---

### Wave 1 — Foldkit submodels and `domain/`

#### App tree (target)

```
client/app/
├── entry.ts
├── init.ts
├── model.ts
├── messages.ts          # parent Message = app-level ∪ Got* wrappers only
├── routes.ts
├── update.ts
├── view.ts
├── subscriptions.ts
├── resources.ts
├── domain/              # was client/lib/
│   ├── index.ts         # optional barrel; prefer named domain modules
│   ├── wire/
│   ├── lobby/           # pure lobby helpers / types / client HTTP (not Foldkit UI)
│   ├── ui/
│   └── …
├── shell/
│   ├── auth/
│   │   ├── index.ts     # export * as Model/Message; export update, view, inform*
│   │   ├── model.ts     # (or submodel.ts renamed to model.ts)
│   │   ├── message.ts   # (messages.ts → message.ts when touching the folder)
│   │   ├── update.ts
│   │   ├── view.ts
│   │   └── inform.ts    # informRouteChanged / informSessionReady as needed
│   ├── decks/…
│   ├── lobby/…
│   ├── leaderboard/…
│   └── account-chrome/…
├── board/
│   └── index.ts + model/message/update/view/inform*
└── game/
    └── fold, stream, messages (parent-owned stream frames OK if documented)
```

Path alias: update `tsconfig` / Vite so `~/*` (or a dedicated alias) resolves to `client/app/domain/*`. Fix all imports in the same wave. Nitro server code that imported `../../lib/...` updates to the new path.

#### Parent `Message` protocol

Parent union includes **only**:

- App-owned tags: `Booted`, `UrlChanged`, `UrlRequested`, `NavigationCompleted`, portrait gate, API meta, gravatar hash, etc.
- Wrappers: `GotAuthMessage`, `GotDeckListMessage`, `GotDeckBuilderMessage`, `GotLeaderboardMessage`, `GotLobbyMessage`, `GotBoardMessage`, `GotGameMessage` (stream snapshot/delta/status fold through a game submodel `update`, not flattened parent tags).

**No** re-export of every child constructor into the parent message module for view dispatch; views wrap with `message => GotXMessage({ message })`.

#### Parent `update`

- `GotXMessage`: `[next, cmds] = X.update(...);` then `Command.mapMessages(cmds, m => GotXMessage({ message: m }))`.
- `UrlChanged` / session-ready entry: resolve `AppRoute`, then call the relevant feature’s `informRouteChanged(model, childRouteSlice)` (and/or `informSessionReady`) instead of `routeEntry` calling `loadDeckList` / `enterBuilder` / `enterLobby` as direct state mutators.
- Each `inform*` helper is `update(model, ChangedRoute({ route }))` (or equivalent internal message) so the child owns the transition.
- Replace `enterLobby` direct resets with lobby `ChangedRoute` / `informRouteChanged` that returns `[LobbyModel, Command[]]`.
- Board `h.submodel` `toParentMessage` wraps into `GotBoardMessage` (stop identity passthrough).

#### OutMessages

When a child needs parent-owned navigation or session replacement, use Foldkit’s OutMessage / third-tuple pattern (or a small parent-interpreted result from `inform*`) rather than children emitting raw navigation commands inconsistently. Lobby → create `GameSlice` / redirect after start remains a **parent** responsibility after `GotLobbyMessage` or an explicit lobby OutMessage.

#### Cold load

Align with Foldkit cold-load guidance: after session resolves for a protected route, run the same `inform*` path as `UrlChanged` so the initial URL seeds child state and boot commands (not only `FetchMe` then a later ad hoc entry).

---

### Wave 2 — Play routes

#### Route table

| Path | Route tag | Surface |
|---|---|---|
| `/play` | `PlayRoute` | Deck picker + Host/Join entry |
| `/play/:deckId/:tableId` | `PregameTableRoute` | Seated pregame lobby |
| `/play/:tableId` | `GameTableRoute` | Live board (after start) |
| Legacy `/play/:numericDeckId` only, `?deck=`, obsolete shapes | `NotFoundRoute` | Hard cut |

Router `oneOf` order: two-segment pregame **before** one-segment game. `normalizeAppRoute` rejects a single-segment play path that is clearly a legacy numeric deck id (not a table id). Table ids remain unguessable hex (existing lobby behavior).

#### Behavior

1. **`/play`:** Load deck list (via deck-list `inform*`). Player selects a library deck (`selectedDeckId` in lobby model). Host creates a table or Join submits code + `deck_id` to BFF (unchanged join payload). On success, navigate to `/play/:deckId/:tableId`.
2. **`/play/:deckId/:tableId`:** Pregame lobby as today (claim/ready/start, poll, watch note). Path deck id is the local player’s bringing deck; seat row still persists `deck_id` in `mtgfr_web` on join.
3. **Start → game:** On lobby `started`, parent navigates to `/play/:tableId` (strip deck segment), activates `GameSlice`, board mounts, game stream keys off table id only.
4. **Share / parse:** `parseTableCode` accepts `/play/:deckId/:tableId` and `/play/:tableId`, plus bare codes. Prefer sharing bare codes or post-start `/play/:tableId` for in-game; pregame invites may still paste two-segment URLs (joiner’s own deck comes from `/play` picker before join, not from the host’s path deck segment).
5. **Home “Play”:** Navigates to `/play` with optional in-memory preselect of that deck for the picker; refresh without selection shows the picker empty/unselected (no deck id in the URL on entry).

#### Spec / product copy updates (implement wave)

- shell-routes route table and guards.
- lobby-entry-ui path-param behavior and FLIP/home morph targets.
- lobby-table-routing “redirect to `/play/{deck}/{table}`” → pregame two-segment; in-game table-only.
- deck-list-and-builder play CTA target.

---

### Wave 3 — Installable PWA

#### Manifest (plugin-generated)

- `name` / `short_name`: `edh.reilley.dev`
- `start_url`: `/`
- `display`: `standalone`
- `background_color` / `theme_color`: forest-floor `#0B1310`
- Icons: 192 + 512, `any` + `maskable`, derived from the dragon-on-disc favicon
- `scope` / `id`: `/`
- No screenshots, share_target, or related_applications

#### HTML

- `theme-color`, `apple-touch-icon` (180) in `index.html`
- Keep existing favicon links and landscape / safe-area behavior

#### `vite-plugin-pwa`

- Register in `client/vite.config.ts`
- **injectManifest** (preferred) with a checked-in SW module whose `fetch` handler is solely `event.respondWith(fetch(event.request))`
- **No** Workbox precache manifest / `globPatterns` asset caching / runtimeCaching routes / offline fallback document
- `devOptions.enabled: false` — verify via production build + preview
- Comment in config: do not add precache without a new product decision

#### Registration

- Once from `client/app/entry.ts` (or `client/app/pwa.ts` imported there)
- Not a Foldkit `Message`, not board/lobby state

#### Shell-routes

- Move “PWA / service worker / offline mode” out of Out of Scope for this **installable-only** slice; keep **offline mode** / precache explicitly out of scope.

---

## Testing Decisions

### Wave 1

- Parent `Message` exhaustiveness: one `Got*` arm per child; `Command.mapMessages` covered by existing story/update tests updated for wrappers.
- Scene tests: dispatch wrapped messages; board `toParentMessage` wrapping.
- Import/path: `tsc` + vitest green after `lib` → `domain` move.
- Route/session cold-load: protected deep link runs `inform*` after session (extend shell/lobby/deck tests).

### Wave 2

- `routes.test.ts`: `/play`, `/play/:deckId/:tableId`, `/play/:tableId`, reject legacy numeric-only `/play/:deckId`.
- `parseTableCode` tests for both path shapes.
- Lobby entry/scene: picker on `/play`; start navigates to table-only path; board stream still keys on table id.
- Update shell surfaces / lobby entry Scene assertions for new paths.

### Wave 3

- Fixture/static asserts: manifest fields; `theme-color` / apple-touch link present.
- SW: no Cache API writes in the fetch path; no precache list in build output (or empty).
- Manual: Chromium Application panel shows manifest + controlling SW after preview build.
- No Scene test for install UI (none shipped).

### Verify bar

- Each wave: `just client-check` before claiming done.
- Prefer `just check` before merge when server/docs-only touch is mixed.

## Out of Scope

- Micro-frontends, Module Federation, independently deployed shells.
- Workspace packages (`@mtgfr/board`, etc.) solely for boundary enforcement.
- Route-level code splitting / lazy board chunk (may be a later design).
- Offline play, precached app shell, catalog dump for offline builder.
- In-app Install promo / `beforeinstallprompt` UI.
- Renaming Nitro `client/server/` or design-token pipeline roots.
- Splitting `updateBoard` internals beyond message-protocol wrapping.
- OAuth, SSR of the board, SEO/sitemaps.

## Further Notes

- This file is **design input**. Living module specs must be updated in the same implementation change as behavior ships (AGENTS.md feature-spec gate).
- Architecture commitment in AGENTS.md (“single event-reactor”) remains; this design **strengthens** that commitment via Foldkit submodel protocol, it does not introduce multiple SPAs.
- Deck identity for seed remains BFF/DB seat `deck_id` → `Tables.Seed`; Wave 2 only changes **URL shape** and entry UX, not the seed contract.
- Brand/icons: Wave 3 closes the favicon design’s deferred Apple-touch / PWA icon set using the same dragon-on-disc art family.
