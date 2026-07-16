# 0022 — Payment settles engine-side with auto-tap

Status: **Accepted**; amends [0020](0020-engine-computed-action-lists-with-ids.md).

## Decision

- `Game::settle_payment` — verify all legality first, then auto-tap mana sources for shortfall (pool first; free taps including lands, rocks, and dorks before paid taps so a filter/signet cannot burn a required color; then paid tap-for-mana abilities such as filter lands/karoos/signets via a feed-first plan so nested activation settle only spends; lands preferred over non-lands). Net-zero converters (Study Hall) stay manual on the radial and are skipped by the planner/`available_mana`.
- All casts/activations/cycling/pay-cost choices route through verify-then-settle.
- Client deletes payment planning; cast = single intent. Manual tap-for-mana remains for floating free mana; paid mana abilities appear as Activate actions on the radial (not meaningful for auto-pass). Snapshot actions carry `auto_tap` object ids from the same planner for hover preview (feeders + filter when a paid ability is planned).

## Consequences

- "Listed as affordable" matches actual payment. Tap events in same delta as cast.
