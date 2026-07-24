# Client Shell, Deck Builder, and Observability

**Status:** Current (as of 2026-07-24)
**Module:** `client/app/` (entry, routes, update/view), `client/app/shell/**` (auth, decks, lobby), `client/lib/**` (rpc-client, wire, lobby-store, faro, ui helpers), `client/server/**` (Nitro BFF routes + Drizzle), `client/styles/global.css`

---

## Problem Statement

The game client needs more than a board. Before reaching the canvas a player must authenticate with an account, manage and build decks, find or create a table in the lobby, and navigate to the live board URL. The client must handle authentication state across routes, lazy-load card art efficiently, surface meaningful observability data (browser → BFF → gRPC API) without leaking private game state, enforce device orientation requirements, and follow a coherent design system built from a single token source.

These concerns — routing, auth, deck management, card art CDN, state management patterns, observability, design tokens, and build tooling — compose the "client shell" that the board and all other screens live inside.

---

## Solution

The client is a **Foldkit** SPA on **Nitro** (Vite). A single event-reactor owns all routes (`client/app/`: `Model` / `Message` / `update` / `view` with shell submodels). Async/wire work uses Effect at runtime boundaries (`client/lib/rpc-client.ts`, streams, BFF); Foldkit owns UI state. The wire contract is a hand-written Effect HTTP client over the same-origin `/api/rpc` BFF, which dials tonic gRPC. Design tokens are authored in `design.tokens.json` (DTCG) and generated under `bun run gen` into Tailwind v4 `@theme` (`client/styles/tokens.generated.css`) and canvas exports (`client/lib/design-tokens.generated.ts`). Biome handles format/lint. Observability: Grafana Faro (browser) + `@effect/opentelemetry` (BFF) + OTLP/tonic (API), no-op locally unless OTLP is set.

---

## User Stories

- As a new player, I visit the root URL, see the deck list, and am redirected to `/login` because I have no session. After signing up, I return to the deck list.
- As a returning player on `/`, I scan commander tiles, search by name, click a tile to play, and right-click an owned deck to edit or delete it.
- As a returning player, I navigate directly to `/decks/new` and the deck builder loads, showing the full card pool on the left and a blank decklist on the right.
- As a deck builder, I click a pool card to add it, right-click to pick a different printing (art preference), and see the commander picker auto-populate with legendary creatures in my list.
- As a player, I visit `/play` (or follow a table share link), see the lobby, pick a deck from my saved decks, ready up, and wait for the host to start.
- As a player on a portrait phone, I see a native dialog telling me to rotate to landscape; the deck builder and board are hidden behind the dialog.
- As an operator, I open Grafana (via port-forward) and see browser → BFF → API traces correlated by W3C `traceparent`, with no hand/library contents in any span.

---

## Behavior

### App shell and routing (`client/app/routes.ts`, `client/app/view.ts`)

A single Foldkit event-reactor owns routing: `client/app/routes.ts` maps paths to shell views; `client/app/view.ts` renders the active route. Auth-gated routes consult the auth submodel (redirect to `/login?next=…` when unsigned-in). No persistent nav chrome. Global chrome is the portrait gate (Landscape Rule). Routes:

| Path | View | Guard |
|---|---|---|
| `/` | Decks list | auth submodel |
| `/login` | Auth | — |
| `/decks/new` | Deck builder | auth submodel |
| `/decks/:id` | Deck builder (edit) | auth submodel |
| `/play` | Lobby / board wrapper | — |
| `/play/:table` | Play (with table id) | — |
| `/api/[...path]` | lobby/table HTTP passthrough | — |
| `/api/rpc/[...path]` | Effect RPC BFF | — |
| `/api/faro/collect` | Faro proxy | — |

Required identifiers live in path params (wire-protocol-and-visibility spec routing rule). Query params are optional: `?deck=` preselects a deck in the lobby; `?next=` is the post-login redirect target.

