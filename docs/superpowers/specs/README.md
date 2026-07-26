# Feature specs — index

Feature specifications for the mtgfr codebase.

## Purpose

These specs are the **source of truth for current feature behavior**. They document what
the system does today, key implementation decisions, testing seams, and out-of-scope gaps.

They absorb the former ADR series (0001–0037) and the former `docs/prds/DEPLOYMENT.md`.
Cite the relevant spec below instead of ADR numbers or invented requirements.

## Companion living docs

These documents are **not superseded** and must stay current alongside the specs:

| Doc | Purpose |
|-----|---------|
| [`CONTEXT.md`](../../../CONTEXT.md) | Domain glossary — ubiquitous language for engine and protocol |
| [`DESIGN.md`](../../../DESIGN.md) | Design system rules / north star (prose) |
| [`design.tokens.json`](../../../design.tokens.json) | DTCG design token source of truth |
| [`PRODUCT.md`](../../../PRODUCT.md) | Product positioning and anti-references |
| [`docs/fidelity/`](../../fidelity/) | Per-deck fidelity reports and increments backlogs (`fidelity-grind`) |
| [`docs/WIRE_COMPAT.md`](../../WIRE_COMPAT.md) | Expand-only proto field rules during drain rolls |
| [`docs/agent-navigation.md`](../../agent-navigation.md) | Engine module ↔ CR navigation |
| [`docs/CR_INDEX.md`](../../CR_INDEX.md) | Generated CR citation index (`just engine-cr-index`) |
| [`docs/client-canvas-map.md`](../../client-canvas-map.md) | Canvas board paint / hits / flights / overlay map |

## Spec list

| Spec | Domain |
|------|--------|
| [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md) | Auth, sessions, deck storage, legality, catalog projection |
| [action-session-and-targeting](2026-07-20-action-session-and-targeting.md) | Board action planning, local prompts, targets, submit pipeline |
| [activation-menu](2026-07-25-activation-menu.md) | Battlefield permanent activation menu, placement, costs, and commit behavior |
| [battlefield](2026-07-20-battlefield.md) | Battlefield paint, permanent chrome, avatars, arrows, packing |
| [board-camera-and-layout](2026-07-20-board-camera-and-layout.md) | Camera transform, screen/world geometry, seat and zone layout |
| [board-composition](2026-07-20-board-composition.md) | Board submodel, Canvas/Mount/HTML surfaces, overlay composition |
| [board-log-panel](2026-07-25-board-log-panel.md) | Board event log panel above the hand bar |
| [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md) | TOML card scripts, effect vocabulary, precons, fidelity posture |
| [card-inspect](2026-07-20-card-inspect.md) | Alt/Option dock inspect, catalog fetch, modifier ledger |
| [choices-actions-and-resolution](2026-07-20-choices-actions-and-resolution.md) | Engine choices, legal actions, payment, resolution |
| [ci-and-release](2026-07-20-ci-and-release.md) | GitHub Actions verify/release/docker, Buildx GHA cache, semantic-release |
| [combat-and-commander-rules](2026-07-20-combat-and-commander-rules.md) | Multiplayer combat, commander tax/damage, elimination |
| [coverage-by-set](2026-07-26-coverage-by-set.md) | Auth-gated `/coverage` set table, search/filter, BFF coverage meta join, faithful-by-set health input |
| [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) | Deck list tiles, search, builder split-pane, card art CDN |
| [engine-core-and-event-model](2026-07-20-engine-core-and-event-model.md) | Pure Rust engine zones, events, SBAs, determinism |
| [flights](2026-07-20-flights.md) | Card movement animation, flight ownership, bitmap paint gating |
| [foldkit-devtools](2026-07-22-foldkit-devtools.md) | Local Foldkit runtime relay and MCP debugging tools |
| [hand-and-zone-bar](2026-07-20-hand-and-zone-bar.md) | Hand, command, graveyard, exile bars and playable outlines |
| [lobby-entry-ui](2026-07-20-lobby-entry-ui.md) | Lobby Host/Join entry, seated chrome, poll, table code, Ready audio unlock |
| [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md) | Lobby, seed, in-memory tables, affinity, drain |
| [mana-tray](2026-07-20-mana-tray.md) | Battlefield mana pool tray and payment mana tray surfaces |
| [observability-ops](2026-07-20-observability-ops.md) | LGTM/Alloy/Grafana ops, Faro, BFF OTEL, scrub rules |
| [production-topology-and-operations](2026-07-20-production-topology-and-operations.md) | k3s, Argo, Tunnel, migrations, Dockerfiles, health/drain |
| [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md) | Pending-choice forms, X prompt, modal and local cost prompts |
| [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) | Shell routes, portrait gate, auth, Foldkit/wire edge, design tokens, brand |
| [stack](2026-07-20-stack.md) | Stack pile, strip, expansion, targetability, auto-resolve pause |
| [system-overlays](2026-07-20-system-overlays.md) | Result, concede, pile expansion, reconnect, portrait gate overlays |
| [table-audio](2026-07-20-table-audio.md) | Synthesized table cues, browser unlock, sound toggle |
| [turn-and-priority-chrome](2026-07-20-turn-and-priority-chrome.md) | Phase, priority, pass/yield, hints, playable chrome, mulligan overlay |
| [turn-priority-and-stack](2026-07-20-turn-priority-and-stack.md) | Engine turn structure, priority, stack, auto-pass, yields |
| [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) | Proto contract, redaction, snapshot/delta stream |

