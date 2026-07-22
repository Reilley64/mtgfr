# Client Shell, Deck Builder, and Observability

**Status:** Current (as of 2026-07-20)
**Module:** `client/src/app.tsx`, `client/src/routes/`, `client/src/atoms.ts`, `client/src/store.ts`, `client/src/guard.tsx`, `client/src/net.ts`, `client/src/effect/client.ts`, `client/src/effect/stream.ts`, `client/src/effect/otel.ts`, `client/src/wire/grpcClient.ts`, `client/src/wire/rpcServer.ts`, `client/src/wire/protoMap.ts`, `client/src/lib/scryfall.ts`, `client/src/lib/cardArtFace.ts`, `client/src/lib/deckBuilderPrint.ts`, `client/src/lib/deckImagePreload.ts`, `client/src/lib/faroCollect.ts`, `client/src/lib/faroSession.ts`, `client/src/lib/buildMeta.ts`, `client/src/lib/traceContext.ts`, `client/src/lib/lobbyClient.ts`, `client/src/lib/lobbyStore.ts`, `client/src/lib/lobbyTypes.ts`, `client/src/lib/lobbyPoll.ts`, `client/src/components/organisms/auth.tsx`, `client/src/components/organisms/deck-builder.tsx`, `client/src/components/organisms/decks.tsx`, `client/src/components/organisms/lobby.tsx`, `client/src/components/organisms/play.tsx`, `client/src/components/atoms/`, `client/src/components/molecules/portrait-gate.tsx`, `client/src/global.css`, `client/src/plugins/otel.client.ts`, `client/src/plugins/otel.server.ts`

---

## Problem Statement

The game client needs more than a board. Before reaching the canvas a player must authenticate with an account, manage and build decks, find or create a table in the lobby, and navigate to the live board URL. The client must handle authentication state across routes, lazy-load card art efficiently, surface meaningful observability data (browser → BFF → gRPC API) without leaking private game state, enforce device orientation requirements, and follow a coherent design system built from a single token source.

These concerns — routing, auth, deck management, card art CDN, state management patterns, observability, design tokens, and build tooling — compose the "client shell" that the board and all other screens live inside.

---

## Solution

The client is a **SolidStart 1.3** app (`ssr: false`) with Vinxi. The shell is a thin `<Router>` in `app.tsx` wrapping file-based routes; the only global chrome is the `PortraitGate` dialog (Landscape Rule) and a `<Suspense>` boundary. All async/wire work goes through **Effect atoms** (`effect/unstable/reactivity/Atom` + `@effect/atom-solid`) per client-shell-deck-builder-and-observability spec. Solid signals/stores handle view-local state and the game fold in `store.ts`. The wire contract is a hand-written Effect HTTP client over the same-origin `/api/rpc` BFF, which dials tonic gRPC to the API server (wire-protocol-and-visibility spec). Design tokens live in the YAML frontmatter of `DESIGN.md` and are mirrored into a Tailwind v4 `@theme` in `global.css` (client-shell-deck-builder-and-observability spec). Biome handles format, lint, and import ordering (client-shell-deck-builder-and-observability spec). Observability runs Grafana Faro (browser) + `@effect/opentelemetry` (BFF) + OTLP/tonic (API), all no-op locally unless the OTLP endpoint is set (production-topology-and-operations spec).

---

## User Stories

- As a new player, I visit the root URL, see the deck list, and am redirected to `/login` because I have no session. After signing up, I return to the deck list.
- As a returning player, I navigate directly to `/decks/new` and the deck builder loads, showing the full card pool on the left and a blank decklist on the right.
- As a deck builder, I click a pool card to add it, right-click to pick a different printing (art preference), and see the commander picker auto-populate with legendary creatures in my list.
- As a player, I visit `/play` (or follow a table share link), see the lobby, pick a deck from my saved decks, ready up, and wait for the host to start.
- As a player on a portrait phone, I see a native dialog telling me to rotate to landscape; the deck builder and board are hidden behind the dialog.
- As an operator, I open Grafana (via port-forward) and see browser → BFF → API traces correlated by W3C `traceparent`, with no hand/library contents in any span.

