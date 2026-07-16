# AGENTS.md

Project instructions for AI coding agents working in this repository.

## What this is

A browser-based 4-player Commander (MTG) game for playing with friends. The **north star is to support *any* card, built faithfully** — no card is out of scope in principle. This is a design posture, not a completeness claim: ~429 card scripts exist today (`crates/cards/data/`), many with `approximates` / `# ponytail:` gaps, and the engine is not rules-complete. Grow the engine and DSL *from real cards* (ADR 0002), TDD, smallest-increment-first, and **flag-don't-force**: when a card needs something the DSL can't yet express, surface it in `docs/FIDELITY_BACKLOG.md` rather than contort the card. The five Secrets of Strixhaven (`soc`, 2026) Commander decks (~389 unique cards) are the **first faithful decks** — the current proving ground, not the terminal scope. See ADR 0014 (and ADR 0012 for the prior step), `.agents/skills/card-dsl/` for card authoring, `.agents/skills/fidelity-grind/` for the deck-to-faithful pipeline (Archidekt link in, fidelity report, grind waves, client catch-up, PR out), and `docs/FIDELITY_BACKLOG.md` for engine work remaining.

## Commands

```
cargo build
cargo nextest run --profile ci           # all tests (via `just server-test`)
cargo nextest run --profile ci <name>    # tests whose name matches <name>
cargo nextest run --profile ci --nocapture  # show println! output from tests
cargo clippy --all-targets      # lint — treat warnings as failures
cargo fmt                       # format before committing
just check                      # format + lint + typecheck + test (both sides)
just format                     # server-format + client-format
just lint                       # server-lint + client-lint
just typecheck                  # client-typecheck
just test                       # server-test + client-test
just migrate                    # apply Toasty migrations (Postgres)
just --group server --list      # server-* recipes only
just --group client --list      # client-* recipes only
just engine-cr-index            # regenerate docs/CR_INDEX.md from engine CR citations
just engine-cr-index-check      # fail if docs/CR_INDEX.md is stale
just deploy                     # apply-machine: roll to tfvars server_image (peers auto)
just tf-apply                   # terraform apply preserving drain peers
just dev                        # tmux: bacon server + client vinxi
```

## Commits & releases

Commits on `main`/`master` follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `chore:`, and `feat!:` / `BREAKING CHANGE:` for majors. [semantic-release](https://semantic-release.org/) is the **only** writer of `v*` tags and GitHub Releases — do not create or push version tags by hand. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) is required so that tag push can trigger `docker.yml` (default `GITHUB_TOKEN` cannot cascade workflows). See [docs/prds/DEPLOYMENT.md](docs/prds/DEPLOYMENT.md).

## Architecture commitments (do not relitigate without reason)

- **Engine:** Pure Rust, deterministic, **sequential state machine** — the stack/priority model, *not* a game-loop. Runs authoritatively on the server.
- **Event-sourced state:** every intent produces events; events mutate board facts. Priority/pass bookkeeping and pending choices are orchestration state in the submit path — preserve intent-replay determinism.
- **Server:** Axum + SSE (server→client state) + POST (client→server intents). Single instance: in-process `tokio::broadcast` fan-out per table (ADR 0005), not Redis. Live games are in-memory only (ADR 0021); Postgres holds users, sessions, decks (ADR 0010). Server-side per-player visibility filtering is a hard rule (hands/libraries are private).
- **Client:** SolidStart 1.3 (Vinxi, `ssr: false`) — hybrid canvas/WebGL board + thin DOM overlay; same-origin `/api` BFF to Axum. **Camera transform** (single source of truth for pan/zoom) and **screen→world hit-testing** are foundations everything downstream assumes.
- **Client state is Effect-first, Solid-second (ADR 0019).** Async work — wire calls, streams, polling — goes through atoms (`effect/unstable/reactivity` + `@effect/atom-solid`); Solid signals/stores are the view layer. Keep `effect` and `@effect/atom-solid` pinned to the same exact beta.
- **Card pool is data-driven scripts.** Let the scripting DSL grow from real cards — resist generalizing it prematurely.
- **Wire types:** OpenAPI codegen from Rust (`openapi.json` → client Effect client). Run `just server-codegen` after schema changes. During a rolling deploy, N and N+1 must coexist until drain empties — see [docs/WIRE_COMPAT.md](docs/WIRE_COMPAT.md) for the expand-only rules and the `/v2` escape hatch.
- **Routing:** Required identifiers belong in **path params** (server: Axum `Path`, client: Solid `:param` segments). **Query params are optional** — filters, paging, redirect targets (`?next=`), and preselection (`?deck=`). Never put a required resource id in a query string.
- **Engine CR lookup:** Start at [`docs/agent-navigation.md`](docs/agent-navigation.md) (module map, `docs/CR_INDEX.md`, regenerate with `just engine-cr-index` / agent hooks).

Crate split: `engine` (pure, no I/O) / `cards` (TOML scripts) / `server` (Axum) / `schema` (wire protocol).

**Reference:** [Forge](https://github.com/Card-Forge/forge) — consult its card scripts and rules implementation for tricky interactions.

## Coding standards (project-specific — enforce these)

- **Readability and maintainability are the top priority**, above cleverness or brevity.
- **Guard-return-first (early return) style.** Handle error/edge/invalid cases up front and `return` (or `?` / `continue`) immediately.
- **TDD is the default workflow.** Use the `tdd` skill. The engine is testable via direct API calls with no UI or network.
- **Every bug fix gets a regression test.** When you find a bug, add a test that fails on the broken behavior and passes with the fix — in the same change if you can. Place it at the lowest layer that catches the failure (engine unit test, schema projection test, client mapping test, HTTP integration test).
- **Keep the engine pure.** No I/O, no networking, no wall-clock or randomness that isn't injected.
- **Use Magic terminology and semantics wherever possible.** The ubiquitous language lives in `CONTEXT.md`; keep code and glossary aligned. When rules and simplicity genuinely conflict, name the rule approximated in a `ponytail:` comment.
