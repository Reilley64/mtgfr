# mtgfr

A browser-based **4-player Commander** (EDH) table for playing with friends.

Authoritative rules engine on the server, MTGA-style board in the client, plus auth, lobby, and a deck builder over a growing card pool. One player hosts a table; everyone else claims a seat with their own deck. Eliminated players stay as spectators.

## Status

Early and incomplete on purpose. The north star is to support *any* card **faithfully** — grown from real cards, with gaps flagged rather than faked. Today that means a few hundred scripted cards and an engine that is not rules-complete. The five Secrets of Strixhaven (`soc`) Commander decks are the first proving ground, not the end of the roadmap.

## Stack

| Layer | Tech |
|-------|------|
| Engine | Pure Rust, deterministic stack/priority state machine |
| Cards | Data-driven TOML scripts (`crates/cards/data/`) |
| Server | Axum, SSE for game state, Postgres for users/sessions/decks |
| Client | SolidJS, canvas/WebGL board + thin DOM overlay |

Live games stay in memory on a single server instance. Hands and libraries are filtered server-side — private info never leaves for the wrong seat.

## Local development

```bash
# Postgres (see docker-compose.yml), then:
just migrate
just dev          # bacon server (:8080) + Vite client (:5173)
```

Useful checks:

```bash
just check        # format, lint, typecheck, test (server + client)
just test
just --list
```

## Docs

- [`CONTEXT.md`](CONTEXT.md) — Magic / domain glossary used in the code
- [`docs/FIDELITY_BACKLOG.md`](docs/FIDELITY_BACKLOG.md) — engine work still needed for faithful cards
- [`docs/adr/`](docs/adr/) — architectural decisions
- [`docs/prds/DEPLOYMENT.md`](docs/prds/DEPLOYMENT.md) — production deploy (k3s + Cloudflare Tunnel)
- [`docs/README.md`](docs/README.md) — full docs index

Agent-oriented working notes live in [`AGENTS.md`](AGENTS.md).