---

## Behavior

### App shell and routing (`app.tsx`, `routes/`)

The root component mounts a `<PortraitGate />` and a SolidStart `<Router>` with file-based `<FileRoutes />` inside a `<Suspense>`. No persistent nav chrome. Routes:

| Path | Component | Guard |
|---|---|---|
| `/` | `Decks` | `RequireAuth` |
| `/login` | `Auth` | — |
| `/decks/new` | `DeckBuilder` | `RequireAuth` |
| `/decks/:id` | `DeckBuilder` (edit) | `RequireAuth` |
| `/play` | `Play` (lobby/board wrapper) | — |
| `/play/:table` | `Play` (with table id) | — |
| `/api/[...path]` | lobby/table HTTP passthrough | — |
| `/api/rpc/[...path]` | Effect RPC BFF | — |
| `/api/faro/collect` | Faro proxy | — |

Required identifiers live in path params (wire-protocol-and-visibility spec routing rule). Query params are optional: `?deck=` preselects a deck in the lobby; `?next=` is the post-login redirect target.

### Portrait gate (`components/molecules/portrait-gate.tsx`, DESIGN.md Landscape Rule)

A native `<dialog showModal>` opens when `(orientation: portrait) and (max-width: 900px)` matches. It uses `openModalWhenReady` to defer `.showModal()` until the dialog is connected. Escape is swallowed (`onCancel` calls `preventDefault`). The scrim covers the background inert. The gate reacts to `matchMedia` change events and closes automatically on landscape flip. It is mounted at the app root so every route is behind it.

### Auth guard (`guard.tsx`, `atoms.ts`)

