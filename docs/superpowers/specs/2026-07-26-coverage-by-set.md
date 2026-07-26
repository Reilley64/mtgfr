# Coverage by Set
**Status:** Current (as of 2026-07-26)
**Module:** `client/app/shell/coverage/**`, `client/app/domain/lobby/client.ts`, `client/app/domain/coverage-meta.ts`, `client/app/domain/scryfall-sets.ts`, `client/app/domain/scryfall-oracle-total.ts`, `client/server/routes/api/meta/coverage/v1.get.ts`, `client/server/lobby-http.ts`, `crates/server/src/health.rs`

## Problem Statement

The global `% faithful` shell badge answers only how much of the total pool is faithful. Players and operators also need to see which individual Scryfall sets are well covered, untouched, or missing denominator data so grind priority and deck-fidelity planning can happen inside the product instead of by ad hoc scripts.

## Solution

Ship an authenticated `/coverage` shell route that renders a searchable set table from BFF-owned coverage meta. The BFF joins live API `faithful_by_set` counts with cached Scryfall set metadata, cached printing-aware per-set oracle totals from Scryfall `default_cards`, and the cached global oracle-cards total; the client reuses the shell badge percent formatter so global and per-set percentages stay consistent.

## User Stories

- As a signed-in player, I can open `/coverage` directly or by clicking the shell `% faithful` badge.
- As a player, I can search sets by code or name and quickly find the one I care about.
- As a player or operator, I can see honest per-set percentages, including `0%` for uncovered sets and `—` when the denominator is unavailable.
- As a player, I can retry after a failed coverage fetch without stale rows lingering on screen.

## Behavior

### Route entry and shell chrome

- `/coverage` is an auth-gated shell route. Unauthenticated entry redirects to `/login?next=%2Fcoverage`.
- Route entry calls `Coverage.informRouteChanged`, which loads coverage through `LobbyClient.coverageMeta()` from `GET /api/meta/coverage/v1`. Child commands and view events lift through `GotCoverageMessage`.
- The page renders `data-testid="coverage-page"` on the same felt shell background family as leaderboard and deck surfaces.
- Header chrome shows `Coverage`, a shellFrame subtitle global `{n}% faithful` line or `— faithful` (`coverage-global-percent`), a `Play` link back to `/`, and the shared avatar account menu with the `Leaderboard` shortcut still visible. Per-set `%` uses vine display emphasis, not priority gold.
- The fixed bottom-left shell badge still renders on this page when `apiVersion` is known. When global badge coverage meta is complete, the `pool-coverage` line links to `/coverage`.

### Loading, error, and retry states

- While loading, the page shows `Loading coverage...`, clears any prior rows, clears prior global counts, clears the prior error, and closes the account menu.
- The search query is preserved across refreshes, including `Try again`.
- If the `LobbyClient` request fails with a tagged transport/decode error, the page enters `status: "error"`, shows `Could not load coverage.` in an alert, keeps rows empty, and renders a `Try again` button.
- The search field is hidden only while loading. It remains available during ready and error states.
- Ready-with-no-rows copy depends on the query: `No set coverage available.` for an empty query and `No sets match.` for a non-empty query (`coverage-empty`). After a load failure, `Try again` uses `coverage-try-again`.

### Table rows, sorting, filtering, and formatting

- The table renders only in `status: "ready"` with at least one visible row.
- Columns are `Set`, `Faithful`, `Scryfall`, and `%`.
- Each row shows the set code, set name, faithful count, oracle total, and formatted percent.
- Search filters by lowercase substring match on set code or set name.
- Rows sort by `releasedAt` descending, with null release dates after dated sets, then set name ascending, then set code ascending.
- The page shell fills a scroll-locked `shellFrame` stage (`lockStageScroll`, `h-full min-h-0 flex-1` + `overflow-hidden` on `coverage-page`) so the row list can overflow; chrome, search, and column headers stay pinned and only `data-testid="coverage-table-body"` (`overflow-y-auto overscroll-contain`) scrolls.
- Rows with `oracleTotal == null` render `—` in both the `Scryfall` and `%` columns.
- Rows with `oracleTotal > 0` and `faithful = 0` render `0%`, not `—`.
- Percent text reuses `formatFaithfulPercent`: one decimal below 10%, otherwise a whole percent.

### Coverage meta pipeline

