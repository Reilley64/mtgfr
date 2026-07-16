# 0015 — Card imagery via self-hosted CDN

Status: **Accepted**; backfill tooling superseded by [0017](0017-deck-builder-search-over-projected-pool.md).

## Decision

- Optional art CDN (bake `VITE_CARD_CDN` at web image build) serving large webp. `client/src/lib/card-ids.json` maps name → Scryfall id.
- Single `imageUrlByName()` used by builder, hand, board. Unmapped names, or builds without a CDN, fall back to Scryfall `named`.

## Consequences

- Regenerate `card-ids.json` when pool changes (`tooling/backfill-card-meta.mjs` per 0017).
