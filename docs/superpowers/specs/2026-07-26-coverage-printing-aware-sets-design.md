# Coverage printing-aware sets (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** card pool TOML / `CardDef.sets`; `tooling/backfill-sets.mjs`; API `faithful_by_set`; BFF coverage meta denominators; `/coverage` sort + independent row scroll; catalog search / wire `sets`. Builds on and **supersedes the per-set metric and default sort** in [coverage-by-set design](2026-07-26-coverage-by-set-design.md) (route, badge entry, and BFF-mediated transport stay). Global badge remains [pool-coverage-badge](2026-07-26-pool-coverage-badge-design.md).

---

## Problem Statement

`/coverage` currently credits each faithful card to a single TOML `set` (default printing) and divides by Scryfall oracle-cards rows whose **representative** printing is in that set. Reprint-heavy products such as Commander 2011 (`cmd`) show nonsense ratios (e.g. 84/37 → clamped 100%) because most CMD printings are not anyone’s current Scryfall default. Default table sort by % also buries recent sets.

## Goal

1. **Per-set Faithful** = faithful pool cards whose oracle appears in that set (any printing).
2. **Per-set Scryfall** = unique Scryfall oracle ids with ≥1 English printing in that set.
3. **Default sort** = set release date descending.
4. **Scroll** = set rows scroll independently; page chrome, search, and column headers stay put.
5. Keep Scryfall **off** the game API at runtime; keep the global `% faithful` badge formula unchanged.

## Locked decisions

| Decision | Choice |
|---|---|
| Per-set Scryfall (denominator) | Unique `oracle_id`s with ≥1 printing in set `X` (from Scryfall `default_cards` bulk on the BFF) |
| Per-set Faithful (numerator) | Faithful pool cards (`approximates` none) that list `X` in `CardDef.sets` — one card may credit many sets |
| Join key / authorship | `CardDef.id` = Scryfall oracle id; `sets` authored/maintained by a backfill script |
| TOML shape | Drop singular `set`; keep `default_print`; add `sets = ["…"]` (unique lowercase codes, sorted) |
| Primary set | None — `default_print` is enough for art; search uses all of `sets` |
| Where mapping lives | Compiled card pool (registry), not a Postgres join table, not a PVC |
| Who talks to Scryfall | **BFF** (denominators + set catalog) and **offline script** (write `sets` into TOMLs). API never dials Scryfall |
| Global badge | Unchanged: faithful count ÷ oracle-cards bulk line count |
| Default table sort | `released_at` descending; null/missing dates last; tie-break name, then code |
| Coverage scroll | Page chrome, search, and column headers stay fixed; only the set **row list** scrolls in the remaining viewport |
| Cache | In-memory compact index on BFF (24h TTL, SWR); no Kubernetes PVC for v1 |

## Approaches considered

1. **BFF-only `default_cards` index + API faithful oracle ids** — correct join at runtime; large BFF download; set membership not visible in pool files.
2. **Postgres `scryfall_oracle_sets` on mtgfr** — durable shared mapping; needs an ingest path; heavier than needed for coverage.
3. **Pool `sets` + backfill script + BFF denominators (chosen)** — set mapping travels with card TOMLs (easy to refresh for new cards); API expands `sets` into `faithful_by_set` with no Scryfall I/O; BFF still supplies unique-oracle-per-set totals and `/sets` metadata; client sorts by release date.

## Design

### Pool & script

- Replace `CardDef.set: &'static str` with `sets: Arc<[&'static str]>` (same spirit as `otags`). Empty means “unrecorded” — card contributes to no per-set faithful counts.
- TOML: remove `set = "…"`; add `sets = ["cmd", "c16", …]`.
- Add `tooling/backfill-sets.mjs` alongside existing `tooling/backfill-*.mjs`:
  - Input: deckable card TOMLs with `id` (oracle id).
  - Source: Scryfall printings for that oracle (prefer one `default_cards` bulk pass for whole-pool runs; single-card path may use search by `oracleid:`).
  - Output: idempotent rewrite of top-level `sets = […]` as unique lowercase set codes, sorted.
  - Document in card-dsl skill / DSL reference: authors run the script when adding cards rather than hand-maintaining long lists.
- Catalog search haystack indexes **all** codes in `sets` (so a non-default printing’s set still matches).
- Wire / schema: `CatalogCard.set: String` → `sets: Vec<String>` (proto + client decode updated in the same change). Deck-builder print badges continue to use Scryfall print objects for the chosen printing’s set label.

### API

