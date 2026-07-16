# ADRs

Point-in-time decisions. Follow status + related links; ignore superseded parts.

| ADR | Decision | Status |
|-----|----------|--------|
| [0001](0001-rust-to-ts-via-utoipa-openapi-orval.md) | utoipa → OpenAPI wire contract | Superseded (Orval half → 0018) |
| [0002](0002-cards-as-data-driven-effect-enum.md) | Cards as `CardDef` + `Ability` + `Effect` data | Current |
| [0003](0003-additive-continuous-effects-no-layers.md) | Additive P/T/keywords, no CR 613 layers yet | Current |
| [0004](0004-resumable-engine-pending-choice.md) | `pending_choice` pauses engine; answer via intent | Current |
| [0005](0005-in-process-fanout-ndjson-snapshot.md) | In-process broadcast; snapshot-then-deltas | Partial (NDJSON → SSE in 0018; affinity extended in 0030) |
| [0006](0006-client-side-fold-and-choice-framework.md) | Self-sufficient deltas; general `PendingChoice` | Current |
| [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md) | Server auto-pass via `has_meaningful_action` | Current (extended by 0020) |
| [0008](0008-multiplayer-combat-elimination-and-lobby.md) | 4-player combat, elimination, lobby | Current (tokens → 0011) |
| [0009](0009-persistence-and-auth-pulled-into-phase-5.md) | Auth+persistence before pool growth | Historical |
| [0010](0010-postgres-via-toasty.md) | Postgres via Toasty; deck JSON blob | Current (games dropped in 0021) |
| [0011](0011-decks-as-data-and-cookie-identity.md) | Persisted decks; session cookie identity | Current |
| [0012](0012-faithful-precon-pool-scope-reversal.md) | Grow pool to five soc precons | Reframed by 0014 |
| [0013](0013-durable-tables-via-replay-and-spectator-projection.md) | Intent replay + spectator `Option<PlayerId>` | Partial (replay dropped in 0021) |
| [0014](0014-any-card-faithful-scope-reversal.md) | North star: any card faithful | Current |
| [0015](0015-card-imagery-via-self-hosted-cdn-and-name-id-map.md) | Optional `VITE_CARD_CDN` + `card-ids.json` | Current (tooling → 0017) |
| [0016](0016-deck-builder-direct-manipulation-and-card-preview.md) | Direct-manipulation builder + `CardPreview` | Current |
| [0017](0017-deck-builder-search-over-projected-pool.md) | `set`/`subtypes` + Postgres catalog search | Current |
| [0018](0018-effect-generated-client-and-sse-stream.md) | Effect v4 client from OpenAPI; SSE stream | Current |
| [0019](0019-effect-first-client-state-via-atom-solid.md) | Atoms for async; Solid for view | Current |
| [0020](0020-engine-computed-action-lists-with-ids.md) | `LegalAction` list; `TakeAction { id }` | Current (amended 0021, 0022) |
| [0021](0021-live-games-in-memory-only.md) | Stable action ids; no durable games | Current |
| [0022](0022-payment-settles-engine-side-with-auto-tap.md) | `settle_payment` auto-taps lands | Current |
| [0023](0023-biome-as-the-client-toolchain.md) | Biome format/lint; `solid`+`test` domains | Current |
| [0024](0024-tailwind-as-the-design-system-runtime.md) | Tailwind v4 `@theme` from DESIGN.md | Current |
| [0025](0025-modal-pinned-card-inspect.md) | Modal Alt-pinned left inspect dock | Current |
| [0026](0026-helpless-stack-hold-dwell.md) | Helpless hover pauses stack hold (capped) | Current |
| [0027](0027-stack-chrome-next-pass-and-yield.md) | Priority context bar; Next vs Pass vs one-shot stack yield | Current (amended; turn yield → 0029) |
| [0028](0028-battlefield-row-packing-and-clusters.md) | Row packing + permanent clusters (no spill) | Current |
| [0029](0029-turn-yield.md) | Turn yield until active / until intentional action | Current |
| [0030](0030-table-instance-affinity-for-drain-rolls.md) | BFF lobby + `table_routes` → pod DNS; Argo/TF image rolls | Current (extends 0005) |
