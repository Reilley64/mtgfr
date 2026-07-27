# Coverage by set (design)

**Status:** Superseded design input (2026-07-26). Route and badge entry still stand, but current metric, sort, scroll, and catalog-field behavior are superseded by [coverage-printing-aware-sets design](2026-07-26-coverage-printing-aware-sets-design.md) and documented in [coverage-by-set](2026-07-26-coverage-by-set.md).
**Surfaces:** new `/coverage` shell route; `shell-routes-and-auth` (badge entry + routing); BFF `meta/coverage`; API faithful-by-set; builds on [pool-coverage-badge](2026-07-26-pool-coverage-badge-design.md).

---

## Problem Statement

The shell `% faithful` badge answers only the global question. Operators and players cannot see which Scryfall sets the pool covers well versus which are untouched, so grind priority and “how are we doing on SoC / modern / …” stay out of product.

## Goal

A bookmarkable **Coverage** page listing the **full Scryfall set catalog**, each row showing pool completeness versus Magic for that set. Entry is a click on the existing `% faithful` badge.

## Locked decisions

| Decision | Choice |
|---|---|
| Per-set metric | Superseded: current behavior credits every code in `sets` and uses `default_cards` unique-oracle denominators |
| Set list | Full Scryfall set catalog (not pool-touched-only) |
| Entry | Click bottom-left `% faithful` badge |
| Navigation | Real route `/coverage` (bookmarkable; browser back works) |
| Auth | Auth-gated like `/` and `/leaderboard` |
| Transport | BFF-mediated meta (Approach 1); no browser→Scryfall for this surface |
| Pool set metadata | Superseded: current card metadata is `sets = [...]`; `default_print` carries the art/default-printing pointer |
| Freshness | Scryfall-derived set list + per-set oracle totals cached on BFF (~24h TTL, stale-while-revalidate). Faithful-by-set changes only on API deploy |

## Approaches considered

1. **BFF set-coverage meta + `/coverage` shell route (chosen)** — API faithful-by-set map; BFF joins Scryfall `/sets` + Scryfall-derived per-set counts; client table page; badge navigates to `/coverage`.
2. Client joins `Cards.Catalog` + live Scryfall — rate limits and chrome dependence on third-party from the browser.
3. Bake static `coverage-by-set.json` at deploy — stale versus Scryfall until redeploy; fights runtime refresh used for the global badge.

## Design

### Metrics

- **Per-set `faithful`:** Superseded by printing-aware `sets` multi-credit behavior in the living coverage spec.
- **Per-set `oracle_total`:** Superseded by `default_cards` unique-oracle denominators in the living coverage spec.
- **Global** `faithful_count` / `oracle_total` on the same response keep the page header aligned with the badge.
- **Percentage:** `100 * faithful / oracle_total` when `oracle_total > 0`; format with one decimal below 10%, otherwise whole percent (same rules as the badge). When `oracle_total` is missing, show `—` (not `0%`). When `oracle_total > 0` and `faithful = 0`, show `0%`.

### API

Expose a faithful-by-set map for the BFF (extend `/health/live` or add a sibling `/health/coverage` JSON). Shape:

```json
{
  "version": "…",
  "faithful_count": 662,
  "faithful_by_set": { "soc": 329, "cmd": 80 }
}
```

Computed from `cards::registry()`; no I/O. Tokens remain outside the registry.

### BFF

`GET /api/meta/coverage/v1` joins API faithful data with cached Scryfall facts:

```json
{
  "faithful_count": 662,
  "oracle_total": 28412,
  "sets": [
    {
      "code": "soc",
      "name": "Secrets of Strixhaven",
      "released_at": "2026-04-01",
      "faithful": 329,
      "oracle_total": 400
    }
  ]
}
```

Cache strategy (TTL **24h**, stale-while-revalidate on failure):

1. `GET https://api.scryfall.com/sets` — code, name, released_at, set_type, digital, card_count.
2. Superseded: current behavior keeps oracle-cards for the global denominator and uses `default_cards` for per-set denominators.
3. Emit one row per set from `/sets` with `card_count > 0` (or equivalent non-empty set). `faithful` defaults to `0` when absent from the API map. If a set has no oracle-bulk rows, omit `oracle_total` so the client shows `—`.

User-Agent: `edh.reilley.dev/0.1`. Prefer deriving per-set oracle totals from the **same bulk** used for the global total (one download, two aggregates) rather than hundreds of `cards/search` calls. Fire-and-forget refresh; never block unrelated shell meta.

Keep `GET /api/meta/version/v1` for the badge (global only). Coverage page uses the richer `/api/meta/coverage/v1`.

### Client

- New Foldkit route `/coverage` → coverage shell view.
- Badge (`pool-coverage`): clickable control that requests navigation to `/coverage` (pointer-events enabled on that line or the stack). Still omitted on the board. On `/coverage` itself, navigating again is a no-op or refresh.
- Page chrome: match leaderboard shell (felt background, Play → `/`, account menu).
- Header: **Coverage** + global `{n}% faithful`.
- Body: client-filtered, client-sorted table — columns **Set** (code + name), **Faithful**, **Scryfall**, **%**. Current default sort is release date descending, then name/code. Search filters code/name.
- Loading / error / retry: leaderboard-style; do not invent rows or percentages.

### Error / degradation

| Condition | Behavior |
|---|---|
| Coverage endpoint fails | Page error + retry; badge keeps prior global meta if any |
| Sets list warm, per-set oracle cold | Rows with `—` until cache completes; soft refresh OK |
| Scryfall fail with stale cache | Serve last good coverage payload |
| Unauthenticated | Redirect to `/login?next=/coverage` |

### Testing

- **Server:** current faithful-by-set tests live in [coverage-by-set](2026-07-26-coverage-by-set.md) and cover `sets` multi-credit behavior.
- **BFF:** join produces full catalog rows; pool-only sets not required; zero-faithful sets present; SWR on Scryfall failure.
- **Client:** Scene — `/coverage` title, table, search; badge click navigates to `/coverage`; `%` formatting; `—` when oracle total missing.
- **Specs at implement time:** update [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md); add a living surface spec for the coverage page (or extend shell-routes if the page stays thin). This file remains design input.

## Out of scope

- Drill-down lists of cards in a set
- Per-card primary-set selection beyond `default_print` art metadata
- Deck-builder set filter chrome
- Board / in-game coverage HUD
- SEO / public unauthenticated marketing page

## Success criteria

- Signed-in users open `/coverage` from the `% faithful` badge and see every Scryfall set with a honest % (or `—`).
- Global header % matches the badge when both metas are warm.
- Scryfall denominators can move without an app redeploy (within cache TTL).
- Failures never invent coverage numbers or block the rest of the shell.
