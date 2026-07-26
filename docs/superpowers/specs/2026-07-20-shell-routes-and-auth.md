# Shell Routes and Auth

**Status:** Current (as of 2026-07-26)
**Module:** `client/app/` (entry, routes, update/view, model, subscriptions, resources), `client/app/shell/auth/**`, `client/app/faro.ts`, `client/app/domain/rpc-client.ts`, `client/app/domain/wire/**`, `client/app/domain/build-meta.ts`, `client/app/domain/client-build-options.ts`, `client/app/domain/design-tokens.generated.ts`, `client/app/domain/ui/**`, `client/styles/global.css`, `client/styles/tokens.generated.css`, `vite.config.ts`

---

## Problem Statement

The game client needs more than a board. Before reaching the canvas a player must authenticate with an account and navigate between shell surfaces. The client must handle authentication state across routes, enforce device orientation requirements, wire the browser to the BFF without leaking private game state on the wire edge, and follow a coherent design system built from a single token source.

These concerns — routing, auth, Foldkit state/effects, the high-level wire/BFF edge, game-stream fold wiring, design tokens, and build tooling — compose the app shell that the board and all other screens live inside. Deck list/builder behavior lives in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md); lobby Host/Join and seated chrome live in [lobby-entry-ui](2026-07-20-lobby-entry-ui.md); browser/BFF/API observability ops live in [observability-ops](2026-07-20-observability-ops.md).

---

## Solution

The client is a **Foldkit** SPA on **Nitro** (Vite). A single event-reactor owns all routes (`client/app/`: `Model` / `Message` / `update` / `view` with shell submodels). Async/wire work uses Effect at runtime boundaries (`client/app/domain/rpc-client.ts`, streams, BFF); Foldkit owns UI state. The wire contract is a hand-written Effect HTTP client over the same-origin `/api/rpc` BFF, which dials tonic gRPC. Design tokens are authored in `design.tokens.json` (DTCG) and generated under `bun run gen` into Tailwind v4 `@theme` (`client/styles/tokens.generated.css`) and canvas exports (`client/app/domain/design-tokens.generated.ts`). Biome handles format/lint. Observability: Grafana Faro (browser) + `@effect/opentelemetry` (BFF) + OTLP/tonic (API) — see [observability-ops](2026-07-20-observability-ops.md); exporters no-op locally unless OTLP is set.

---

## User Stories

- As a new player, I visit the root URL, see the deck list, and am redirected to `/login` because I have no session. After signing up, I return to the deck list.
- As a player on a portrait phone, I see a native dialog telling me to rotate to landscape; the deck builder and board are hidden behind the dialog.
- As a returning player, I sign in on `/login` and am sent to the validated `?next=` path (or a safe default) without open-redirect risk.

---

## Behavior

### App shell and routing (`client/app/routes.ts`, `client/app/view.ts`)

A single Foldkit event-reactor owns routing: `client/app/routes.ts` maps paths to shell views; `client/app/view.ts` renders the active route. Auth-gated routes consult the auth submodel (redirect to `/login?next=…` when unsigned-in). No persistent nav chrome. Global chrome is the portrait gate (Landscape Rule). Routes:

| Path | View | Guard |
|---|---|---|
| `/` | Decks list | auth submodel |
| `/login` | Auth | — |
| `/leaderboard` | Leaderboard | auth submodel |
| `/decks/new` | Deck builder | auth submodel |
| `/decks/:id` | Deck builder (edit) | auth submodel |
| `/play/:deckId` | Lobby Host/Join entry for a required deck id | auth submodel |
| `/play/:deckId/:table` | Pregame lobby for a required deck id and table id | auth submodel |
| `/play/:table` | Table-scoped lobby / board wrapper for a non-numeric table id | auth submodel |
| `/api/[...path]` | lobby/table HTTP passthrough | — |
| `/api/rpc/[...path]` | Effect RPC BFF | — |
| `/api/faro/collect` | Faro proxy | — |

Required identifiers live in path params ([wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) routing rule). Query params are optional: `?next=` is the post-login redirect target.
Bare `/play` and `?deck=` entry points are Not found (hard cut). Single-segment `/play/...` paths are discriminated by segment shape: numeric segments normalize to `PlayRoute` deck entry, and non-numeric segments normalize to the table-scoped in-game route.

### App module layout (`client/app/messages.ts`, `client/app/domain/`, feature `index.ts`)

