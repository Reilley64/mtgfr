# Foldkit submodels, domain layout, play routes, and installable PWA (design)

**Status:** Approved design input (2026-07-26).
**Surfaces (update at implement time):** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md), [lobby-entry-ui](2026-07-20-lobby-entry-ui.md), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md), [deck-list-and-builder](2026-07-20-deck-list-and-builder.md), [board-composition](2026-07-20-board-composition.md); favicon / icon notes in [favicon-dragon-silhouette-design](2026-07-25-favicon-dragon-silhouette-design.md).
**Upstream patterns:** [Foldkit project organization](https://foldkit.dev/patterns/project-organization), [Informing submodels](https://foldkit.dev/patterns/informing-submodels), [Submodels](https://foldkit.dev/patterns/submodels).

---

## Problem Statement

The client is a single Foldkit SPA with feature folders, but it drifts from Foldkit’s documented organization and submodel protocol: child messages are flattened into the parent `Message` union, route entry uses ad hoc helpers instead of `inform*`, there are no `index.ts` namespace re-exports, and shared code lives in `client/lib/` rather than an app-local `domain/`. Play URLs still require a deck id path segment even after the seat’s deck is stored in `mtgfr_web` on join/seed. Progressive Web App installability (manifest / service worker / offline) is explicitly out of scope in shell-routes today; friends want a native-feeling install without offline play.

## Goal

1. Align the client with Foldkit project-organization and informing-submodels conventions (`Got*Message`, `Command.mapMessages`, `inform*`, feature `index.ts` namespaces, `domain/`).
2. Reshape play routes so deck picking stays on `/` (home deck list), Host/Join entry stays `/play/:deckId`, seated pregame stays `/play/:deckId/:tableId`, and the live game becomes table-only (`/play/:tableId`).
3. Ship an **installable-only** PWA via `vite-plugin-pwa` (browser-native install; network-only service worker; no precache / offline mode).

## Locked decisions

| Decision | Choice |
|---|---|
| Micro-frontends / Module Federation | **No** — one Foldkit reactor, one Vite/Nitro app |
| Submodel protocol | **Full Foldkit alignment (C)** — `Got*Message` wrappers, `Command.mapMessages`, `inform*` helpers that run child `update` |
| Shared code location | Rename/move `client/lib/` → `client/app/domain/` (Foldkit `domain/` sibling to features); keep `client/server/` and `client/styles/` outside the app TEA tree |
| Feature namespaces | Add `index.ts` re-exports per feature (`export * as Lobby from './lobby'`, etc.) |
| `page/` folder rename | **Not required** — keep product folders `shell/`, `board/`, `game/` as the feature roots (equivalent role to docs’ `page/`) |
| Deck pick | `/` — home deck list (unchanged); Play navigates to `/play/:deckId` |
| Play entry (Host/Join) | `/play/:deckId` (unchanged) |
| Pregame (seated lobby) | `/play/:deckId/:tableId` (unchanged path shape) |
| Live game | `/play/:tableId` only (after start); deck already on seats in DB / seed |
| Legacy play URLs | Hard cut Not found: bare `/play`, `?deck=`, and other obsolete shapes (not `/play/:deckId`) |
| PWA ambition | Installable only (A) — not offline |
| PWA tooling | **Approach 2** — `vite-plugin-pwa` with **injectManifest** (or equivalent) and a hand-authored **network-only** SW (no precache, no runtime caching) |
| Install UX | Browser-native only — no in-app Install button / `beforeinstallprompt` chrome |
| Foldkit version | `foldkit` **`^0.132.0`** (from `^0.131.0`); bump `@foldkit/vite-plugin` / `@foldkit/devtools-mcp` only if 0.132 peers or changelog require it |
| Effect version | **`effect` and every direct `@effect/*` dependency pinned to the same exact `4.0.0-beta.101`** (from `4.0.0-beta.97`) — AGENTS.md same-beta rule |
| Effect platform packages | Direct deps match runtimes: `@effect/platform-browser` (SPA). **Do not** keep unused `@effect/platform-node` (Nitro BFF is Bun). Add `@effect/platform-bun@4.0.0-beta.101` only when a BFF Effect Layer actually needs it; `@effect-grpc/codegen` may still pull `platform-node` transitively |
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

1. Move deck picking onto `/play` — Rejected; home deck list on `/` already owns that job.
2. Drop deck from all play URLs (`/play/:table` only) — Loses explicit pregame “bringing this deck” in the path for Host/Join and seated lobby.
3. **Keep `/` → `/play/:deckId` → `/play/:deckId/:tableId`; strip to `/play/:tableId` after start (chosen)** — Minimal URL change; in-game share links no longer carry a deck segment.
4. Keep current `/play/:deckId/:table` for the live board forever — Rejected; once seeded, table id alone is enough.

## Design

### Wave plan

Implement as **four waves** (separate PRs or stacked commits; each wave that changes product behavior updates living surface specs in the same change):

| Wave | Deliverable |
|---|---|
| **0** | Dependency bump: `foldkit` `^0.132.0`; `effect` + `@effect/opentelemetry` + `@effect/platform-browser` + `@effect/sql-pg` (and any other direct `@effect/*` pins) all **`4.0.0-beta.101`**; **remove** unused direct `@effect/platform-node` (BFF is Bun; add `@effect/platform-bun` only when needed); refresh `bun.lock`; fix compile/test breakages from the bump only — no protocol/route/PWA work in this wave unless required to compile |
| **1** | Foldkit submodel protocol + `client/app/domain/` move + feature `index.ts` namespaces |
| **2** | Play route reshape (in-game table-only URL) + lobby/parse/share updates |
| **3** | Installable PWA (`vite-plugin-pwa`, icons, network-only SW, `entry` registration) |

Wave 0 is a prerequisite for Waves 1–3 (new Foldkit APIs / examples track 0.132). Waves 1–2 may merge if review prefers one PR; Wave 3 must not enable precaching.

#### Wave 0 — dependency bump (detail)

- Edit `client/package.json` pins; run `bun install` in `client/`.
- Do **not** leave mixed Effect betas (fiber/type skew) among **direct** `@effect/*` pins.
- Drop unused direct `@effect/platform-node`. Prefer `@effect/platform-bun` for any future BFF platform Layers (Nitro `preset: "bun"`).
- `@effect-grpc/*` stays on its own beta line unless the install fails peer resolution — then bump only as far as needed and document in the Wave 0 PR.
- Verify: `just client-check` (and Foldkit DevTools still connect if `@foldkit/devtools-mcp` / vite-plugin versions change).

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
| `/` | `HomeRoute` | Deck list — choose deck / Play (unchanged) |
| `/play/:deckId` | `PlayRoute` | Host/Join entry for that deck (unchanged) |
| `/play/:deckId/:tableId` | `PregameTableRoute` (today’s `TableRoute` pregame role) | Seated pregame lobby |
| `/play/:tableId` | `GameTableRoute` | Live board (after start) |
| Bare `/play`, `?deck=`, obsolete shapes | `NotFoundRoute` | Hard cut |

Router `oneOf` order: two-segment pregame **before** one-segment game **before** `/play/:deckId` entry (or discriminate game vs entry: table ids are unguessable hex; deck ids are numeric — `normalizeAppRoute` sends numeric single segments to `PlayRoute` and non-numeric to `GameTableRoute`). Table ids remain unguessable hex (existing lobby behavior).

#### Behavior

1. **`/`:** Deck list as today. Play on a deck tile navigates to `/play/:deckId` (FLIP morph unchanged).
2. **`/play/:deckId`:** Host/Join entry as today. Host create or Join with code + `deck_id` → navigate to `/play/:deckId/:tableId`.
3. **`/play/:deckId/:tableId`:** Pregame lobby as today (claim/ready/start, poll, watch note). Path deck id is the local player’s bringing deck; seat row still persists `deck_id` in `mtgfr_web` on join.
4. **Start → game:** On lobby `started`, parent navigates to `/play/:tableId` (strip deck segment), activates `GameSlice`, board mounts, game stream keys off table id only.
5. **Share / parse:** `parseTableCode` accepts `/play/:deckId/:tableId` and `/play/:tableId`, plus bare codes. Joiner still picks their own deck on `/` then joins from `/play/:theirDeckId` (or pastes a code there); host path deck segment is not the joiner’s deck.

#### Spec / product copy updates (implement wave)

- shell-routes route table and guards (add `GameTableRoute`; clarify pregame vs in-game).
- lobby-entry-ui: start handoff navigates to table-only path; entry/pregame paths unchanged.
- lobby-table-routing: pregame remains `/play/{deck}/{table}`; in-game becomes `/play/{table}` only.
- deck-list-and-builder: Play CTA remains `/play/:deckId` (no change to home picker).

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

### Wave 0

- Lockfile and `package.json` show `foldkit` `^0.132.0` and Effect family `4.0.0-beta.101` with no stray `beta.97` **direct** deps; no unused direct `@effect/platform-node`.
- `just client-check` green; no intentional product behavior changes.

### Wave 1

- Parent `Message` exhaustiveness: one `Got*` arm per child; `Command.mapMessages` covered by existing story/update tests updated for wrappers.
- Scene tests: dispatch wrapped messages; board `toParentMessage` wrapping.
- Import/path: `tsc` + vitest green after `lib` → `domain` move.
- Route/session cold-load: protected deep link runs `inform*` after session (extend shell/lobby/deck tests).

### Wave 2

- `routes.test.ts`: `/play/:deckId` entry, `/play/:deckId/:tableId` pregame, `/play/:tableId` in-game; bare `/play` and `?deck=` Not found; numeric vs hex single-segment discrimination.
- `parseTableCode` tests for both path shapes.
- Lobby entry/scene: home → `/play/:deckId` unchanged; start navigates to table-only path; board stream still keys on table id.
- Update shell surfaces / lobby entry Scene assertions for the in-game path.

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