### Portrait gate (`client/app/view.ts`, `client/app/subscriptions.ts`, DESIGN.md Landscape Rule)

A native `<dialog showModal>` opens when `(orientation: portrait) and (max-width: 900px)` matches. A Foldkit Mount command defers `.showModal()` until the dialog is connected. Escape is swallowed (`OnCancel` prevents dismissal). The scrim covers the background inert. A Foldkit subscription listens to `matchMedia` changes and closes the gate automatically on landscape flip. It is mounted at the app root so every route is behind it.

### Auth guard (`client/app/update.ts`, `client/app/shell/auth/**`)

`FetchMe` is a Foldkit command wrapping `client.me()` with all failures folded to `null` — any 401, decode error, or transport failure is treated as "not signed in." Route entry runs session checks for protected routes. While the session is unresolved, protected content stays blank; once resolved to `null`, the app redirects to `/login?next=<current-path>`. The `next` redirect target is validated server-side and in-browser: only same-origin absolute paths starting with `/` (not `//` or `/\`) are accepted.

Unsigned protected content never renders.

### Foldkit state and effects (`client/app/model.ts`, `client/app/update.ts`, `client/app/subscriptions.ts`, `client/app/resources.ts`)

The app model is the single UI state tree. `update(model, message)` is the only state transition point and returns `[Model, Command[]]`. Shell submodels own auth, deck list, deck builder, and lobby state; the board owns board interaction state while game deltas fold into `client/app/game/fold.ts`.

Async work is expressed as Foldkit **Commands** backed by Effect programs. Commands depend on the `RpcClient` resource from `client/app/resources.ts`, so wire access is explicit at the runtime boundary. Session checks, auth submit, deck loading, catalog search, deck save/delete, lobby host/join, and table navigation all flow through commands.

Long-lived listeners are Foldkit **Subscriptions**. App subscriptions cover portrait orientation, lobby polling, and game stream frames. Dependency functions decide when each stream is active; returning `Stream.empty` stops work when the route or table changes. Components do not own long-lived fibers.

### Wire protocol (wire-protocol-and-visibility spec, `client/lib/rpc-client.ts`, `client/server/routes/api/rpc/[...path].ts`, `client/lib/wire/grpcClient.ts`)

The browser talks only to the same-origin BFF via the hand-written Effect HTTP client (`client/lib/rpc-client.ts`) over `/api/rpc`. The Nitro BFF dispatches `/api/rpc/**` requests and calls tonic gRPC through `client/lib/wire/grpcClient.ts`. There is no direct browser-to-gRPC communication. The proto wire is the sole contract.

`makeClient(fetch)` accepts a fetch implementation so tests can stub it. `client` is the app singleton (credentials: include, prepended `/api/rpc`). Wire types (`wire/types.ts`) are Effect Schema-decoded DTOs; `wire/protoMap.ts` maps them to/from proto.

### Game delta stream (`client/app/game/stream-subscription.ts`, `client/app/game/fold.ts`)

The game stream is a Foldkit subscription keyed by route table id and active game table id. It opens only when the app is on `/play/:table` and the game slice is active. Snapshot and delta frames become messages, then `update` folds them through `applySnapshotPure` / `applyDeltaPure`. `model.game.connected` drives the reconnect banner; rejected intents set `game.reject` and `board.reject`. The subscription goes empty after navigation or table mismatch, so no residual stream continues after leaving the board.

### Table routing and lobby (`client/lib/lobby/client.ts`, `client/app/shell/lobby/**`, lobby-table-routing-and-live-game spec)

`tableId()` reads the table id from `/play/:table` path. `parseTableCode` normalizes bare codes and share links (pasted URLs with `://` or `/play/` path segment). `setTableUrl` reflects a joined table into the URL via `history.replaceState`.

The lobby polls `GET /tables/{table}/lobby` via a Foldkit subscription until `started`. Seat rows show seat-color dots (`seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane`). The host (first joiner) sees a Start button when ≥2 seats are claimed and all are ready. Table share-link copy uses `navigator.clipboard.writeText` from an Effect-backed command — denied permission reveals a manual-copy input instead of throwing. `unlockTableAudio()` is called on Ready-up (the required user-gesture unlock for the shared `AudioContext`).

When `selectedDeckId` is set from Play → Host/Join or `?deck=`, the lobby shows locked **Bring: `<deck name>`** text and a **Back** link to `/`. It does not render the deck `<select>` in entry or claim-seat states. Bare `/play` with no selected deck keeps the deck selector plus Host / Join controls.

### Deck list and builder (`client/app/shell/decks/**`, client-shell-deck-builder-and-observability spec, accounts-decks-and-catalog spec)

**Deck list** (`/`) shows saved decks from the deck list submodel as a compact tile grid.
Each tile uses commander `art_crop`, deck name, color-identity pips, and a Precon chip when
`id < 0`. The whole tile links to `/play?deck={id}`. A **Search decks…** field filters by
deck name and commander display name (client-only). Display order: owned decks first
(API relative order), then precons by ascending id (newest release first). Right-click on
an owned deck opens Edit (`/decks/{id}`) and Delete (confirm dialog); precons do not open
a context menu. A New Deck button navigates to `/decks/new`.

**Deck builder** (`/decks/new`, `/decks/:id`) is a split-pane layout:

- **Left: card pool grid.** Loads from `/api/rpc/cards/search` in 100-card pages via an `IntersectionObserver` sentinel at the grid bottom. Filters: text search (tokenized LIKE over `search_blob`), set, subtypes (accounts-decks-and-catalog spec). Pool tiles are `POOL_CARD` style: art thumbnail + name + type + cost pips, click-to-add. Right-click (or 500 ms long-press) opens a context menu with printing options and basics shortcuts.
- **Right: decklist panel.** Commander picker (legendary creatures in the list), deck name field, 99-card decklist with per-card counts and a running total. Click a row to remove one. Deck save calls `/api/rpc/decks` or `/api/rpc/decks/:id` with `SaveDeckRequest`.
- **Printing preference.** Card identity is the Scryfall oracle id (`CardDef.id`); a Printing is a Scryfall UUID used only for art (accounts-decks-and-catalog spec). `preferredPrint` is session-sticky per oracle id — once you pick a printing for a card, adding it again reuses that choice. `searchPrints(oracleId)` fetches Scryfall prints for the picker.
- **Singleton enforcement.** Non-basic non-commander cards cap at 1. Commander is set via the context menu only; `canBeCommander` restricts to legendary creatures.
- **Full Commander legality** is enforced server-side on save; the client surfaces validation errors returned as `CreateDeck422` / `UpdateDeck422` tagged Schema errors.
- **Card lookup.** `lookupCardsByIds(ids, client)` fetches oracle data for deck hydration through `/api/rpc/cards/lookup`.

### Card art CDN (client-shell-deck-builder-and-observability spec, accounts-decks-and-catalog spec, `lib/scryfall.ts`)

Art is keyed by Scryfall **Printing** UUID. `imageUrlByPrint(printId, size, face)` returns:
- CDN URL (`VITE_CARD_CDN/large/{face}/{a}/{b}/{id}.webp`) when `VITE_CARD_CDN` is baked at build.
- Scryfall image API (`https://api.scryfall.com/cards/{id}?format=image&version={size}`) otherwise (local dev only).

Missing CDN art is a broken `<img>` — no Scryfall fallback in production. The CDN path replicates Scryfall's folder fan (`first two hex chars` of the UUID). DFC backs are fetched with `face=back` in the Scryfall path; CDN serves the same `large` webp. `imageFaceAfterLoadError` falls back from `back` to `front` on load error (DFC prepare/flip cards have no Scryfall `/back/` — transformer backs that exist load on first try).

`cardBackUrl()` returns `/card-back.webp` for library piles and face-down cards.

**Deck image preload** (`lib/deckImagePreload.ts`): on Board mount, `preloadDecksIntoCache(ids, cache)` fetches all seated decks' art into `sharedImageCache` so gameplay hits the cache. `imageCache.ts` provides a simple URL→HTMLImageElement cache with a subscriber list for canvas redraws on image settle.

### Design system (client-shell-deck-builder-and-observability spec, `DESIGN.md`, `design.tokens.json`, `global.css`)

`design.tokens.json` (DTCG) is the **single source of truth** for design token values. `bun run gen` (Style Dictionary) generates `client/styles/tokens.generated.css` (Tailwind v4 `@theme`) and `client/lib/design-tokens.generated.ts` (canvas named colors). `global.css` imports generated theme output and keeps hand-authored keyframes/interaction rules. Foldkit HTML helpers and shared UI helpers in `client/lib/ui/` own component recipes — never via `@apply`, and not as token component maps. Inline style is used only for CSS variables; classes carry appearance. Arbitrary values (`bg-[#18221ef5]`) are for one-off values that token files do not name; they do not extend the token list.

Key semantic tokens:
- `forest-floor` (#0B1310) — canvas background, `index.html` inline background (prevents flash).
- `forest-surface` (#101816FA) — panels.
- `forest-hud` (#0C1412EB) — HUD panels.
- `llanowar` / `llanowar-deep` — primary buttons (hover → active).
- `priority-gold` (#FFD76A) — priority orb. **Gold = a decision is owed** (The Gold Means Act Rule).
- `vine` (#22CC44) — active borders.
- Seat colors: `seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane` — player identity, never semantics.
- Combat semantics: `mountain-red` (attack), `wall-green` (block), `island-blue` (targeting).

Typography is `system-ui` only. Screen ramp: `title` 18/700, `body` 14/400, `button-label` 14/600, `label` 13, `caption` 12, `game` 15/600, `display` 22/700. HUD density: `chip` 11, `micro` 10 (board/hand chrome only). No display fonts. Rounded corners: `panel` 12px, `modal` 10px, `game` 10px, `hud` 8px, `control` 6px, `focus` 4px.

The `mana-oracle.css` import brings in the mana-font glyph subset (icon font, not body text). A custom `@font-face` overrides the mana-font package to prefer woff2 for canvas `ctx.fillText`. Mana pips in oracle text use `ms.ms-oracle` with `font-size: 0.78em` so pips don't dominate the body line.

### Biome (client-shell-deck-builder-and-observability spec)

Biome 2.5.3 handles format, lint, and import ordering (`assist.actions.source.organizeImports`, `sortBareImports: true`). `nursery/useSortedClasses` is at error for Tailwind class sorting and configured for safe fixes over `cn` / `clsx`. CSS: `tailwindDirectives: true`. The `test` domain is recommended.

### BFF OTEL and Faro observability (production-topology-and-operations spec)

**Browser (Faro):** `client/app/faro.ts` (called from `client/app/entry.ts`) installs `@grafana/faro-web-sdk` + `@grafana/faro-web-tracing`. Posts to same-origin `/api/faro/collect`; the BFF proxies to Alloy `faro.receiver`. Session sampling forced to 100%; stale sessions (`isSampled=false` in `sessionStorage`) are repaired. `traceparent` propagation is same-origin `/api` only.

**BFF (OTEL):** `plugins/otel.server.ts` (Nitro plugin) installs a process-scoped `@effect/opentelemetry` `ManagedRuntime` once at server start via `initOtel()`. Exports OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set; no-ops otherwise. `runTracedRequest(traceparent, spanName, body)` is the standard edge entry: it continues inbound W3C `traceparent` as the BFF span parent **only when sampled** (unsampled Faro non-recording spans are ignored — avoids `<root span not yet received>` Tempo orphans), opens `spanName`, and runs on the OTEL runtime.

**Trace propagation rule:** Effect parent spans live in **fiber Context**. The gRPC `ManagedRuntime` is separate — context does not survive `runPromise` into outbound calls. `grpcRequestEnv` captures `{ sessionToken, traceparent }` once per request edge under `runTracedRequest`, then passes it into every gRPC call. Do not use Node AsyncLocalStorage or per-helper optional `traceparent` args.

**Scrub rules:** identifiers, timing, error classes only. No hand/library contents, intent payloads, or auth headers. Faro collect is capped at 512 KiB (`FARO_MAX_BODY_BYTES`); oversize requests return 413.

**Local dev:** exporters no-op when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset. `RUST_LOG` still drives fmt. Grafana is operator-only via `kubectl port-forward` — no public hostname.

### Auth UI (`client/app/shell/auth/view.ts`, `client/app/shell/auth/update.ts`)

Single-page login/signup (toggled, not separate routes). `Login` and `Signup` are Foldkit commands wrapping `client.login` / `client.signup`. 401 → "Wrong email or password", 409 → "That email is already registered", anything else → "Something went wrong." On success the server sets an HttpOnly session cookie and the client navigates to `safeNext(params.next)`. `safeNext` enforces same-origin absolute paths only: rejects missing, relative, protocol-relative `//`, backslash `/\`, or scheme-carrying targets.

### Build metadata (`lib/buildMeta.ts`)

`appVersion()` and `gitCommit()` read from `VITE_APP_VERSION` and `VITE_GIT_COMMIT` env vars baked at build time. Consumed by the BFF OTEL SDK's `serviceVersion` and `vcs.ref.head.revision` resource attributes, and by the `AppVersion` component.

### Lobby poll and table lifecycle (`client/app/shell/lobby/poll.ts`, `client/app/shell/lobby/subscriptions.ts`, `client/lib/lobby-store.ts`)

`lobbyPoll(tableId)` is an Effect stream consumed by a Foldkit subscription. The subscription polls lobby state while a table is present and stops when `started` is true. `client/lib/lobby-store.ts` holds lobby helpers for multi-seat coordination. Once the lobby moves to `started`, the app transitions from the lobby view to the board mount, preserving the table id in the route.

---

## Implementation Decisions

- **Foldkit `update` is the state boundary.** UI state changes only through messages handled by `client/app/update.ts` and shell child updates. Async work returns messages through Foldkit commands and subscriptions, which gives consistent error folding, runtime resource injection, and automatic stream teardown.
- **`FetchMe` folds all failures to `null`.** Any 401, decode error, or transport failure during `client.me()` is "not signed in" — mirrors the guard's semantics. Route entry refreshes session state for protected routes to avoid stale login redirects.
- **Deck-builder search is server-side.** The client holds no full catalog. `/api/rpc/cards/search` calls `Cards.Search` with tokenized LIKE over `search_blob` (includes `otags`) on the server (accounts-decks-and-catalog spec). The pool grid pages in 100-card chunks via IntersectionObserver — no client-side filtering of a local dataset.
- **Printing is art-preference only.** Card rules identity is the oracle id. Decks store `(id, count, print)` with `print` required. The engine is print-agnostic. Wire DTOs carry `print` for consistent art across all clients.
- **`safeNext` is checked both in-browser and server-side.** Open-redirect mitigations are layered: the client validates before navigation; the server validates before the session redirect.
- **Faro unsampled span ignore.** Faro's tracing sampler often marks sessions `NOT_RECORD` while the fetch instrumentation still injects a `traceparent` for the non-recording span. Parenting BFF spans under an unsampled span leaves Tempo with `<root span not yet received>` orphans. The BFF rejects inbound `traceparent` where `traceFlags & 0x01 === 0`.
- **No `@apply`, no `@layer components`.** Foldkit views and shared UI helpers carry styling through Tailwind classes; inline style carries only CSS variable data. This is the Tailwind client-shell-deck-builder-and-observability spec house rule.
- **Biome class sorting.** `nursery/useSortedClasses` is at error and configured for safe `cn` / `clsx` fixes. Keep class strings sorted in code review and use the editor or Biome fix path for drift.
- **Gzip LZ77 benefit from sorted classes.** client-shell-deck-builder-and-observability spec notes that consistent Tailwind class ordering makes repeated utility sequences longer LZ77 matches under gzip on the shipped JS/HTML.
- **`VITE_CARD_CDN` is build-time baked**, not runtime. Changing CDN requires a new image build.
- **No Scryfall fallback in production.** Missing CDN art is a broken image. This is a deliberate choice (client-shell-deck-builder-and-observability spec) to avoid rate-limiting Scryfall in production.

---

## Testing Decisions

- `client/app/shell/**/*.test.ts` — auth, decks list/builder, lobby stories and helpers.
- `client/app/routes.test.ts`, `client/app/smoke.test.ts` — routing and smoke.
- `client/app/game/*.test.ts` — game fold, stream subscription.
- `client/lib/rpc-client.test.ts` — Effect HTTP client (stubbed fetch).
- `client/lib/wire/*.test.ts` — BFF gRPC / RPC method gate.
- `client/lib/lobby-store.test.ts` — lobby state.
- `client/lib/deck-builder/*.test.ts` — print prefs, menus, hover preview.
- `client/lib/ui/*.test.ts`, `client/lib/cn.test.ts` — Foldkit UI helpers (`buttonClass`, surfaces).
- `client/lib/build-meta.test.ts` — version/commit env var reading.
- Board geometry/paint/HTML tests live under `client/app/board/**` (see board spec / `docs/client-canvas-map.md`).
- Integration test: `just client-check` runs Biome lint + typecheck + Vitest. The full check is `just check` (server + client).

---

## Out of Scope

- Server-side rendering of board state (SPA on Nitro; no SSR of the board).
- Progressive Web App (PWA) / service worker / offline mode.
- Sitemaps, SEO meta, or marketing pages (`robots.txt` disallows all crawlers).
- Multi-account switching within one browser session.
- OAuth / social login (email+password only).
- Grafana Faro public dashboard (operator-only via port-forward).
- Cross-browser fallback for `navigator.clipboard` beyond the existing try/catch reveal pattern.

---

## Further Notes

- **`design.tokens.json` is the token source.** Token values are authored there, then generated into `client/styles/tokens.generated.css` and `client/lib/design-tokens.generated.ts`; never hand-edit generated outputs.
- **Effect / `@effect/*` packages must be pinned to the same exact beta.** Breaking the pin causes runtime type mismatches between Effect fibers from different versions.
- **Wire codegen.** `.proto` is the sole contract (wire-protocol-and-visibility spec). After proto changes: `just server-codegen` / `bun run gen` to regenerate the gitignored `client/lib/wire/generated/` directory. The BFF gRPC client imports from there.
- **Faro size cap.** `FARO_MAX_BODY_BYTES = 512 KiB`. The `/api/faro/collect` route rejects oversized payloads with 413 before reading the full body (`contentLengthTooLarge` checks `Content-Length` header first; `readBodyCapped` streams and checks).
- **Safe area insets.** The landscape rule applies to notched devices — `viewport-fit=cover` with safe-area insets. The portrait gate handles the notched-portrait case; landscape layout tightens padding but does not re-stack.
- **`just client-check`** is the canonical verification: Biome format + lint (including sorted-class check) + TypeScript typecheck + Vitest. Always run before committing client changes.
- **The lobby is on `mtgfr_web`** (Nitro BFF / Drizzle / Postgres `mtgfr_web`), not `mtgfr` (Toasty / game/user data). `just client-migrate` applies Drizzle migrations; `just migrate` applies Toasty migrations. Both must run before DB-touching work.
- **Live client architecture** is Foldkit + Nitro with `client/app/`, `client/lib/`, and `client/server/` as the module split.