- `/health/live` keeps `version`, `faithful_count`, and `faithful_by_set`.
- Recompute `faithful_by_set`: for each deckable def with `approximates.is_none()`, for each code in `def.sets` (skip empty), increment that lowercase key. A card in N sets increments N buckets.
- Consequently the **sum of `faithful_by_set` values may exceed `faithful_count`** (multi-credit). Drop any test/invariant that required the sum ≤ global faithful.
- No Scryfall calls. No new Postgres tables for set membership.

### BFF

- Keep `GET /api/meta/coverage/v1` response shape (`faithful_count`, `oracle_total`, `sets[]` with `code`, `name`, `released_at`, `faithful`, `oracle_total`).
- Change how `oracle_total` per set is derived: parse Scryfall **default_cards** bulk into a compact in-memory index — unique `oracle_id` count per set code — then discard the bulk. Do **not** use oracle-cards’ single representative `set` for per-set denominators.
- Keep oracle-cards bulk (or equivalent) for the **global** `oracle_total` used by the badge and coverage header.
- Keep `/sets` cache for names, `released_at`, and `card_count > 0` row filter.
- Join: API `faithful_by_set[code]` (default 0) with cached per-set unique-oracle totals; missing denominator → `null` → client `—`.
- UA `edh.reilley.dev/0.1`; 24h TTL; stale-while-revalidate; fire-and-forget refresh. No PVC / emptyDir for v1.

### Client

- `/coverage` default sort: `released_at` descending (ISO date strings compare lexicographically when present); rows with null/missing `released_at` after dated rows; then `name`, then `code`.
- Search filter unchanged (code / name).
- Percent formatting unchanged (`formatFaithfulPercent`, including clamp when faithful > oracle after drift).
- **Independent row scroll:** the page shell is a full-height column (`h-full` / flex), not a single document scroll. Fixed (non-scrolling) region: title + global % + Play/account chrome, error/status, search field, and the Set / Faithful / Scryfall / % column header row. The set rows live in a sibling panel with `min-h-0 flex-1 overflow-y-auto` (or equivalent) so only that list scrolls inside the remaining viewport. Avoid `overflow-y-auto` on the outer `main` once the inner scroller owns the rows (otherwise nested scroll fights). Safe-area padding stays on the outer shell.

### Error / degradation

| Condition | Behavior |
|---|---|
| Coverage endpoint fails | Page error + retry; badge keeps prior global meta if any |
| Sets list warm, per-set oracle cold | Rows with `—` until cache completes |
| Scryfall fail with stale cache | Serve last good denominators + catalog |
| Card with empty `sets` | Omits from all per-set faithful counts (authoring gap; script should fill) |
| Unauthenticated | Redirect to `/login?next=/coverage` |

### Testing

- **Script:** pure helpers — printings → unique sorted codes; TOML rewrite strips/reinserts `sets` idempotently (fixture files).
- **Engine/cards:** `sets` deserializes; fixtures may use empty `sets`.
- **Server health:** multi-set credit (one faithful card in two codes increments both); `approximates` excluded; empty `sets` omitted.
- **BFF:** per-set `oracle_total` counts unique oracles across printings (regression: a reprint set’s total is not the default-print-only count); SWR on failure.
- **Client:** `visibleCoverageRows` sorts by release date desc; Scene coverage still covers load/search/error/retry; assert the row list container is the scroll owner (e.g. `data-testid="coverage-table-body"` with overflow scroll) while chrome/search/headers stay outside it.
- **Catalog:** search for a set code that appears only in `sets` (not formerly singular `set`) returns the card.
- **Specs at implement time:** update living [coverage-by-set](2026-07-26-coverage-by-set.md), [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) / accounts-catalog as needed, [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md) + DSL reference. This file remains design input.

## Out of scope

- Drill-down lists of cards in a set
- Kubernetes PVC or emptyDir persistence for Scryfall bulk/index
- Postgres `scryfall_oracle_sets` (or equivalent) join tables
- Changing the global badge numerator/denominator
- Board / in-game coverage HUD
- SEO / public unauthenticated marketing page

## Success criteria

- CMD-style rows show faithful ≤ Scryfall unique-oracle-in-set (absent authoring/cache drift), with a denominator reflecting every oracle printed in the product — not ~37 default-print orphans.
- New cards get `sets` via one script invocation, not hand-edited reprint lists.
- `/coverage` opens with newest sets first by `released_at`.
- Scrolling the set list does not move the page header, search, or column headers.
- API process never calls Scryfall; global badge behavior unchanged.
- Failures never invent coverage numbers or block the rest of the shell.