`useAuthGuard()` consumes the shared `meAtom` via `useAtomResource`. `meAtom` is an Effect atom wrapping `client.me()` with all failures folded to `null` — any 401, decode error, or transport failure is treated as "not signed in." The guard refreshes `meAtom` on mount (so a login navigation does not present a stale `null`) and redirects to `/login?next=<current-path>` on `null`. The `next` redirect target is validated server-side and in-browser: only same-origin absolute paths starting with `/` (not `//` or `/\`) are accepted.

`RequireAuth` wraps children in a `Show when={user()}` so unsigned content never renders.

### Effect-first client state (client-shell-deck-builder-and-observability spec, `atoms.ts`, `store.ts`)

All async/wire work — session check, deck list, catalog search — lives in **atoms** (`Atom.make(...)`, `Atom.fn(...)`). Solid components consume atoms via `useAtomValue`, `useAtomSet`, `useAtomResource`, `useAtomMount`. No `createResource(() => run(…))` and no manual fiber lifecycle in components.

Shared atoms live in `atoms.ts`:
- `meAtom` — signed-in user or `null`.
- `decksAtom` — deck list; waits on `meAtom` (empty array for unsigned-in to avoid a 401 race).

Screen-local atoms (deck builder, lobby) live in their owning component files. Tests use `AtomRegistry.make()`.

`store.ts` holds the game state as a Solid store (`createStore<GameStore>`): `{ state: VisibleState | null, seq, reject, log }`. `applyDelta` and `applySnapshot` are the only mutators. The game store is **not** an Effect atom — it is a Solid store so the canvas's `createEffect` can track it synchronously without async suspension.

### Wire protocol (wire-protocol-and-visibility spec, `effect/client.ts`, `wire/grpcClient.ts`)

The browser talks only to the same-origin BFF via the hand-written Effect HTTP client (`effect/client.ts`) over `/api/rpc`. The BFF calls tonic gRPC (`wire/grpcClient.ts`) to the API server. There is no direct browser-to-gRPC communication. The proto wire is the sole contract.

`makeClient(fetch)` accepts a fetch implementation so tests can stub it. `client` is the app singleton (credentials: include, prepended `/api/rpc`). Wire types (`wire/types.ts`) are Effect Schema-decoded DTOs; `wire/protoMap.ts` maps them to/from proto.

### Game delta stream (`effect/stream.ts`, `net.ts`)

`gameStreamFamily(tableId)` is a per-table atom that opens a streaming delta connection. Mounting it (via `useAtomMount` in `Board.tsx`) runs the stream fiber; unmounting interrupts it. Each frame calls `applySnapshot` or `applyDelta` on `store.ts`. `connectedAtom` reflects the stream health (for the reconnect banner). `setReject` records server error messages to `game.reject`. The stream fiber runs for exactly the Board component's lifetime — no residual fibers after navigation.

### Table routing and lobby (`lib/lobbyClient.ts`, `components/organisms/lobby.tsx`, lobby-table-routing-and-live-game spec)

`tableId()` reads the table id from `/play/:table` path. `parseTableCode` normalizes bare codes and share links (pasted URLs with `://` or `/play/` path segment). `setTableUrl` reflects a joined table into the URL via `history.replaceState`.

The lobby polls `GET /tables/{table}/lobby` via `lobbyPollFamily` (atom-based poll) until `started`. Seat rows show seat-color dots (`seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane`). The host (first joiner) sees a Start button when ≥2 seats are claimed and all are ready. Table share-link copy uses `navigator.clipboard.writeText` wrapped in `Atom.fn` — a denied permission reveals a manual-copy input instead of throwing. `unlockTableAudio()` is called on Ready-up (the required user-gesture unlock for the shared `AudioContext`).

### Deck list and builder (`components/organisms/decks.tsx`, `components/organisms/deck-builder.tsx`, client-shell-deck-builder-and-observability spec, accounts-decks-and-catalog spec)

**Deck list** (`/`) shows saved decks from `decksAtom`. Each row links to the builder. A New Deck button navigates to `/decks/new`.

**Deck builder** (`/decks/new`, `/decks/:id`) is a split-pane layout:

- **Left: card pool grid.** Loads from `GET /cards/search` in 100-card pages via an `IntersectionObserver` sentinel at the grid bottom. Filters: text search (tokenized LIKE over `search_blob`), set, subtypes (accounts-decks-and-catalog spec). Pool tiles are `POOL_CARD` style: art thumbnail + name + type + cost pips, click-to-add. Right-click (or 500 ms long-press) opens a context menu with printing options and basics shortcuts.
- **Right: decklist panel.** Commander picker (legendary creatures in the list), deck name field, 99-card decklist with per-card counts and a running total. Click a row to remove one. Deck save calls `PUT /decks/:id` or `POST /decks` with `SaveDeckRequest`.
- **Printing preference.** Card identity is the Scryfall oracle id (`CardDef.id`); a Printing is a Scryfall UUID used only for art (accounts-decks-and-catalog spec). `preferredPrint` is session-sticky per oracle id — once you pick a printing for a card, adding it again reuses that choice. `searchPrints(oracleId)` fetches Scryfall prints for the picker.
- **Singleton enforcement.** Non-basic non-commander cards cap at 1. Commander is set via the context menu only; `canBeCommander` restricts to legendary creatures.
- **Full Commander legality** is enforced server-side on save; the client surfaces validation errors returned as `CreateDeck422` / `UpdateDeck422` tagged Schema errors.
- **Card lookup.** `lookupCardsByIds(ids, client)` fetches oracle data for deck hydration (`GET /cards/lookup?names=`).

### Card art CDN (client-shell-deck-builder-and-observability spec, accounts-decks-and-catalog spec, `lib/scryfall.ts`)

Art is keyed by Scryfall **Printing** UUID. `imageUrlByPrint(printId, size, face)` returns:
- CDN URL (`VITE_CARD_CDN/large/{face}/{a}/{b}/{id}.webp`) when `VITE_CARD_CDN` is baked at build.
- Scryfall image API (`https://api.scryfall.com/cards/{id}?format=image&version={size}`) otherwise (local dev only).

Missing CDN art is a broken `<img>` — no Scryfall fallback in production. The CDN path replicates Scryfall's folder fan (`first two hex chars` of the UUID). DFC backs are fetched with `face=back` in the Scryfall path; CDN serves the same `large` webp. `imageFaceAfterLoadError` falls back from `back` to `front` on load error (DFC prepare/flip cards have no Scryfall `/back/` — transformer backs that exist load on first try).

`cardBackUrl()` returns `/card-back.webp` for library piles and face-down cards.

**Deck image preload** (`lib/deckImagePreload.ts`): on Board mount, `preloadDecksIntoCache(ids, cache)` fetches all seated decks' art into `sharedImageCache` so gameplay hits the cache. `imageCache.ts` provides a simple URL→HTMLImageElement cache with a subscriber list for canvas redraws on image settle.

### Design system (client-shell-deck-builder-and-observability spec, `DESIGN.md`, `global.css`)

DESIGN.md's YAML frontmatter is the **single source of truth** for design tokens. `global.css` mirrors those tokens into a Tailwind v4 `@theme` block. Solid component wrappers in `components/atoms/` realize the DESIGN.md `components` map — never via `@apply`. `style` is used only for CSS variables (`style={{ "--x": ... }}`); classes carry appearance. Arbitrary values (`bg-[#18221ef5]`) are for one-off values that DESIGN.md does not name; they do not extend the token list.

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

Biome 2.5.3 handles format, lint, and import ordering (`assist.actions.source.organizeImports`, `sortBareImports: true`). `nursery/useSortedClasses` at error for Tailwind class sorting — the fix is **unsafe** (use `bunx biome check --write --unsafe --only=lint/nursery/useSortedClasses` or an editor fix; `bun run lint:fix` alone does not sort). `src/api/generated.ts` is excluded. CSS: `tailwindDirectives: true`. Domains: `solid`, `test` (recommended).

### BFF OTEL and Faro observability (production-topology-and-operations spec)

**Browser (Faro):** `plugins/otel.client.ts` (imported from `entry-client.tsx`) installs `@grafana/faro-web-sdk` + `@grafana/faro-web-tracing`. Posts to same-origin `/api/faro/collect`; the BFF proxies to Alloy `faro.receiver`. Session sampling forced to 100%; stale sessions (`isSampled=false` in `sessionStorage`) are repaired. `traceparent` propagation is same-origin `/api` only.

**BFF (OTEL):** `plugins/otel.server.ts` (Nitro plugin) installs a process-scoped `@effect/opentelemetry` `ManagedRuntime` once at server start via `initOtel()`. Exports OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT` is set; no-ops otherwise. `runTracedRequest(traceparent, spanName, body)` is the standard edge entry: it continues inbound W3C `traceparent` as the BFF span parent **only when sampled** (unsampled Faro non-recording spans are ignored — avoids `<root span not yet received>` Tempo orphans), opens `spanName`, and runs on the OTEL runtime.

**Trace propagation rule:** Effect parent spans live in **fiber Context**. The gRPC `ManagedRuntime` is separate — context does not survive `runPromise` into outbound calls. `grpcRequestEnv` captures `{ sessionToken, traceparent }` once per request edge under `runTracedRequest`, then passes it into every gRPC call. Do not use Node AsyncLocalStorage or per-helper optional `traceparent` args.

**Scrub rules:** identifiers, timing, error classes only. No hand/library contents, intent payloads, or auth headers. Faro collect is capped at 512 KiB (`FARO_MAX_BODY_BYTES`); oversize requests return 413.

**Local dev:** exporters no-op when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset. `RUST_LOG` still drives fmt. Grafana is operator-only via `kubectl port-forward` — no public hostname.

### Auth UI (`components/organisms/auth.tsx`)

Single-page login/signup (toggled, not separate routes). `authenticateFn` is an `Atom.fn` wrapping `client.login` or `client.signup`. 401 → "Wrong email or password", 409 → "That email is already registered", anything else → "Something went wrong." On success the server sets an HttpOnly session cookie and the client navigates to `safeNext(params.next)`. `safeNext` enforces same-origin absolute paths only: rejects missing, relative, protocol-relative `//`, backslash `/\`, or scheme-carrying targets.

### Build metadata (`lib/buildMeta.ts`)

`appVersion()` and `gitCommit()` read from `VITE_APP_VERSION` and `VITE_GIT_COMMIT` env vars baked at build time. Consumed by the BFF OTEL SDK's `serviceVersion` and `vcs.ref.head.revision` resource attributes, and by the `AppVersion` component.

### Lobby poll and table lifecycle (`lobbyPoll.ts`, `lib/lobbyStore.ts`)

`lobbyPollFamily(tableId)` is an `Atom.fn` that polls lobby state and stops when `started`. `startLobbyPoll` kicks the poll. `lobbyStore.ts` holds reactive lobby state for multi-seat coordination. Once the lobby moves to `started`, `Play` (`components/organisms/play.tsx`) transitions from the lobby view to the `Board` component, updating the document title with the table id.

---

## Implementation Decisions

- **Effect atoms, not `createResource`.** No `createResource(() => run(…))`, no manual fiber lifecycle in components. This gives consistent error folding, automatic fiber interruption on unmount, and shared state across screens without prop drilling.
- **`meAtom` folds all failures to `null`.** Any 401, decode error, or transport failure during `client.me()` is "not signed in" — mirrors the guard's semantics. The guard refreshes on mount to avoid a stale cached `null` right after login.
- **Deck-builder search is server-side.** The client holds no full catalog. `GET /cards/search` with tokenized LIKE over `search_blob` (includes `otags`) on the server (accounts-decks-and-catalog spec). The pool grid pages in 100-card chunks via IntersectionObserver — no client-side filtering of a local dataset.
- **Printing is art-preference only.** Card rules identity is the oracle id. Decks store `(id, count, print)` with `print` required. The engine is print-agnostic. Wire DTOs carry `print` for consistent art across all clients.
- **`safeNext` is checked both in-browser and server-side.** Open-redirect mitigations are layered: the client validates before navigation; the server validates before the session redirect.
- **Faro unsampled span ignore.** Faro's tracing sampler often marks sessions `NOT_RECORD` while the fetch instrumentation still injects a `traceparent` for the non-recording span. Parenting BFF spans under an unsampled span leaves Tempo with `<root span not yet received>` orphans. The BFF rejects inbound `traceparent` where `traceFlags & 0x01 === 0`.
- **No `@apply`, no `@layer components`.** The design system's component surfaces are Solid wrappers in `components/atoms/`; classes carry styling, `style` carries only CSS variable data. This is the Tailwind client-shell-deck-builder-and-observability spec house rule.
- **Biome unsafe fix for class sorting.** `nursery/useSortedClasses` is at error; the automated fix is `--unsafe` because Biome cannot verify that reordering utility classes is semantically safe. Do not add class sorting to `lint:fix`; fix manually or via the editor action.
- **Gzip LZ77 benefit from sorted classes.** client-shell-deck-builder-and-observability spec notes that consistent Tailwind class ordering makes repeated utility sequences longer LZ77 matches under gzip on the shipped JS/HTML.
- **`VITE_CARD_CDN` is build-time baked**, not runtime. Changing CDN requires a new image build.
- **No Scryfall fallback in production.** Missing CDN art is a broken image. This is a deliberate choice (client-shell-deck-builder-and-observability spec) to avoid rate-limiting Scryfall in production.

---

## Testing Decisions

- `client/src/atoms.test.ts` — shared atom wiring (`meAtom`, `decksAtom`).
- `client/src/lib/apiUpstream.test.ts`, `lib/apiUpstreamAuth.test.ts` — BFF proxy helpers.
- `client/src/effect/client.test.ts`, `effect/api.test.ts`, `effect/api-endpoints.test.ts` — Effect HTTP client (stubbed fetch).
- `client/src/effect/stream.test.ts` — delta stream atom behavior.
- `client/src/effect/otel.parent.test.ts` — trace parent continuation and sampled/unsampled filtering.
- `client/src/lib/faroCollect.test.ts`, `lib/faroSession.test.ts` — Faro proxy helpers and session repair.
- `client/src/lib/deckBuilderPrint.test.ts` — printing preference reconciliation.
- `client/src/lib/deckImagePreload.test.ts` — deck art preload logic.
- `client/src/lib/scryfall.test.ts` — CDN URL construction, Scryfall API fallback.
- `client/src/lib/traceContext.test.ts` — W3C `traceparent` parse/format.
- `client/src/lib/buildMeta.test.ts` — version/commit env var reading.
- `client/src/lib/lobbyStore.test.ts`, `lib/lobby.test.ts`, `lobby.test.ts` — lobby state and poll.
- `client/src/store.test.ts` — game fold: `applyDelta`, `applySnapshot`, `extractProvenance`.
- `client/src/net.test.ts` — `parseTableCode`, `tableId`, `setTableUrl`.
- `client/src/wire/grpcClient.test.ts`, `wire/protoMap.test.ts`, `wire/rpcServer.test.ts` — BFF gRPC client and proto mapping.
- `client/src/plugins/runtime.test.ts` — Nitro plugin runtime.
- `client/src/lib/lookupCards.test.ts` — card lookup for deck hydration.
- `client/src/components/atoms/button.test.ts`, `lib/cn.test.ts` — atom/component helpers.
- Integration test: `just client-check` runs Biome lint + typecheck + Vitest. The full check is `just check` (server + client).

---

## Out of Scope

- Server-side rendering of board state (SSR is disabled, `ssr: false`).
- Progressive Web App (PWA) / service worker / offline mode.
- Sitemaps, SEO meta, or marketing pages (`robots.txt` disallows all crawlers).
- Multi-account switching within one browser session.
- OAuth / social login (email+password only).
- Grafana Faro public dashboard (operator-only via port-forward).
- Cross-browser fallback for `navigator.clipboard` beyond the existing try/catch reveal pattern.

---

## Further Notes

- **DESIGN.md is the token source.** Adding a token to `global.css` that does not exist in DESIGN.md is drift. The two must stay in sync.
- **Effect / `@effect/atom-solid` / `@effect/sql-pg` must be pinned to the same exact beta.** Breaking the pin causes runtime type mismatches between Effect fibers from different versions.
- **Wire codegen.** `.proto` is the sole contract (wire-protocol-and-visibility spec). After proto changes: `just server-codegen` / `bun run gen` to regenerate the gitignored `client/lib/wire/generated/` directory. The BFF gRPC client imports from there.
- **Faro size cap.** `FARO_MAX_BODY_BYTES = 512 KiB`. The `/api/faro/collect` route rejects oversized payloads with 413 before reading the full body (`contentLengthTooLarge` checks `Content-Length` header first; `readBodyCapped` streams and checks).
- **Safe area insets.** The landscape rule applies to notched devices — `viewport-fit=cover` with safe-area insets. The portrait gate handles the notched-portrait case; landscape layout tightens padding but does not re-stack.
- **`just client-check`** is the canonical verification: Biome format + lint (including sorted-class check) + TypeScript typecheck + Vitest. Always run before committing client changes.
- **The lobby is on `mtgfr_web`** (Nitro BFF / Drizzle / Postgres `mtgfr_web`), not `mtgfr` (Toasty / game/user data). `just client-migrate` applies Drizzle migrations; `just migrate` applies Toasty migrations. Both must run before DB-touching work.
- **Client stack is Foldkit + Nitro** as of the 2026-07-21 cutover — the SolidStart / Vinxi shell described in this spec's "Implementation Decisions" was replaced by the Foldkit event-reactor SPA. See the [Foldkit migration design](2026-07-21-foldkit-client-migration-design.md) for the current module split (`client/app/`, `client/lib/`, `client/server/`).
