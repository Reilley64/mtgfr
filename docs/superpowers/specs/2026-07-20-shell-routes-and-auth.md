# Shell Routes and Auth

**Status:** Current (as of 2026-07-26)
**Module:** `client/app/` (entry, routes, update/view, model, subscriptions, resources, `pwa.ts`, `sw.ts`), `client/app/shell/frame/shell-frame.ts`, `client/app/shell/auth/**`, `client/app/faro.ts`, `client/app/domain/rpc-client.ts`, `client/app/domain/wire/**`, `client/app/domain/build-meta.ts`, `client/app/domain/client-build-options.ts`, `client/app/domain/design-tokens.generated.ts`, `client/app/domain/ui/**`, `client/styles/global.css`, `client/styles/tokens.generated.css`, `vite.config.ts`

---

## Problem Statement

The game client needs more than a board. Before reaching the canvas a player must authenticate with an account and navigate between shell surfaces. The client must handle authentication state across routes, enforce device orientation requirements, wire the browser to the BFF without leaking private game state on the wire edge, and follow a coherent design system built from a single token source.

These concerns — routing, auth, Foldkit state/effects, the high-level wire/BFF edge, game-stream fold wiring, design tokens, and build tooling — compose the app shell that the board and all other screens live inside. Deck list/builder behavior lives in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md); lobby Host/Join and seated chrome live in [lobby-entry-ui](2026-07-20-lobby-entry-ui.md); browser/BFF/API observability ops live in [observability-ops](2026-07-20-observability-ops.md).

---

## Solution

The client is a **Foldkit** SPA on **Nitro** (Vite). A single event-reactor owns all routes (`client/app/`: `Model` / `Message` / `update` / `view` with shell submodels). Async/wire work uses Effect at runtime boundaries (`client/app/domain/rpc-client.ts`, streams, BFF); Foldkit owns UI state. The wire contract is a hand-written Effect HTTP client over the same-origin `/api/rpc` BFF, which dials tonic gRPC. Design tokens are authored in `design.tokens.json` (DTCG) and generated under `bun run gen` into Tailwind v4 `@theme` (`client/styles/tokens.generated.css`) and canvas exports (`client/app/domain/design-tokens.generated.ts`). Vite also ships an installable-only PWA surface through `vite-plugin-pwa`: a generated manifest plus a checked-in network-only service worker registered once from app boot. Biome handles format/lint. Observability: Grafana Faro (browser) + `@effect/opentelemetry` (BFF) + OTLP/tonic (API) — see [observability-ops](2026-07-20-observability-ops.md); exporters no-op locally unless OTLP is set.

---

## User Stories

- As a new player, I visit the root URL, see the deck list, and am redirected to `/login` because I have no session. After signing up, I return to the deck list.
- As a player on a portrait phone, I see the landscape-first layout rotated in place via CSS; the deck builder and board stay usable without a dialog or vertical reflow.
- As a returning player, I sign in on `/login` and am sent to the validated `?next=` path (or a safe default) without open-redirect risk.

---

## Behavior

### App shell and routing (`client/app/routes.ts`, `client/app/view.ts`)

A single Foldkit event-reactor owns routing: `client/app/routes.ts` maps paths to shell views; `client/app/view.ts` renders the active route. Auth-gated routes consult the auth submodel (redirect to `/login?next=…` when unsigned-in). No persistent nav chrome. Non-board routes render through `shellFrame` (Landscape Rule applies at the app root). Routes:

| Path | View | Guard |
|---|---|---|
| `/` | Decks list | auth submodel |
| `/login` | Auth | — |
| `/leaderboard` | Leaderboard | auth submodel |
| `/coverage` | Coverage-by-set page | auth submodel |
| `/decks/new` | Deck builder | auth submodel |
| `/decks/:id` | Deck builder (edit) | auth submodel |
| `/play/:deckId` | Lobby Host/Join entry for a required deck id | auth submodel |
| `/play/:deckId/:table` | Pregame lobby for a required deck id and table id | auth submodel |
| `/play/:table` | Table-scoped lobby / board wrapper for a generated table code containing at least one letter | auth submodel |
| `/api/rpc/[...path]` | Effect RPC BFF | — |
| `/api/faro/collect` | Faro proxy | — |

Required identifiers live in path params ([wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) routing rule). Query params are optional: `?next=` is the post-login redirect target.

### BFF lobby and meta HTTP (`client/server/routes/api/**`, `client/server/lobby-http.ts`)