Foldkit features live under `client/app/shell/**`, `client/app/board/**`, and `client/app/game/**`. Shared wire, RPC, i18n, UI helpers, and other non-TEA utilities live in `client/app/domain/` (sibling to features; `client/server/` and `client/styles/` stay outside the app TEA tree).

The parent `Message` union in `client/app/messages.ts` carries app-level tags (`UrlChanged`, `ReceivedMeGravatarHash`, …), shared shell ticks (`CardArtTick`, `DeckCardFlipTick`, `ModalOpened`, account-chrome messages), and **`Got*Message` wrappers only** for child submodels — not flattened child tags. Wrappers: `GotAuthMessage`, `GotDeckListMessage`, `GotDeckBuilderMessage`, `GotLeaderboardMessage`, `GotLobbyMessage`, `GotBoardMessage`, `GotGameMessage`.

Each shell feature exports a namespace from `index.ts` (`shell/auth`, `shell/decks/list`, `shell/decks/builder`, `shell/leaderboard`, `shell/lobby`). Views dispatch child messages as `message => GotXMessage({ message })`. Child commands and subscriptions lift back through `Command.mapMessages(cmds, m => GotXMessage({ message: m }))`. The board submodel's `toParentMessage` wraps into `GotBoardMessage` (no identity passthrough).

Route entry and post-session cold-load call per-surface **`informRouteChanged`** helpers (`shell/*/inform.ts`) so the child owns reset/load transitions; the parent still owns auth redirects and lobby-driven game-slice activation after the lifted child fold.

Lobby Host/Join and seated chrome for `/play/:deckId`, `/play/:deckId/:table`, and `/play/:table` are specified in [lobby-entry-ui](2026-07-20-lobby-entry-ui.md). Deck list (`/`) and builder (`/decks/…`) are specified in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md). The home route entry loads decks only; it does not issue a separate top-players teaser fetch. The leaderboard route (`/leaderboard`) renders a ranked list from `rpc.ratings.leaderboard({ limit, offset })`, showing rank, username, and rating for authenticated players, with header chrome that keeps `Play` back to `/` and reuses the shared avatar account menu instead of a standalone sign-out button. Route entry loads the first page as `limit = 50, offset = 0`; `Load more` appends the next page. When a later page load fails after prior rows are already visible, the existing rows stay on screen, `Load more` is hidden, and `Try again` clears the current rows and restarts from the first page. Every shell surface that already showed the fixed bottom-left API badge now uses the shared two-line stack: `{n}% faithful` above `API {version}` when both coverage counts are present; when either coverage count is missing or invalid, the shell renders only the version line. The board remains out of scope for this chrome.

### Portrait gate (`client/app/view.ts`, `client/app/subscriptions.ts`, DESIGN.md Landscape Rule)

A native `<dialog showModal>` opens when `(orientation: portrait) and (max-width: 900px)` matches. A Foldkit Mount command defers `.showModal()` until the dialog is connected. Escape is swallowed (`OnCancel` prevents dismissal). The scrim covers the background inert. A Foldkit subscription listens to `matchMedia` changes and closes the gate automatically on landscape flip. It is mounted at the app root so every route is behind it.

### Auth guard (`client/app/update.ts`, `client/app/shell/auth/**`)

