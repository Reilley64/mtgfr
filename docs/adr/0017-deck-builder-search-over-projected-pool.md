# 0017 — Deck-builder search over projected pool

Status: **Accepted**; supersedes ADR 0015 throwaway resolver tooling.

## Decision

- `CardDef` gains `set`, `subtypes`, and `otags` (catalog metadata; `#[serde(default)]`).
- Boot projects `cards::registry()` into Postgres `catalog_cards` (search blob + wire JSON).
- `GET /cards/search` tokenized `LIKE`; `GET /cards/lookup?names=` for deck hydration.
- Backfill via `tooling/backfill-card-meta.mjs` from Scryfall.

## Consequences

- Client never holds full catalog. Raw SQL via Toasty escape hatch (`$1`/`?1` per backend).
- Re-run backfill when pool changes.
- `otags` (Scryfall oracle-tag slugs) added as catalog metadata; backfilled via `tooling/backfill-otags.mjs`; folded into `search_blob` for thematic deck-builder search.
