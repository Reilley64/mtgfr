# OpenSpec capabilities

Living requirements for mtgfr. Run `openspec list --specs` or open a capability folder.

| Capability | Covers |
|------------|--------|
| [engine](engine/spec.md) | Pure deterministic rules engine: zones, events, SBAs, turns, priority, choices, combat, Commander |
| [card-dsl](card-dsl/spec.md) | TOML card/token authoring, vocabulary growth, fidelity, legality identity |
| [wire-protocol](wire-protocol/spec.md) | Proto contract, visibility, stream frames, expand-only compatibility |
| [accounts-and-catalog](accounts-and-catalog/spec.md) | Auth, decks, ratings, catalog projection |
| [lobby-and-live-game](lobby-and-live-game/spec.md) | Lobby BFF, seed, in-memory tables, pod affinity, drain |
| [client-shell](client-shell/spec.md) | Foldkit shell routes, auth UI, tokens, PWA, coverage, UI components, interaction tests |
| [deck-builder](deck-builder/spec.md) | Deck list and builder, printings, art URLs |
| [game-board](game-board/spec.md) | Lobby entry UI and in-game board surfaces |
| [production-and-ops](production-and-ops/spec.md) | k3s topology, CI/release, observability, card CDN |

Project context for agents: [`../config.yaml`](../config.yaml).

Companion docs outside OpenSpec: `CONTEXT.md`, `DESIGN.md`, `PRODUCT.md`, `docs/fidelity/`, `docs/WIRE_COMPAT.md`, `docs/AGENT_NAVIGATION.md`, `docs/CLIENT_CANVAS_MAP.md`.
