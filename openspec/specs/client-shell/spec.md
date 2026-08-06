# client-shell Specification

## Purpose

The Foldkit SPA shell owns routing, authentication, shared chrome, design tokens, the same-origin BFF edge, installable PWA surface, coverage and leaderboard pages, and the shared UI component layer that shell and board HTML overlays compose from.

## Requirements

### Requirement: Foldkit App Shell and Routing

The client SHALL be a Foldkit SPA on Nitro (Vite) with a single event-reactor (`Model` / `Message` / `update` / `view`) and shell submodels. Async and wire work SHALL stay in Effect at runtime boundaries; Foldkit SHALL own UI state. Required identifiers SHALL live in path params; query params SHALL be optional (`?next=` for post-login redirect). Parent `Message` tags for child surfaces SHALL be `Got*Message` wrappers only. Route entry and post-session cold-load SHALL call per-surface `informRouteChanged` helpers. Route bodies SHALL be keyed by surface so navigation remounts rather than patching reused elements.

#### Scenario: Auth-gated route table
- **WHEN** a signed-out player opens a protected path
- **THEN** the app redirects to `/login?next=<current-path>` without rendering protected content

#### Scenario: Play route discrimination
- **WHEN** a player opens a single-segment `/play/...` path
- **THEN** integer-looking segments normalize to deck entry and table codes (six characters containing at least one letter) normalize to the table-scoped route; bare `/play` and `?deck=` entry are not found

#### Scenario: Surface key remount
- **WHEN** navigation switches between shell surfaces that share `shellFrame`
- **THEN** the keyed surface remounts so Mount hooks attach to the incoming surface

### Requirement: Authentication and Session Guard