## Process / policy (not a product surface)

| Doc | Domain |
|-----|--------|
| [client-interaction-test-policy](2026-07-22-client-interaction-test-policy-design.md) | Outcome-focused client interaction test policy (process doc; keep) |
| [engine-refactor-program](2026-07-25-engine-refactor-program-design.md) | Design input for engine Waves A–F (interned defs, obligations, triggers, choices, CR 613/614); update living surface specs per wave |
| [sba-fidelity-gaps](2026-07-26-sba-fidelity-gaps-design.md) | Design input for CR 704.5j legend rule + CR 704.5r counter annihilation; update living engine-core / choices / prompts / wire specs |
| [pool-coverage-badge](2026-07-26-pool-coverage-badge-design.md) | Design input for shell `% faithful` coverage above API version (faithful pool ÷ Scryfall oracle total); update shell-routes / meta at implement time |
| [coverage-by-set](2026-07-26-coverage-by-set-design.md) | Design input for `/coverage` set table (faithful-by-set ÷ Scryfall per-set oracle); badge navigates; depends on pool-coverage-badge |
| [gha-server-verify-sharding](2026-07-26-gha-server-verify-sharding-design.md) | Design input for 3-shard nextest cold `verify-server` wall-clock; update `ci-and-release` at implement time |

## Authoring conventions

Each spec uses: Problem Statement → Solution → User Stories → Behavior → Implementation
Decisions → Testing Decisions → Out of Scope → Further Notes.

- Keep **one spec per code target / feature surface**, not per topic, PR, or wave.
- Document **current behavior only**: no TBD, no Solid/migration history, no historical client narrative.
- When a target splits, merges, or renames, update the relevant specs in the same change.
- Follow the [`AGENTS.md` Feature specs section](../../../AGENTS.md#feature-specs).
- **Superpowers:** If the change touches an existing surface, update that living module spec
  in the same change. A new `*-design.md` is design input only — it does not replace the
  surface-spec update (see AGENTS.md). Do not edit Cursor `superpowers` plugin skill files for
  this policy; project rules in `AGENTS.md` govern.
- Use [`CONTEXT.md`](../../../CONTEXT.md) vocabulary. Reference DTCG token names
  (`design.tokens.json`); canvas uses `design-tokens.generated.ts` for named colors; unnamed
  paint literals remain the exception.
- **PR / loop review:** Code review must fail merge on PR-scoped design sidecars without the
  corresponding surface-spec update, missing surface-spec updates, or non-current narrative
  in this directory (see [`AGENTS.md` Feature specs](../../../AGENTS.md#feature-specs)). Plans
  may be written under `docs/superpowers/plans/` locally (gitignored), not here.
