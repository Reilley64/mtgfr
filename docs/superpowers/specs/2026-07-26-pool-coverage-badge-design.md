# Pool coverage badge (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** `shell-routes-and-auth` (app version chrome), BFF `meta/version`, API `/health/live`; catalog fidelity posture from `card-dsl-and-card-pool`.

---

## Problem Statement

Players and operators have no in-product sense of how far the shipped card pool has progressed toward “any card, faithfully.” Docs and grind reports track fidelity offline; the shell only shows `API {version}` in the bottom-left badge.

## Goal

Show a single percentage in the client shell: **faithful pool cards ÷ unique Scryfall oracle cards**, stacked **above** the existing API version line.

## Locked decisions

| Decision | Choice |
|---|---|
| Numerator | Deckable pool cards with **no** `approximates` (“silence means faithful”) |
| Denominator | Unique Scryfall **oracle** identities (oracle-cards cardinality) |
| Display | Percentage only (e.g. `2.3% faithful`) — no counts in chrome |
| Placement | Above `API {version}` in the existing bottom-left badge stack |
| Surfaces | Shell pages that already show the API badge (auth, deck list, builder, lobby, leaderboard). **Not** the in-game board |
| Freshness | Denominator refreshes at runtime (periodic Scryfall fetch + cache). Numerator changes only when the API ships a new card pool |
| Transport | Approach 1 — BFF-mediated meta; no browser→Scryfall calls |

## Approaches considered

1. **BFF-cached coverage via shell meta (chosen)** — Server exposes faithful count; BFF caches Scryfall oracle total; client renders `%`. Matches `meta/version/v1` posture.
2. Client fetches Scryfall; server only returns faithful count — rate limits and failure modes in the browser.
3. Bake both into API `/health/live` with server-side Scryfall polling — outbound third-party I/O on game pods.

## Design

### Metrics

- **`faithful_count`:** Count of deckable `CardDef`s in the engine registry where `approximates` is absent. Tokens and other non-deckable defs are excluded. Same pool the boot-time `catalog_cards` projection uses.
- **`oracle_total`:** Number of unique Scryfall oracle identities. Implementation must equal (or be validated against) the length of Scryfall’s **oracle-cards** bulk set. Prefer a cheap Scryfall API that returns that cardinality; if none is reliable at implement time, the BFF may download oracle-cards once per cache TTL and count objects. Do **not** silently substitute `/catalog/card-names` `total_items` without documenting a deliberate caveat (names ≠ oracle ids).

### Endpoints

**API `/health/live`** (existing JSON) gains:

```json
{ "version": "…", "faithful_count": 662 }
```

`faithful_count` is computed from the in-memory registry (or the catalog projection) when the process serves health — cheap integer, no I/O.

**BFF `GET /api/meta/version/v1`** (existing) expands to:

```json
{
  "version": "…",
  "faithful_count": 662,
  "oracle_total": 28412
}
```

- BFF reads `version` + `faithful_count` from upstream `/health/live` (one round-trip, as today for version).
- BFF maintains a cache for `oracle_total` (key e.g. `scryfall:oracle_total`), **TTL 24h**, stale-while-revalidate: on Scryfall failure, serve the last good total when present.
- Coverage fields are optional on the wire so older clients ignore them; the SPA treats them as required only for rendering the % line.

### Client UI

- Extend `appVersionBadge` (`client/lib/ui/app-version.ts`) into a two-line fixed bottom-left stack (same position / muted label styling as today):
  1. `{n}% faithful` — `data-testid="pool-coverage"`
  2. `API {version}` — `data-testid="app-version"` (unchanged)
- Percentage: `100 * faithful_count / oracle_total`. Format with **one decimal when below 10%** (e.g. `2.3%`), otherwise a whole number (e.g. `12%`). Render only when both counts are present and `oracle_total > 0`.
- Model: keep fetching via the existing API-version Effect path; store optional coverage fields beside `apiVersion`.
- No new colors, pills, cards, or tooltips required for v1.

### Error / degradation

| Condition | Behavior |
|---|---|
| Version unknown | No badge (unchanged) |
| Version ok, coverage incomplete | Version line only |
| Scryfall refresh fails, warm cache | Keep last `oracle_total` |
| Scryfall refresh fails, cold cache | Omit `%` line |
| Shell load | Never blocked on Scryfall |

### Testing

- **Server:** `faithful_count` excludes cards with `approximates` and non-deckable defs; included in `/health/live` JSON tests.
- **BFF:** cache hit; cache miss + Scryfall ok; Scryfall fail with stale; Scryfall fail without stale (omit `oracle_total`).
- **Client:** Scene test — with coverage + version in model, `pool-coverage` appears above `app-version` with the expected `% faithful` string; without coverage, version alone. Assert outcome, not migration framing.
- **Specs to update at implement time:** [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) (badge chrome + meta shape); [lobby-table-routing](2026-07-20-lobby-table-routing-and-live-game.md) or accounts/catalog only if `/health/live` is documented there. This file remains design input, not a living surface spec.

## Out of scope

- Per-card fidelity UI or engine `summary` as alternate rules text
- Showing approximated count or raw fraction in chrome
- Board / in-game HUD placement
- SEO or marketing use of the percentage
- Changing the definition of “faithful” beyond absent `approximates`

## Success criteria

- Shell shows `{n}% faithful` above `API {version}` when meta is complete.
- Denominator can move after Scryfall updates without an app redeploy (within cache TTL).
- Numerator matches the shipped faithful pool (no `approximates`).
- Failures never blank the whole shell or invent a fake percentage.