The shell SHALL support email+password login and signup on `/login`. `FetchMe` SHALL fold every failure to unsigned-in. Post-login navigation SHALL use `safeNext`: only same-origin absolute paths starting with `/` (not `//` or `/\`). Signed-in sessions SHALL SHA-256 hash the email into `meGravatarHash` for account chrome without storing raw email in UI face state. Unsigned protected content SHALL never render.

#### Scenario: Open-redirect rejection
- **WHEN** `?next=` carries a protocol-relative or scheme-carrying target
- **THEN** the client and server reject it and use a safe default

#### Scenario: Login error mapping
- **WHEN** login returns 401 or signup returns 409
- **THEN** the auth panel shows the mapped user-facing error and stays on `/login`

### Requirement: Shared Shell Frame and Account Chrome

Auth, decks, lobby, leaderboard, and coverage routes SHALL render through `shellFrame`: felt atmosphere, three-column header, optional subtitle, centered stage, and the fixed bottom-left meta badge. Board routes SHALL bypass `shellFrame`. Signed-in shell routes SHALL show shared account chrome: optional Leaderboard link plus avatar `Menu` (username heading, Change at Gravatar opening gravatar.com in a new tab, Sign out). Shell body text SHALL use `font-shell`; titles SHALL use `font-display`. Under `prefers-reduced-motion: reduce`, stage-enter animation SHALL not run.

#### Scenario: Account menu sign-out
- **WHEN** the player chooses Sign out from the account menu on a protected route
- **THEN** the session ends and the app redirects to `/login?next=…`

#### Scenario: Builder header actions
- **WHEN** the deck builder is open
- **THEN** Cancel is in the header leading slot and Save is in the trailing slot before account chrome

### Requirement: Landscape Rotate

When `(orientation: portrait) and (max-width: 900px)` matches, the app root SHALL apply CSS landscape rotate (swap width/height, rotate 90°, safe-area padding). Every route — shell and board — SHALL live under that rotate root. The board SHALL NOT reflow into a vertical portrait layout.

#### Scenario: Portrait phone rotate
- **WHEN** a portrait phone viewport matches the landscape-rotate media query
- **THEN** `landscape-root` carries `landscape-rotate-root` and landscape-first layouts remain side-by-side

### Requirement: Brand, Tokens, PWA, and Favicon

Player-facing wordmark and document title SHALL be `edh.reilley.dev`. `design.tokens.json` SHALL be the DTCG token source (primitive/semantic tiers, OKLCH, typed composites; no `$type: "css"`); generated CSS `@theme` and canvas constants SHALL be produced by codegen and not hand-edited. The site favicon SHALL be the dragon-on-disc silhouette family (`favicon.svg` / `.ico` and PWA icons). The shell SHALL be installable via `vite-plugin-pwa` with a network-only service worker (no offline precache or runtime caching for app/API/game streams). Production builds SHALL emit public source maps (`build.sourcemap: true`).

#### Scenario: Network-only worker
- **WHEN** the service worker handles a fetch
- **THEN** it always performs `fetch(event.request)` with no precache or offline document fallback

#### Scenario: Auth brand hero
- **WHEN** the player is on `/login`
- **THEN** the stage shows the `edh.reilley.dev` wordmark as display hero above the auth panel

### Requirement: Meta Badge and Coverage by Set

Boot SHALL fetch `/api/meta/version/v1`. When `apiVersion` is known, shell surfaces SHALL show `API {version}` bottom-left. When `faithfulCount` and `oracleTotal` are also known with `oracleTotal > 0`, the shell SHALL show `{n}% faithful` above the version line and link that line to `/coverage`. The board SHALL omit this chrome. `/coverage` SHALL be an auth-gated searchable set table from `GET /api/meta/coverage/v1`, sorting by release date descending (nulls last), filtering by set code/name, and showing `—` when denominators are missing. Per-set denominators SHALL be printing-aware unique oracle ids; global `oracle_total` SHALL remain the oracle-cards bulk count. The BFF SHALL join Scryfall set metadata with API `faithful_by_set` and omit non-deckable set types.

#### Scenario: Incomplete coverage meta
- **WHEN** either coverage count is missing or invalid
- **THEN** the shell badge shows only the version line

#### Scenario: Coverage retry
- **WHEN** coverage load fails and the player chooses Try again
- **THEN** rows clear, the query is preserved, and the load restarts

#### Scenario: Zero-faithful set row
- **WHEN** a set has `oracleTotal > 0` and `faithful = 0`
- **THEN** the row shows `0%`, not `—`

### Requirement: Leaderboard Surface

`/leaderboard` SHALL be auth-gated and load ranked players via `rpc.ratings.leaderboard` with paging (`limit`/`offset`, Load more). Failed later pages SHALL keep existing rows, hide Load more, and offer Try again that restarts from page one. Rating emphasis SHALL use vine, not priority gold. Home SHALL NOT fetch a separate top-players teaser.

#### Scenario: Leaderboard page-two failure
- **WHEN** a later page load fails after rows are visible
- **THEN** existing rows remain and Try again clears them and restarts from the first page

### Requirement: Same-Origin Wire Edge

The browser SHALL talk only to same-origin `/api/rpc` via the Effect HTTP client; the Nitro BFF SHALL dial tonic gRPC. Lobby and meta HTTP SHALL be one Nitro route file per operation with path-param table ids for join/ready/start. Game stream subscriptions SHALL open only on table-scoped play routes with an active game slice and tear down when the route or table changes. Effect / `@effect/*` / `foldkit` / `@foldkit/ui` SHALL stay pinned to exact matching versions.

#### Scenario: No direct browser gRPC
- **WHEN** the client needs game or catalog RPCs
- **THEN** requests go to `/api/rpc` and never dial tonic from the browser

### Requirement: UI Component Layer

`client/app/domain/ui/` SHALL export Foldkit component functions that wrap `@foldkit/ui` headless primitives with module-private cva recipes (`button`, `input`, `modalDialog`, `confirmDialog`, `windowedGrid`) and class helpers for primitive-owned chrome (`menuPanelClass` / `menuItemClass`, `surfaces.ts`). Elements with variants SHALL be components; elements without variants SHALL be `cn` helpers. Recipes SHALL import `cva` only through `recipe.ts` wired to `cn`. Disabled buttons SHALL set the native `disabled` property. Prompt modals and the mulligan overlay SHALL remain hand-rolled because `Dialog` always allows Escape/backdrop dismiss.

#### Scenario: Disabled button is inert
- **WHEN** a `button` is rendered with `disabled`
- **THEN** the native disabled property blocks focus, click, and form submission

#### Scenario: Windowed builder grids
- **WHEN** the deck builder pool or print picker renders thousands of tiles
- **THEN** `windowedGrid` mounts only viewport-near tiles

### Requirement: Foldkit DevTools

Local Vite SHALL expose Foldkit DevTools MCP on relay port `9988`. App entry SHALL keep `devTools: { Message }` in `Runtime.makeApplication`. `foldkit_list_runtimes` SHALL see a runtime only while a browser tab has the app open. DevTools SHALL NOT affect production board behavior.

#### Scenario: Runtime visibility
- **WHEN** the client app is open in a browser tab during local dev
- **THEN** Foldkit DevTools can list the live runtime

### Requirement: Client Interaction Testing

Shell and board HTML surfaces SHALL have Scene coverage with stable `data-testid` markers. Changes to pointer, keyboard, hover, drag, Mount hosts, lobby/host flow, or BFF env defaults SHALL add or extend unit/Scene tests that assert user-visible outcomes, not only presence. Interaction/UI PRs SHALL run the verify Interaction checklist before claiming done. Product-language test names SHALL be used; migration/parity framing SHALL NOT.

#### Scenario: Outcome assertion required
- **WHEN** a PR changes hand drag-play behavior
- **THEN** tests assert the hand tile hides and a flight is seeded, not merely that a testid exists