`FetchMe` is a Foldkit command wrapping `client.me()` with all failures folded to `null` — any 401, decode error, or transport failure is treated as "not signed in." Route entry runs session checks for protected routes. While the session is unresolved, protected content stays blank; once resolved to `null`, the app redirects to `/login?next=<current-path>`. The `next` redirect target is validated server-side and in-browser: only same-origin absolute paths starting with `/` (not `//` or `/\`) are accepted.

The `/login` route renders the auth screen through a Foldkit submodel boundary. Child auth messages lift into the app through `GotAuthMessage`, and the parent keeps session ownership by inspecting `ReceivedMe` after the child auth update runs. Child auth commands are lifted back to app messages with Foldkit `Command.mapMessages`, so auth remains an isolated submodel while session redirects and `HashMeGravatar` stay parent-owned.

When `ReceivedMe` carries a signed-in user, the app queues `HashMeGravatar`, a Foldkit command that SHA-256 hashes the user's email through `client/app/domain/gravatar.ts` and stores the result as `session.meGravatarHash`. The completion message includes the source email, and `update` ignores stale hash results when the current session email no longer matches. Signed-out sessions clear `meGravatarHash`. The resulting hash feeds the shared account chrome face on home and leaderboard, matching the seat/avatar helper family without exposing raw email in UI state.

Unsigned protected content never renders.

### Foldkit state and effects (`client/app/model.ts`, `client/app/update.ts`, `client/app/subscriptions.ts`, `client/app/resources.ts`)

The app model is the single UI state tree. `update(model, message)` is the only state transition point and returns `[Model, Command[]]`. Shell submodels own auth, deck list, deck builder, leaderboard, and lobby state; the board owns board interaction state while game deltas fold into `client/app/game/fold.ts`. Auth, deck list, deck builder, leaderboard, and lobby child updates all cross the parent boundary through `Got*Message` wrappers. Route entry and post-session cold-load re-entry call each surface's `informRouteChanged` helper so the child owns its reset/load transition while the parent still owns auth redirects and lobby-driven game-slice activation.

Async work is expressed as Foldkit **Commands** backed by Effect programs. Commands depend on the `RpcClient` resource from `client/app/resources.ts`, so wire access is explicit at the runtime boundary. Session checks, auth submit, deck loading, catalog search, deck save/delete, leaderboard loading, lobby host/join, and table navigation all flow through commands.

Boot also fetches `/api/meta/version/v1` through `client/app/domain/lobby/client.ts`. The `apiMeta()` helper decodes the required app `version` plus optional `faithful_count` / `oracle_total` fields from the BFF meta response. The app model stores all three values (`apiVersion`, `faithfulCount`, `oracleTotal`) through the existing `FetchApiVersion` Foldkit command and threads them into shell views as shared `AppChromeMeta`. Malformed or missing coverage fields fold to `null`, so the version line still renders and the `% faithful` line is simply omitted.

Long-lived listeners are Foldkit **Subscriptions**. App subscriptions cover portrait orientation, lobby polling, and game stream frames. Dependency functions decide when each stream is active; returning `Stream.empty` stops work when the route or table changes. Components do not own long-lived fibers.

### Wire protocol (high-level; detail in [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md))

Modules: `client/app/domain/rpc-client.ts`, `client/server/routes/api/rpc/[...path].ts`, `client/app/domain/wire/grpcClient.ts`.

The browser talks only to the same-origin BFF via the hand-written Effect HTTP client (`client/app/domain/rpc-client.ts`) over `/api/rpc`. The Nitro BFF dispatches `/api/rpc/**` requests and calls tonic gRPC through `client/app/domain/wire/grpcClient.ts`. There is no direct browser-to-gRPC communication. The proto wire is the sole contract.

`makeClient(fetch)` accepts a fetch implementation so tests can stub it. `client` is the app singleton (credentials: include, prepended `/api/rpc`). Wire types (`wire/types.ts`) are Effect Schema-decoded DTOs; `wire/protoMap.ts` maps them to/from proto.

### Game delta stream (high-level; board surfaces own paint)

Modules: `client/app/game/stream-subscription.ts`, `client/app/game/fold.ts`.

The game stream is a Foldkit subscription keyed by route table id and active game table id. It opens only when the app is on a table-scoped play route (`/play/:deckId/:table` or `/play/:table`) and the game slice is active. When lobby start flips the seated pregame route into `/play/:table`, the same table id continues to key the live board, so the URL strip does not require a second table lookup or a deck segment. Snapshot and delta frames become messages, then `update` folds them through `applySnapshotPure` / `applyDeltaPure`. `model.game.connected` drives the reconnect banner; rejected intents set `game.reject` and `board.reject`. The subscription goes empty after navigation or table mismatch, so no residual stream continues after leaving the board.

### Design system (`DESIGN.md`, `design.tokens.json`, `client/styles/global.css`)

`design.tokens.json` (DTCG) is the **single source of truth** for design token values. Token prose and rules live in [`DESIGN.md`](../../../DESIGN.md). `bun run gen` (Style Dictionary) generates `client/styles/tokens.generated.css` (Tailwind v4 `@theme`) and `client/app/domain/design-tokens.generated.ts` (canvas named colors). `global.css` imports generated theme output and keeps hand-authored keyframes/interaction rules. Foldkit HTML helpers and shared UI helpers in `client/app/domain/ui/` own component recipes — never via `@apply`, and not as token component maps. Inline style is used only for CSS variables; classes carry appearance. Arbitrary values (`bg-[#18221ef5]`) are for one-off values that token files do not name; they do not extend the token list.

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

### Brand display

Player-facing wordmark and document title use **`edh.reilley.dev`** (lowercase hostname, no scheme). Scryfall and related tooling HTTP User-Agent identity is **`edh.reilley.dev/0.1`** (call sites include `client/app/domain/deck-builder/scryfall.ts` and tooling scripts). Surfaces that show the wordmark include HTML `<title>`, Foldkit `Document.title` / nav brand link (`client/app/view.ts`), auth panel hero, and lobby panel hero. Package names, database names (`mtgfr` / `mtgfr_web`), proto package, GHCR image names, and similar infrastructure identifiers are not renamed as part of this brand display (see Further Notes).

The site favicon is a filled `forest-floor` (#0B1310) circle with a closed-mouth elder-dragon head-and-neck bust cut out as transparent negative space (side profile, facing right; neck base planted on the bottom rim) — GitHub Invertocat-style, not a lettermark and not a square plate. Source of truth is `client/public/favicon.svg`; `client/public/favicon.ico` is a multi-size alpha raster fallback. Install surfaces derive `client/public/pwa-192.png`, `client/public/pwa-512.png`, and `client/public/apple-touch-icon.png` from the same dragon-on-disc art family. `client/index.html` declares `<meta name="theme-color" content="#0B1310" />`, `<link rel="apple-touch-icon" href="/apple-touch-icon.png" />`, `<link rel="icon" href="/favicon.svg" type="image/svg+xml" />`, then `<link rel="icon" href="/favicon.ico" sizes="any" />`.

### Biome

Biome 2.5.3 handles format, lint, and import ordering (`assist.actions.source.organizeImports`, `sortBareImports: true`). `nursery/useSortedClasses` is at error for Tailwind class sorting and configured for safe fixes over `cn` / `clsx`. CSS: `tailwindDirectives: true`. The `test` domain is recommended.

### Observability (pointer)

Browser Faro, BFF OTEL (`client/server/plugins/otel.server.ts`), scrub rules, Faro body caps, and LGTM operator access are specified in [observability-ops](2026-07-20-observability-ops.md) (cluster topology context in [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)).

### Auth UI (`client/app/shell/auth/view.ts`, `client/app/shell/auth/update.ts`)

Single-page login/signup (toggled, not separate routes). `Login` and `Signup` are Foldkit commands wrapping `client.login` / `client.signup`. 401 → "Wrong email or password", 409 → "That email is already registered", anything else → "Something went wrong." On success the server sets an HttpOnly session cookie and the client navigates to `safeNext(params.next)`. `safeNext` enforces same-origin absolute paths only: rejects missing, relative, protocol-relative `//`, backslash `/\`, or scheme-carrying targets.

### Build metadata (`client/app/domain/build-meta.ts`, `client/app/domain/ui/app-version.ts`)

`appVersion()` and `gitCommit()` read from `VITE_APP_VERSION` and `VITE_GIT_COMMIT` env vars baked at build time. Consumed by the BFF OTEL SDK's `serviceVersion` and `vcs.ref.head.revision` resource attributes, and by the `AppVersion` component.

Bottom-left shell chrome (`appVersionBadge`): when `apiVersion` is known, show `API {version}` (`data-testid="app-version"`). When `faithfulCount` and `oracleTotal` are also known and `oracleTotal > 0`, show `{n}% faithful` on the line above (`data-testid="pool-coverage"`). Percentage uses one decimal below 10%, otherwise whole percent (`formatFaithfulPercent`). Coverage comes from `GET /api/meta/version/v1` (`faithful_count` from API `/health/live`, `oracle_total` from BFF-cached Scryfall oracle-cards JSONL count, 24h TTL, non-blocking refresh). Incomplete coverage → version line only. Not shown on the in-game board.

### Production source maps (`vite.config.ts`, `client/app/domain/client-build-options.ts`)

Vite production builds set `build.sourcemap: true` (via `clientBuildSourcemap`) so the large first-party client bundle ships a sibling `.js.map` with a `//# sourceMappingURL=` comment and embedded `sourcesContent`. Chrome DevTools and Faro can resolve minified frames without a separate map-upload pipeline. Maps are public static assets under `.output/public/assets/` (same as the JS); `"hidden"` is intentionally not used because browsers only auto-fetch maps when the comment is present.

---

## Implementation Decisions

- **Foldkit `update` is the state boundary.** UI state changes only through messages handled by `client/app/update.ts` and shell child updates. Async work returns messages through Foldkit commands and subscriptions, which gives consistent error folding, runtime resource injection, and automatic stream teardown.
- **Shell children lift through wrappers, not raw parent tags.** Auth, deck list, deck builder, leaderboard, and lobby commands/subscriptions map back through `Got*Message`, and route entry uses per-surface `informRouteChanged` helpers instead of mutating child slices directly from the parent. Parent-owned redirects and game activation still happen after the lifted child fold.
- **`FetchMe` folds all failures to `null`.** Any 401, decode error, or transport failure during `client.me()` is "not signed in" — mirrors the guard's semantics. Route entry refreshes session state for protected routes to avoid stale login redirects.
- **`safeNext` is checked both in-browser and server-side.** Open-redirect mitigations are layered: the client validates before navigation; the server validates before the session redirect.
- **No `@apply`, no `@layer components`.** Foldkit views and shared UI helpers carry styling through Tailwind classes; inline style carries only CSS variable data. This is the Tailwind shell house rule.
- **Biome class sorting.** `nursery/useSortedClasses` is at error and configured for safe `cn` / `clsx` fixes. Keep class strings sorted in code review and use the editor or Biome fix path for drift.
- **Gzip LZ77 benefit from sorted classes.** Consistent Tailwind class ordering makes repeated utility sequences longer LZ77 matches under gzip on the shipped JS/HTML.
- **Public client source maps.** Production uses `build.sourcemap: true` (not `"hidden"`) so DevTools/Faro can deminify the large first-party bundle. Original TypeScript is fetchable alongside the asset; acceptable for this friend-group deployment without a private map store.

---

## Testing Decisions

- `client/app/shell/auth/**/*.test.ts` — auth stories and helpers, including `ReceivedMe` → `HashMeGravatar` session storage and stale-result guarding.
- `client/app/update.test.ts` — parent-level regressions such as lifting auth child messages through `GotAuthMessage`.
- `client/app/routes.test.ts`, `client/app/smoke.test.ts` — routing and smoke; includes protected `/leaderboard` entry, auth redirect, home entry loading decks without a teaser fetch, retry-from-page-one behavior, and numeric-vs-table single-segment `/play/...` discrimination.
- `client/app/shell/lobby/**/*.test.ts`, `client/app/shell/leaderboard/**/*.test.ts` — route-inform resets, wrapper-lifted parent folds (`GotLobbyMessage`, `GotLeaderboardMessage`), lobby redirect/game handoff, and leaderboard retry/load-more state.
- `client/app/shell/surfaces.test.ts` — shell Scene coverage for auth, deck, leaderboard, and lobby surfaces, including shared account chrome and the `% faithful` + `API {version}` shell badge stack; Scene asserts `pool-coverage` above `app-version` when the model has complete meta.
- `client/app/domain/ui/app-version.test.ts` — percent formatting and stacked badge rendering rules.
- `client/app/game/*.test.ts` — game fold, stream subscription.
- `client/app/domain/rpc-client.test.ts` — Effect HTTP client (stubbed fetch).
- `client/app/domain/wire/*.test.ts` — BFF gRPC / RPC method gate.
- `client/app/domain/ui/*.test.ts`, `client/app/domain/cn.test.ts` — Foldkit UI helpers (`buttonClass`, surfaces).
- `client/app/domain/build-meta.test.ts` — version/commit env var reading.
- `client/app/domain/client-build-options.test.ts` — production `build.sourcemap` stays `true` and wired in `vite.config.ts`.
- Board geometry/paint/HTML tests live under `client/app/board/**` (see board specs / `docs/client-canvas-map.md`).
- Integration test: `just client-check` runs Biome lint + typecheck + Vitest. The full check is `just check` (server + client).

---

## Out of Scope

- Server-side rendering of board state (SPA on Nitro; no SSR of the board).
- Progressive Web App (PWA) / service worker / offline mode.
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
- **Safe area insets.** The landscape rule applies to notched devices — `viewport-fit=cover` with safe-area insets. The portrait gate handles the notched-portrait case; landscape layout tightens padding but does not re-stack.
- **`just client-check`** is the canonical verification: Biome format + lint (including sorted-class check) + TypeScript typecheck + Vitest. Always run before committing client changes.
- **Live client architecture** is Foldkit + Nitro with `client/app/`, `client/app/domain/`, and `client/server/` as the module split.
- **Pool coverage badge design input:** [2026-07-26-pool-coverage-badge-design.md](2026-07-26-pool-coverage-badge-design.md).
