# mtgfr

A browser-based **4-player Commander** (EDH) table for playing with friends.

Authoritative rules engine on the server, MTGA-style board in the client, plus auth, lobby, and a deck builder over a growing card pool. One player hosts a table; everyone else claims a seat with their own deck. Eliminated players stay as spectators.

## Status

Early and incomplete on purpose. The north star is to support *any* card **faithfully** — grown from real cards, with gaps flagged rather than faked. Today that means ~493 scripted cards (`crates/cards/data/`) and an engine that is not rules-complete. The five Secrets of Strixhaven (`soc`) Commander decks (~389 unique cards) are the first proving ground, not the end of the roadmap.

The public origin ships `robots.txt` that disallows crawlers; this is a friends table, not a content site.

## Stack

| Layer | Tech |
|-------|------|
| Engine | Pure Rust, deterministic stack/priority state machine |
| Cards | Data-driven TOML scripts (`crates/cards/data/`) |
| Wire | `.proto` → tonic gRPC (API) + Effect RPC (browser → Nitro BFF) |
| API | tonic gRPC (game / auth / decks / catalog / seed) + Axum `/health/*` only |
| BFF / client | Foldkit SPA on Nitro (Vite); lobby + `table_routes` on Postgres `mtgfr_web` (Drizzle); canvas + Mount bitmap board + thin HTML overlays |
| Durable data | Postgres `mtgfr` (users, sessions, decks); `mtgfr_web` (lobbies, table→pod routes) |
| Deploy | k3s + Cloudflare Tunnel; Argo-owned API/web rolls with SIGTERM drain |

Live games stay **in memory per API process**. Concurrent pods pin each table via BFF `table_routes` → pod DNS. Hands and libraries are filtered server-side — private info never leaves for the wrong seat.

## Local development

```bash
docker compose up -d          # Postgres on :5432 (mtgfr)
# create mtgfr_web if you need lobby persistence locally, then:
just migrate                  # Toasty → mtgfr
just client-migrate           # Drizzle → mtgfr_web (needs WEB_DATABASE_URL)
just dev                      # tmux: bacon server (:8080) + Foldkit/Vite client (default :3000)
```

Without `WEB_DATABASE_URL`, the BFF falls back to localhost for game paths (lobby DB features need the web DB).

Useful checks:

```bash
just check        # format, lint, typecheck, test (server + client)
just test
just --list
```

## Docs

- [`CONTEXT.md`](CONTEXT.md) — Magic / domain glossary used in the code
- [`PRODUCT.md`](PRODUCT.md) / [`DESIGN.md`](DESIGN.md) — product intent and design system
- [`docs/superpowers/specs/`](docs/superpowers/specs/) — feature specs for existing modules (source of truth)
- [`docs/fidelity/`](docs/fidelity/) — per-deck fidelity reports and increments backlogs (created by `fidelity-grind`)
- [`docs/WIRE_COMPAT.md`](docs/WIRE_COMPAT.md) — expand-only proto rules across drain rolls
- [`docs/README.md`](docs/README.md) — full docs index

Agent-oriented working notes live in [`AGENTS.md`](AGENTS.md). Releases are cut by [semantic-release](https://semantic-release.org/) on `main` (PRs are squash-merged — the **PR title** is the release signal).
