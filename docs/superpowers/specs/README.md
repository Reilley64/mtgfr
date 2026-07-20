# Feature specs — index

Reverse-engineered feature specifications for the mtgfr codebase, written as of 2026-07-20.

## Purpose

These specs are the **source of truth for existing module behavior**. They document what
the system does today, key implementation decisions, testing seams, and out-of-scope gaps.

They absorb the former ADR series (0001–0037) and the former `docs/prds/DEPLOYMENT.md`.
Cite the relevant spec below instead of ADR numbers.

## Companion living docs

These documents are **not superseded** and must stay current alongside the specs:

| Doc | Purpose |
|-----|---------|
| [`CONTEXT.md`](../../../CONTEXT.md) | Domain glossary — ubiquitous language for engine and protocol |
| [`DESIGN.md`](../../../DESIGN.md) | Design token source of truth |
| [`PRODUCT.md`](../../../PRODUCT.md) | Product positioning and anti-references |
| [`docs/fidelity/`](../../fidelity/) | Per-deck fidelity reports and increments backlogs (`fidelity-grind`) |
| [`docs/WIRE_COMPAT.md`](../../WIRE_COMPAT.md) | Expand-only proto field rules during drain rolls |
| [`docs/agent-navigation.md`](../../agent-navigation.md) | Engine module ↔ CR navigation |
| [`docs/CR_INDEX.md`](../../CR_INDEX.md) | Generated CR citation index (`just engine-cr-index`) |
| [`docs/client-canvas-map.md`](../../client-canvas-map.md) | Canvas board paint / hits / flights / overlay map |

## Spec list

| Spec | Domain |
|------|--------|
| [engine-core-and-event-model](2026-07-20-engine-core-and-event-model.md) | Pure Rust engine — zones, events, SBAs, determinism |
| [turn-priority-and-stack](2026-07-20-turn-priority-and-stack.md) | Turn structure, priority, stack, auto-pass, yields, End Turn |
| [combat-and-commander-rules](2026-07-20-combat-and-commander-rules.md) | Multiplayer combat, commander tax/damage, elimination |
| [choices-actions-and-resolution](2026-07-20-choices-actions-and-resolution.md) | Pending choices, legal actions, payment, resolution |
| [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md) | TOML card scripts, Effect vocabulary, precons, fidelity posture |
| [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) | Proto contract, redaction, snapshot/delta stream |
| [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md) | Auth, decks, legality, catalog search, Postgres `mtgfr` |
| [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md) | Lobby, seed, in-memory tables, affinity, drain |
| [client-game-board-and-interaction](2026-07-20-client-game-board-and-interaction.md) | Canvas board, targeting, flights, chrome, audio, inspect |
| [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) | Routes, atoms, deck builder, CDN, Faro/OTEL, design system |
| [production-topology-and-operations](2026-07-20-production-topology-and-operations.md) | k3s, Argo, Tunnel, migrations, releases, LGTM |

## Authoring conventions

Each spec uses: Problem Statement → Solution → User Stories → Behavior → Implementation
Decisions → Testing Decisions → Out of Scope → Further Notes.

**No TBD.** Investigate the code before writing. Specs document what exists, not what is
planned. Use [`CONTEXT.md`](../../../CONTEXT.md) vocabulary. Reference DESIGN.md token
names rather than raw hex (canvas hex literals are the documented exception).