- `GET /api/meta/coverage/v1` returns global `faithful_count`, global `oracle_total`, and a `sets` array shaped as `{ code, name, released_at, faithful, oracle_total }`.
- On each coverage fetch the BFF loads Scryfall `/sets`, per-set unique-oracle totals from `default_cards`, and the global oracle-cards total via `load*` helpers: cold caches are awaited so the first `/coverage` response is not all `—`; warm caches return immediately and refresh in the background (SWR).
- Only Scryfall sets with `card_count > 0` appear in the joined set list.
- Scryfall rows with `set_type` in `memorabilia`, `token`, `minigame`, or `vanguard` are omitted — they are not deckable pool targets (Art Series, tokens, minigames, vanguard/avatars).
- Set rows always come from the cached Scryfall set list, not from the card registry alone, so zero-faithful sets still appear.
- When the API live-status fetch succeeds, `faithful_by_set` joins into the cached set rows and missing set keys default to `faithful: 0`.
- When the API live-status fetch fails or parses invalidly, the BFF still returns cached Scryfall rows with `faithful: 0`, cached global `oracle_total` when available, and `faithful_count: null`. The page stays in the ready state if the HTTP response still decodes.
- Per-set denominators count unique Scryfall `oracle_id`s that have at least one English printing in the set, derived from the `default_cards` bulk feed rather than oracle-cards' single default-printing set field.
- The global `oracle_total` used by the page header and shell badge remains the oracle-cards bulk line count.
- When no cached per-set oracle total exists for a set, the joined row keeps `oracle_total: null` so the client shows `—` instead of inventing a denominator.

### API live health input

- `GET /health/live` always reports `version`, `faithful_count`, and `faithful_by_set`.
- `faithful_count` counts deckable card defs whose `approximates` field is absent.
- `faithful_by_set` counts those same faithful card defs once for every lowercase code in their `sets` field.
- A card with multiple `sets` credits multiple buckets, so the aggregate `faithful_by_set` values can be larger than the global faithful count.
- Cards with empty `sets`, or empty entries inside `sets`, are omitted from `faithful_by_set`.

## Implementation Decisions

- Keep Scryfall joins in the BFF, not the browser, so the shell surface stays same-origin and independent of third-party client fetches.
- Reuse the shell badge formatter for both the page header and row percentages so `/coverage` and the fixed `% faithful` chrome never diverge on display rules.
- Represent unavailable denominators as `null` on the wire and `—` in the UI. Do not coerce them to `0`.
- Compute `faithful_by_set` from `cards::registry()` inside server health with no extra I/O.
- Use 24-hour in-memory caches for Scryfall set metadata, `default_cards` per-set denominators, and the global oracle-cards total; coverage awaits cold fills and SWR-refreshes when warm.
- Align the coverage shell feature with other `Got*` submodels: `shell/coverage/index.ts` namespace exports, `informRouteChanged`, parent `GotCoverageMessage`, and `Command.mapMessages` lift — no flat coverage tags in the parent `Message` union.
- Keep coverage HTTP behind the `LobbyClient` Effect service so tests can inject the same same-origin meta client used by lobby commands.

## Testing Decisions

- `crates/server/src/health.rs` tests assert `faithful_by_set` matches the registry, omits empty set codes, excludes approximated cards, and multi-credits cards whose `sets` list has more than one code; tests do not require the aggregate to stay below the global faithful count.
- `client/app/domain/scryfall-set-oracle-totals.test.ts` asserts per-set denominators count unique oracle ids across `default_cards` printings.
- `client/app/domain/coverage-meta.test.ts` asserts the BFF join uses Scryfall rows as the source of truth for the set list, joins printing-aware per-set oracle totals, and leaves missing per-set oracle totals as `null`.
- `client/app/shell/coverage/view.test.ts` asserts release-date-desc row sorting, lowercase filtering, and `—` formatting when either count is missing.
- `client/app/shell/coverage/story.test.ts` asserts `GotCoverageMessage` parent folding for refresh.
- `client/app/shell/coverage/update.test.ts` asserts `FetchCoverage` loads through `LobbyClient` and folds tagged failures to `Could not load coverage.`.
- `client/app/routes.test.ts` asserts `/coverage` route parsing, auth redirect, and retry behavior that clears rows while preserving the query, with messages wrapped as `GotCoverageMessage` / `GotAuthMessage`.
- `client/app/shell/surfaces.test.ts` asserts the coverage page scene, `coverage-table-body` row scroller, search/filter empty state, and shell badge link to `/coverage`.
- Verification for this task runs focused server nextest filters, focused client Vitest suites, `just client-typecheck`, `just client-lint`, and `just server-format-check`.

## Out of Scope

- Card-level drill-down from a set row into individual scripts or fidelity gaps.
- Per-card primary-set selection beyond `default_print` art metadata.
- In-game coverage HUD or board chrome.
- Public unauthenticated coverage pages, SEO, or crawler-facing coverage content.

## Further Notes

- Design input remains in [2026-07-26-coverage-by-set-design.md](2026-07-26-coverage-by-set-design.md), superseded on metric/sort details by [2026-07-26-coverage-printing-aware-sets-design.md](2026-07-26-coverage-printing-aware-sets-design.md); this file documents the shipped surface.
- The entry badge behavior and route registration also appear in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).