Lobby and meta HTTP are one Nitro route file per operation. Each handler exports `defineHandler` from `nitro/h3` inline; shared auth, tracing, and JSON helpers live in `client/server/lobby-http.ts`.
Route files stay at the Nitro Promise boundary (`export default defineHandler(async …)`), but pass Effect bodies that yield `Response` values into `withLobbyAuth` / `runMetaGet`. `withLobbyAuth` handles the session cookie, `grpcRequestEnv`, `fetchMe`, span annotation, idle lobby sweep, and `WebDbLive` provisioning inside the traced Effect; table route bodies yield lobby-store Effects directly instead of calling `runWebDb`.

| Route | File |
|---|---|
| `GET /api/meta/health/v1` | `api/meta/health/v1.get.ts` |
| `GET /api/meta/version/v1` | `api/meta/version/v1.get.ts` |
| `GET /api/meta/coverage/v1` | `api/meta/coverage/v1.get.ts` |
| `POST /api/tables/v1` | `api/tables/v1.post.ts` |
| `GET /api/tables/{table}/lobby/v1` | `api/tables/[table]/lobby/v1.get.ts` |
| `DELETE /api/tables/{table}/route/v1` | `api/tables/[table]/route/v1.delete.ts` |
| `POST /api/tables/{table}/join/v1` | `api/tables/[table]/join/v1.post.ts` |
| `POST /api/tables/{table}/ready/v1` | `api/tables/[table]/ready/v1.post.ts` |
| `POST /api/tables/{table}/start/v1` | `api/tables/[table]/start/v1.post.ts` |

Join, ready, and start take the required `{table}` id in the path; bodies carry only operation fields (`deck_id`, `ready`, or empty).
Bare `/play` and `?deck=` entry points are Not found (hard cut). Single-segment `/play/...` paths are discriminated by segment shape: integer-looking segments normalize to `PlayRoute` deck entry, and table codes normalize to the table-scoped in-game route. Minted lobby table codes are six characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ` and are regenerated until they contain at least one letter, so generated share codes never collide with numeric deck ids.

### Installable PWA (`client/app/pwa.ts`, `client/app/sw.ts`, `vite.config.ts`)

The shell is installable but not offline-capable. `vite-plugin-pwa` generates the manifest with `edh.reilley.dev` branding, `/` scope/id/start URL, standalone display, `#0B1310` theme/background colors, and the checked-in `pwa-192.png`, `pwa-512.png`, and Apple touch icon assets from `client/public/`. App boot calls `registerPwa()` once from `client/app/entry.ts`; service worker registration is not modeled as a Foldkit message.

The checked-in worker is intentionally network-only. Its `fetch` handler always does `fetch(event.request)` and does not precache app assets, install an offline document fallback, or define runtime caching for `/api`, `/api/rpc`, or live game streams. `vite.config.ts` keeps `injectManifest.globPatterns` empty, disables the inject-manifest precache injection point, and leaves `devOptions.enabled` false so local Vite dev never installs a development worker by surprise.

### App module layout (`client/app/messages.ts`, `client/app/domain/`, feature `index.ts`)

Foldkit features live under `client/app/shell/**`, `client/app/board/**`, and `client/app/game/**`. Shared wire, RPC, i18n, UI helpers, and other non-TEA utilities live in `client/app/domain/` (sibling to features; `client/server/` and `client/styles/` stay outside the app TEA tree).

The parent `Message` union in `client/app/messages.ts` carries app-level tags (`UrlChanged`, `ReceivedMeGravatarHash`, …), shared shell ticks (`CardArtTick`, `DeckCardFlipTick`, `ModalOpened`, account-chrome messages), and **`Got*Message` wrappers only** for child submodels — not flattened child tags. Wrappers: `GotAuthMessage`, `GotDeckListMessage`, `GotDeckBuilderMessage`, `GotCoverageMessage`, `GotLeaderboardMessage`, `GotLobbyMessage`, `GotBoardMessage`, `GotGameMessage`.

Each shell feature exports a namespace from `index.ts` (`shell/auth`, `shell/decks/list`, `shell/decks/builder`, `shell/coverage`, `shell/leaderboard`, `shell/lobby`). Views dispatch child messages as `message => GotXMessage({ message })`. Child commands and subscriptions lift back through `Command.mapMessages(cmds, m => GotXMessage({ message: m }))`. The board submodel's `toParentMessage` wraps into `GotBoardMessage`, except shared shell ticks such as `CardArtTick` from `BindCardArt` / `cardArt` (passthrough like deck list and lobby) — wrapping those throws because they are not `BoardMessage` variants and would tear down art mounts.

Route entry and post-session cold-load call per-surface **`informRouteChanged`** helpers (`shell/*/inform.ts`) so the child owns reset/load transitions; the parent still owns auth redirects and lobby-driven game-slice activation after the lifted child fold.

Lobby Host/Join and seated chrome for `/play/:deckId`, `/play/:deckId/:table`, and `/play/:table` are specified in [lobby-entry-ui](2026-07-20-lobby-entry-ui.md). Deck list (`/`) and builder (`/decks/…`) are specified in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md). The home route entry loads decks only; it does not issue a separate top-players teaser fetch. The leaderboard route (`/leaderboard`) renders a ranked list from `rpc.ratings.leaderboard({ limit, offset })`, showing rank (display emphasis), username, and rating (vine, not priority gold) for authenticated players, with header chrome that keeps `Play` back to `/` and reuses the shared avatar account menu instead of a standalone sign-out button. Route entry loads the first page as `limit = 50, offset = 0`; `Load more` (`leaderboard-load-more`) appends the next page. When a later page load fails after prior rows are already visible, the existing rows stay on screen, `Load more` is hidden, and `Try again` (`leaderboard-try-again`) clears the current rows and restarts from the first page.

The coverage route (`/coverage`) loads `LobbyClient.coverageMeta()` from `GET /api/meta/coverage/v1` on route entry through `Coverage.informRouteChanged` / `GotCoverageMessage`, and renders a searchable set-completeness table for authenticated users. The shell header shows `Coverage`, a subtitle global `% faithful` line (`coverage-global-percent`), a `Play` link back to `/`, and the shared avatar account menu with the `Leaderboard` shortcut kept visible. Rows filter by lowercase match on set code or name, sort by release date descending with null release dates last, and show `Set`, `Faithful`, `Scryfall`, and `%` columns. Percentage formatting reuses the shared badge formatter; rows with missing `oracle_total` show `—` instead of inventing a denominator or percent, and the global header follows the same rule when either global count is missing. `Try again` restarts the coverage load from an empty loading state after errors, while the error alert stays visible and the search input remains available outside the loading state. Every shell surface that already showed the fixed bottom-left API badge now uses the shared two-line stack: `{n}% faithful` above `API {version}` when both coverage counts are present; when either coverage count is missing or invalid, the shell renders only the version line. The board remains out of scope for this chrome. When coverage meta is complete, the badge `% faithful` line links to `/coverage`. Detailed page behavior and the BFF/API join contract live in [coverage-by-set](2026-07-26-coverage-by-set.md).

### Shell frame (`client/app/shell/frame/shell-frame.ts`)

Auth, deck list, deck builder, lobby, leaderboard, and coverage routes render through `shellFrame`: full-bleed felt atmosphere (`shell-atmosphere-auth` or `shell-atmosphere-shell`), a three-column header (`shell-header-leading` / title / `shell-header-trailing`), optional subtitle (string or Html), a centered stage (`shell-stage` with `shell-stage-enter`), and the shared `% faithful` + `API {version}` badge. The frame root is a viewport-contained flex column (`fixed inset-0 flex flex-col overflow-hidden`); the header is `shrink-0` and the stage is `flex-1 min-h-0`. List/lobby/auth pages leave the stage as `overflow-y-auto`; builder and coverage pass `lockStageScroll` so the stage is `overflow-hidden` and only their inner hosts (catalog / table body) scroll. Auth passes no shell header title; its stage renders the `edh.reilley.dev` wordmark as a display hero above the auth mode title (`font-display`), while the auth panel itself contains only the fields, submit action, in-form error (`auth-error` via `alertClass`), and mode toggle. Lobby passes `Lobby` as the shell header title and its stage renders only the Host/Join or table body, with no inner wordmark/hero. Deck builder places Cancel in leading and Save in trailing before account chrome. Coverage places the global `% faithful` line in the header subtitle. Shell body text uses `font-shell` (Manrope); route titles and stage title beats use `font-display` (Space Grotesk). Board routes bypass `shellFrame` and mount the board submodel directly. Inline shell alerts reuse `alertClass` from `client/app/domain/ui/surfaces.ts` (auth/lobby legality overrides with burn-red; start-gate copy stays caution amber). Under `prefers-reduced-motion: reduce`, `.shell-stage-enter` sets `animation: none` and lobby entry swap motion does not run.

### Landscape rotate (`client/app/view.ts`, `client/app/subscriptions.ts`, `client/styles/global.css`, DESIGN.md Landscape Rule)

When `(orientation: portrait) and (max-width: 900px)` matches, the app root (`data-testid="landscape-root"`) gets class `landscape-rotate-root`. CSS swaps width/height, rotates the subtree 90°, and applies best-effort `env(safe-area-inset-*)` padding so landscape-first layouts stay side-by-side without a dialog, portrait reflow, or notch clipping. A Foldkit subscription listens to `matchMedia` changes and dispatches `LandscapeRotateChanged`; boot seeds `landscapeRotate.active` from the same query (`client/app/init.ts`, `isPortraitPhone()` in `client/app/subscriptions.ts`). Every route — shell and board — lives under the rotate root when active.

### Auth guard (`client/app/update.ts`, `client/app/shell/auth/**`)

`FetchMe` is a Foldkit command wrapping `client.me()` with all failures folded to `null` — any 401, decode error, or transport failure is treated as "not signed in." Route entry runs session checks for protected routes. While the session is unresolved, protected content stays blank; once resolved to `null`, the app redirects to `/login?next=<current-path>`. The `next` redirect target is validated server-side and in-browser: only same-origin absolute paths starting with `/` (not `//` or `/\`) are accepted.

The `/login` route renders the auth screen through a Foldkit submodel boundary. Child auth messages lift into the app through `GotAuthMessage`, and the parent keeps session ownership by inspecting `ReceivedMe` after the child auth update runs. Child auth commands are lifted back to app messages with Foldkit `Command.mapMessages`, so auth remains an isolated submodel while session redirects and `HashMeGravatar` stay parent-owned.

When `ReceivedMe` carries a signed-in user, the app queues `HashMeGravatar`, a Foldkit command that SHA-256 hashes the user's email through `client/app/domain/gravatar.ts` and stores the result as `session.meGravatarHash`. The completion message includes the source email, and `update` ignores stale hash results when the current session email no longer matches. Signed-out sessions clear `meGravatarHash`. The resulting hash feeds the shared account chrome face on home and leaderboard, matching the seat/avatar helper family without exposing raw email in UI state.

Unsigned protected content never renders.

### Foldkit state and effects (`client/app/model.ts`, `client/app/update.ts`, `client/app/subscriptions.ts`, `client/app/resources.ts`)

The app model is the single UI state tree. `update(model, message)` is the only state transition point and returns `[Model, Command[]]`. Shell submodels own auth, deck list, deck builder, coverage, leaderboard, and lobby state; the board owns board interaction state while game deltas fold into `client/app/game/fold.ts`. Auth, deck list, deck builder, coverage, leaderboard, and lobby child updates all cross the parent boundary through `Got*Message` wrappers. Route entry and post-session cold-load re-entry call each surface's `informRouteChanged` helper so the child owns its reset/load transition while the parent still owns auth redirects and lobby-driven game-slice activation.

Async work is expressed as Foldkit **Commands** backed by Effect programs. Commands depend on resources from `client/app/resources.ts`: `RpcClient` for Effect RPC calls and `LobbyClient` for same-origin lobby/meta HTTP. Session checks, auth submit, deck loading, catalog search, deck save/delete, leaderboard loading, coverage loading, API-version metadata, lobby host/join/ready/start, and table navigation all flow through commands.

Boot also fetches `/api/meta/version/v1` through the `FetchApiVersion` Foldkit command and `LobbyClient.apiMeta()`. The client decodes the required app `version` plus optional `faithful_count` / `oracle_total` fields from the BFF meta response. The app model stores all three values (`apiVersion`, `faithfulCount`, `oracleTotal`) and threads them into shell views as shared `AppChromeMeta`. Tagged lobby/meta transport failures fold to null metadata, so the version line is omitted until a successful response and the `% faithful` line is omitted when coverage fields are incomplete. `LobbyClient.coverageMeta()` also decodes `GET /api/meta/coverage/v1` nullable global counts plus per-set rows (`code`, `name`, `released_at`, `faithful`, `oracle_total`) into camelCase shell data for the `/coverage` page.

Long-lived listeners are Foldkit **Subscriptions**. App subscriptions cover portrait orientation, lobby polling, and game stream frames. Dependency functions decide when each stream is active; returning `Stream.empty` stops work when the route or table changes. Components do not own long-lived fibers.

### Wire protocol (high-level; detail in [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md))

Modules: `client/app/domain/rpc-client.ts`, `client/server/routes/api/rpc/[...path].ts`, `client/app/domain/wire/grpcClient.ts`.

The browser talks only to the same-origin BFF via the hand-written Effect HTTP client (`client/app/domain/rpc-client.ts`) over `/api/rpc`. The Nitro BFF dispatches `/api/rpc/**` requests and calls tonic gRPC through `client/app/domain/wire/grpcClient.ts`. There is no direct browser-to-gRPC communication. The proto wire is the sole contract.

The `/api/rpc/[...path]` route opens exactly one `runTracedRequest` runtime edge around Effect
dispatch. `dispatchRpc` returns an Effect that resolves to `RpcOutcome`, including mapped gRPC
errors; the route sets or clears the HttpOnly session cookie imperatively only after that Effect
finishes. For in-game methods, `dispatchRpc` resolves the owning pod through `resolveTableAddress`,
which runs the Effect-native `lookupTableRoute` store program via `runWebDb` (`client/server/db/client.ts`)
— a pooled `ManagedRuntime` over the `WebDb` Drizzle `effect-postgres` service on `mtgfr_web`.

`makeClient(fetch)` accepts a fetch implementation so tests can stub it. `client` is the app singleton (credentials: include, prepended `/api/rpc`). Wire types (`wire/types.ts`) are Effect Schema-decoded DTOs; `wire/protoMap.ts` maps them to/from proto.

### Game delta stream (high-level; board surfaces own paint)

Modules: `client/app/game/stream-subscription.ts`, `client/app/game/fold.ts`.

The game stream is a Foldkit subscription keyed by route table id and active game table id. It opens only when the app is on a table-scoped play route (`/play/:deckId/:table` or `/play/:table`) and the game slice is active. When lobby start flips the seated pregame route into `/play/:table`, the same table id continues to key the live board, so the URL strip does not require a second table lookup or a deck segment. Snapshot and delta frames become messages, then `update` folds them through `applySnapshotPure` / `applyDeltaPure`. `model.game.connected` drives the reconnect banner; rejected intents set `game.reject` and `board.reject`. The subscription goes empty after navigation or table mismatch, so no residual stream continues after leaving the board.

### Design system (`DESIGN.md`, `design.tokens.json`, `client/styles/global.css`)

`design.tokens.json` is the **single source of truth** for design token values, authored as DTCG 2025.10-aligned tokens. Token prose and rules live in [`DESIGN.md`](../../../DESIGN.md); the DTCG architecture design is historical input in [dtcg-token-architecture-design](2026-07-27-dtcg-token-architecture-design.md). The token file uses `primitive` and `semantic` tiers: primitives hold OKLCH source decisions and spacing, while semantic tokens keep the public CSS/Tailwind/canvas names and point at primitives or other semantics through DTCG aliases. `bun run gen` (Style Dictionary) resolves aliases, emits CSS Color 4 `oklch(...)` strings to `client/styles/tokens.generated.css` (Tailwind v4 `@theme`), emits matching canvas constants and typed structures to `client/app/domain/design-tokens.generated.ts`, and emits narrow `hexFallbacks` for meta/PWA/favicon surfaces that still require hex. The token source does not use `$type: "css"`; shadows, cubic beziers, durations, and typography are typed composites. Shadow layers author explicit `spread`, but generated CSS omits zero spread as the default. Typography composites omit unused sub-values that codegen treats as defaults. `global.css` imports generated theme output and keeps hand-authored keyframes/interaction rules. Foldkit HTML helpers and shared UI helpers in `client/app/domain/ui/` own component recipes — never via `@apply`, and not as token component maps. Inline style is used only for CSS variables; classes carry appearance. Arbitrary values (`bg-[#18221ef5]`) are for one-off values that token files do not name; they do not extend the token list.

Key semantic tokens:
- `forest-floor` — canvas background, `index.html` inline background, and generated `hexFallbacks.forestFloor` for meta/PWA/favicon fill (prevents flash).
- `forest-surface` — panels.
- `forest-hud` — HUD panels.
- `llanowar` / `llanowar-deep` — primary buttons (hover → active).
- `priority-gold` — priority orb. **Gold = a decision is owed** (The Gold Means Act Rule).
- `playable-border` — alias of `snow-mint`, used when a card has a current action.
- `vine` — active borders.
- Seat colors: `seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane` — player identity, never semantics.
- Combat semantics: `mountain-red` (attack), `wall-green` (block), `island-blue` (targeting).

Shell typography uses Manrope (`font-shell`) and Space Grotesk (`font-display` for titles); board HUD and canvas chrome use `font-sans` (`system-ui`). Screen ramp: `title` 18/700, `body` 14/400, `button-label` 14/600, `label` 13, `caption` 12, `game` 15/600, `display` 22/700. HUD density: `chip` 11, `micro` 10 (board/hand chrome only). Rounded corners: `panel` 12px, `modal` 10px, `game` 10px, `hud` 8px, `control` 6px, `focus` 4px.

The `mana-oracle.css` import brings in the mana-font glyph subset (icon font, not body text). A custom `@font-face` overrides the mana-font package to prefer woff2 for canvas `ctx.fillText`. Mana pips in oracle text use `ms.ms-oracle` with `font-size: 0.78em` so pips don't dominate the body line.

### Brand display

Player-facing wordmark and document title use **`edh.reilley.dev`** (lowercase hostname, no scheme). Scryfall and related tooling HTTP User-Agent identity is **`edh.reilley.dev/0.1`** (call sites include `client/app/domain/deck-builder/scryfall.ts` and tooling scripts). Surfaces that show the wordmark include HTML `<title>`, Foldkit `Document.title` / nav brand link (`client/app/view.ts`), and the auth stage hero. Lobby uses the shell header title `Lobby` and does not render an inner brand wordmark. Package names, database names (`mtgfr` / `mtgfr_web`), proto package, GHCR image names, and similar infrastructure identifiers are not renamed as part of this brand display (see Further Notes).

The site favicon is a filled `forest-floor` circle whose hex fill matches generated `hexFallbacks.forestFloor`, with a closed-mouth elder-dragon head-and-neck bust cut out as transparent negative space (side profile, facing right; neck base planted on the bottom rim) — GitHub Invertocat-style, not a lettermark and not a square plate. Source of truth is `client/public/favicon.svg`; `client/public/favicon.ico` is a multi-size alpha raster fallback. Install surfaces derive `client/public/pwa-192.png`, `client/public/pwa-512.png`, and `client/public/apple-touch-icon.png` from the same dragon-on-disc art family. `client/index.html` declares `<meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover" />`, a `theme-color` whose content matches `hexFallbacks.forestFloor`, `<link rel="apple-touch-icon" href="/apple-touch-icon.png" />`, `<link rel="icon" href="/favicon.svg" type="image/svg+xml" />`, then `<link rel="icon" href="/favicon.ico" sizes="any" />`; the Vite PWA manifest uses the same fallback for `theme_color` and `background_color`.

### Biome

Biome 2.5.3 handles format, lint, and import ordering (`assist.actions.source.organizeImports`, `sortBareImports: true`). `nursery/useSortedClasses` is at error for Tailwind class sorting and configured for safe fixes over `cn` / `clsx`. CSS: `tailwindDirectives: true`. The `test` domain is recommended.

### Observability (pointer)

Browser Faro, BFF OTEL (`client/server/plugins/otel.server.ts`), scrub rules, Faro body caps, and LGTM operator access are specified in [observability-ops](2026-07-20-observability-ops.md) (cluster topology context in [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)).

### Auth UI (`client/app/shell/auth/view.ts`, `client/app/shell/auth/update.ts`)

Single-page login/signup (toggled, not separate routes). `Login` and `Signup` are Foldkit commands wrapping `client.login` / `client.signup`. 401 → "Wrong email or password", 409 → "That email is already registered", anything else → "Something went wrong." On success the server sets an HttpOnly session cookie and the client navigates to `safeNext(params.next)`. `safeNext` enforces same-origin absolute paths only: rejects missing, relative, protocol-relative `//`, backslash `/\`, or scheme-carrying targets.

### Build metadata (`client/app/domain/build-meta.ts`, `client/app/domain/ui/app-version.ts`)

`appVersion()` and `gitCommit()` read from `VITE_APP_VERSION` and `VITE_GIT_COMMIT` env vars baked at build time. Consumed by the BFF OTEL SDK's `serviceVersion` and `vcs.ref.head.revision` resource attributes, and by the `AppVersion` component.

Bottom-left shell chrome (`appVersionBadge`): when `apiVersion` is known, show `API {version}` (`data-testid="app-version"`). When `faithfulCount` and `oracleTotal` are also known and `oracleTotal > 0`, show `{n}% faithful` on the line above (`data-testid="pool-coverage"`). When coverage meta is complete, `pool-coverage` is an `<a href="/coverage">` (`coverageHref` from `AppChromeMeta`); the version line stays non-interactive. The outer stack keeps `pointer-events-none` (`appVersionClass`) with `pointer-events-auto` on the link. Percentage uses one decimal below 10%, otherwise whole percent (`formatFaithfulPercent`). Coverage comes from `GET /api/meta/version/v1` (`faithful_count` from API `/health/live`, `oracle_total` from BFF-cached Scryfall oracle-cards JSONL count, 24h TTL, non-blocking refresh). Incomplete coverage → version line only. Not shown on the in-game board.

### Production source maps (`vite.config.ts`, `client/app/domain/client-build-options.ts`)

Vite production builds set `build.sourcemap: true` (via `clientBuildSourcemap`) so the large first-party client bundle ships a sibling `.js.map` with a `//# sourceMappingURL=` comment and embedded `sourcesContent`. Chrome DevTools and Faro can resolve minified frames without a separate map-upload pipeline. Maps are public static assets under `.output/public/assets/` (same as the JS); `"hidden"` is intentionally not used because browsers only auto-fetch maps when the comment is present.

---

## Implementation Decisions

- **Foldkit `update` is the state boundary.** UI state changes only through messages handled by `client/app/update.ts` and shell child updates. Async work returns messages through Foldkit commands and subscriptions, which gives consistent error folding, runtime resource injection, and automatic stream teardown.
- **Shell children lift through wrappers, not raw parent tags.** Auth, deck list, deck builder, coverage, leaderboard, and lobby commands/subscriptions map back through `Got*Message`, and route entry uses per-surface `informRouteChanged` helpers instead of mutating child slices directly from the parent. Parent-owned redirects and game activation still happen after the lifted child fold.
- **`FetchMe` folds all failures to `null`.** Any 401, decode error, or transport failure during `client.me()` is "not signed in" — mirrors the guard's semantics. Route entry refreshes session state for protected routes to avoid stale login redirects.
- **`safeNext` is checked both in-browser and server-side.** Open-redirect mitigations are layered: the client validates before navigation; the server validates before the session redirect.
- **No `@apply`, no `@layer components`.** Foldkit views and shared UI helpers carry styling through Tailwind classes; inline style carries only CSS variable data. This is the Tailwind shell house rule.
- **Biome class sorting.** `nursery/useSortedClasses` is at error and configured for safe `cn` / `clsx` fixes. Keep class strings sorted in code review and use the editor or Biome fix path for drift.
- **Gzip LZ77 benefit from sorted classes.** Consistent Tailwind class ordering makes repeated utility sequences longer LZ77 matches under gzip on the shipped JS/HTML.
- **Public client source maps.** Production uses `build.sourcemap: true` (not `"hidden"`) so DevTools/Faro can deminify the large first-party bundle. Original TypeScript is fetchable alongside the asset; acceptable for this friend-group deployment without a private map store.
- **Service worker stays network-only.** Installability is in scope; offline play is not. Do not add precache or runtime caching without a fresh product decision because the authoritative game client still depends on live network state.
- **BFF RPC dispatch runs as one Effect.** `/api/rpc/[...path]` does method/body/cookie handling in
  Nitro, then runs `dispatchRpc` once through `runTracedRequest`; `dispatchRpc` performs gRPC calls
  as Effects and returns outcome values instead of throwing for normal gRPC failures.

---

## Testing Decisions

- `client/app/shell/auth/**/*.test.ts` — auth stories and helpers, including `ReceivedMe` → `HashMeGravatar` session storage and stale-result guarding.
- `client/app/update.test.ts` — parent-level regressions such as lifting auth child messages through `GotAuthMessage`.
- `client/app/routes.test.ts`, `client/app/smoke.test.ts` — routing and smoke; includes protected `/leaderboard` and `/coverage` entry, auth redirects, home entry loading decks without a teaser fetch, leaderboard retry-from-page-one behavior, numeric-vs-table single-segment `/play/...` discrimination, the landscape rotate class plus safe-area HTML/CSS contract, and coverage refresh/query/account-menu state (including post-failure `status: "error"`), with coverage child messages lifted through `GotCoverageMessage`.
- `client/app/shell/lobby/**/*.test.ts`, `client/app/shell/leaderboard/**/*.test.ts`, `client/app/shell/coverage/**/*.test.ts` — route-inform resets, wrapper-lifted parent folds (`GotLobbyMessage`, `GotLeaderboardMessage`, `GotCoverageMessage`), lobby redirect/game handoff, leaderboard retry/load-more state, and coverage sort/filter/`—` formatting.
- `client/app/shell/surfaces.test.ts` — shell Scene coverage for auth, deck, leaderboard, coverage, and lobby surfaces, including shared account chrome and the `% faithful` + `API {version}` shell badge stack; Scene asserts auth stage hero `auth-brand` outside `auth-panel`, `pool-coverage` above `app-version` when the model has complete meta, asserts `pool-coverage[href="/coverage"]`, and asserts `/coverage` renders the global percent, search field, row filtering/empty state, and retry error UI.
- `client/app/domain/ui/app-version.test.ts` — percent formatting and stacked badge rendering rules, including optional `coverageHref` link.
- `client/app/shell/coverage/view.test.ts` — coverage row sort/filter rules and `—` fallback when row or global counts are incomplete.
- `client/app/game/*.test.ts` — game fold, stream subscription.
- `client/app/domain/rpc-client.test.ts` — Effect HTTP client (stubbed fetch).
- `client/app/domain/wire/*.test.ts` — BFF gRPC / RPC method gate.
- `client/app/domain/ui/*.test.ts`, `client/app/domain/cn.test.ts` — Foldkit UI components and surface class helpers ([ui-component-layer](2026-07-28-ui-component-layer.md)).
- `client/app/domain/build-meta.test.ts` — version/commit env var reading.
- `client/app/domain/client-build-options.test.ts` — production `build.sourcemap` stays `true` and wired in `vite.config.ts`.
- `client/app/pwa-html.test.ts`, `client/app/sw.network.test.ts` — HTML install metadata plus source guards that keep the worker/config network-only; production `bun run build` emits the manifest and custom worker.
- Board geometry/paint/HTML tests live under `client/app/board/**` (see board specs / `docs/client-canvas-map.md`).
- Integration test: `just client-check` runs Biome lint + typecheck + Vitest. The full check is `just check` (server + client).

---

## Out of Scope

- Server-side rendering of board state (SPA on Nitro; no SSR of the board).
- Offline mode, precached app shell assets, service-worker runtime caching, and in-app install UI (browser-native install only).
- Sitemaps, SEO meta, or marketing pages (`robots.txt` disallows all crawlers).
- Multi-account switching within one browser session.
- OAuth / social login (email+password only).
- Deck list/builder UX detail (see [deck-list-and-builder](2026-07-20-deck-list-and-builder.md)).
- Lobby Host/Join and seated lobby chrome detail (see [lobby-entry-ui](2026-07-20-lobby-entry-ui.md)).
- Full observability plane ops (see [observability-ops](2026-07-20-observability-ops.md)).

---

## Further Notes

- **`design.tokens.json` is the token source.** Token values are authored there, then generated into `client/styles/tokens.generated.css` and `client/app/domain/design-tokens.generated.ts`; never hand-edit generated outputs. Design-system prose SoT remains [`DESIGN.md`](../../../DESIGN.md).
- **Brand non-rename.** Display wordmark and public User-Agent use `edh.reilley.dev`; DBs (`mtgfr`, `mtgfr_web`), proto (`mtgfr.v1`), GHCR images, K8s labels, npm/cargo package names, clap CLI name, Terraform example hostname (`edh.example.com`), localStorage keys, Faro/OTEL service names, and Style Dictionary format ids are not renamed for brand display alone.
- **Effect / `@effect/*` packages must be pinned to the same exact beta.** Breaking the pin causes runtime type mismatches between Effect fibers from different versions.
- **Wire codegen.** `.proto` is the sole contract ([wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md)). After proto changes: `just server-codegen` / `bun run gen` to regenerate the gitignored `client/app/domain/wire/generated/` directory. The BFF gRPC client imports from there.
- **Safe area insets.** The landscape rule applies to notched devices — `viewport-fit=cover` with safe-area insets. Portrait phones use CSS landscape rotate (no dialog); short landscape layout tightens padding but does not re-stack.
- **`just client-check`** is the canonical verification: Biome format + lint (including sorted-class check) + TypeScript typecheck + Vitest. Always run before committing client changes.
- **Live client architecture** is Foldkit + Nitro with `client/app/`, `client/app/domain/`, and `client/server/` as the module split.
- **Pool coverage badge design input:** [2026-07-26-pool-coverage-badge-design.md](2026-07-26-pool-coverage-badge-design.md).
- **Coverage by set design input:** [2026-07-26-coverage-by-set-design.md](2026-07-26-coverage-by-set-design.md).
