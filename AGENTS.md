# AGENTS.md

Project instructions for AI coding agents working in this repository.

## What this is

A browser-based 4-player Commander (MTG) game for playing with friends. The **north star is to support *any* card, built faithfully** — no card is out of scope in principle. This is a design posture, not a completeness claim: ~493 card scripts exist today (`crates/cards/data/`), many with `approximates` / `# ponytail:` gaps, and the engine is not rules-complete. Grow the engine and DSL *from real cards* (ADR 0002), TDD, smallest-increment-first, and **flag-don't-force**: when a card needs something the DSL can't yet express, surface it in `docs/FIDELITY_BACKLOG.md` rather than contort the card. The five Secrets of Strixhaven (`soc`, 2026) Commander decks (~389 unique cards) are the **first faithful decks** — the current proving ground, not the terminal scope. See ADR 0014 (and ADR 0012 for the prior step), `.agents/skills/card-dsl/` for card authoring, `.agents/skills/fidelity-grind/` for the deck-to-faithful pipeline (Archidekt link in, fidelity report, grind waves, client catch-up, PR out), and `docs/FIDELITY_BACKLOG.md` for engine work remaining.

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
just client-migrate             # Drizzle migrations for mtgfr_web (WEB_DATABASE_URL)
just dev                        # tmux: bacon server + client vinxi
```

## Commits & releases

Commits on `main`/`master` follow the [Angular commit message guidelines](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md) (`feat:`, `fix:`, `build:`, `ci:`, `docs:`, `perf:`, `refactor:`, `test`, …; breaking changes via a `BREAKING CHANGE:` footer). [commitlint](https://github.com/conventional-changelog/commitlint) with `@commitlint/config-angular` enforces this on `commit-msg` (Husky). [semantic-release](https://semantic-release.org/) is the **only** writer of `v*` tags and GitHub Releases — do not create or push version tags by hand. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) is required so that tag push can trigger `docker.yml` (default `GITHUB_TOKEN` cannot cascade workflows). See [docs/prds/DEPLOYMENT.md](docs/prds/DEPLOYMENT.md).

**PRs are squash-merged.** The squash commit message on `main` is the **PR title** (plus `(#N)`), not the branch’s individual commits. semantic-release analyzes that squash line only — title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge should cut a release; `build:` / `ci:` / `docs:` / `refactor:` / `test:` / `style:` / `perf:` alone will verify green and skip a version bump.

## Architecture commitments (do not relitigate without reason)

- **Engine:** Pure Rust, deterministic, **sequential state machine** — the stack/priority model, *not* a game-loop. Runs authoritatively on the server.
- **Event-sourced state:** every intent produces events; events mutate board facts. Priority/pass bookkeeping and pending choices are orchestration state in the submit path — preserve intent-replay determinism.
- **Server:** tonic gRPC (game/auth/decks/catalog/seed) + Axum HTTP health only (ADR 0032). Live games are in-memory only (ADR 0021); Postgres `mtgfr` holds users, sessions, decks (ADR 0010). Pre-game lobby + `table_routes` live in SolidStart on Postgres `mtgfr_web` (Drizzle). BFF routes in-game by table id → pod DNS gRPC; seeds hit Service `edh-api` (newest instance only). API/web Deployments are Argo-owned; rolls drain on SIGTERM (ADR 0030). Server-side per-player visibility filtering is a hard rule (hands/libraries are private).
- **Client:** SolidStart 1.3 (Vinxi, `ssr: false`) — hybrid canvas/WebGL board + thin DOM overlay; same-origin Effect RPC (`/api/rpc`) to the BFF, which dials tonic. **Camera transform** (single source of truth for pan/zoom) and **screen→world hit-testing** are foundations everything downstream assumes. Design tokens live in [`DESIGN.md`](DESIGN.md) / `client/src/global.css`.
- **Client state is Effect-first, Solid-second (ADR 0019).** Async work — wire calls, streams, polling — goes through atoms (`effect/unstable/reactivity` + `@effect/atom-solid`); Solid signals/stores are the view layer. Keep `effect`, `@effect/atom-solid`, and `@effect/sql-pg` pinned to the same exact beta. BFF Drizzle uses `@effect/sql-pg` (pg-proxy).
- **Observability:** self-hosted LGTM + Faro + OTEL (ADR 0034). Exporters no-op locally unless `OTEL_EXPORTER_OTLP_ENDPOINT` / Faro upstream is set; never put hand/library contents or intent payloads in telemetry.
- **Card pool is data-driven scripts.** Let the scripting DSL grow from real cards — resist generalizing it prematurely.
- **Wire types:** `.proto` is the sole contract (ADR 0032) → prost/tonic (`build.rs` → `OUT_DIR`) + Effect-gRPC clients (`just server-codegen` / `bun run gen` → gitignored `client/src/wire/generated/`). Run codegen after proto changes. See [docs/WIRE_COMPAT.md](docs/WIRE_COMPAT.md) for expand-only proto field rules.
- **Routing:** Required identifiers belong in **path params** (server: Axum `Path`, client: Solid `:param` segments). **Query params are optional** — filters, paging, redirect targets (`?next=`), and preselection (`?deck=`). Never put a required resource id in a query string.
- **Public crawl posture:** `client/public/robots.txt` disallows all crawlers; do not add sitemaps or marketing SEO without revisiting that choice.
- **Engine CR lookup:** Start at [`docs/agent-navigation.md`](docs/agent-navigation.md) (module map, `docs/CR_INDEX.md`, regenerate with `just engine-cr-index` / agent hooks).
- **Client canvas board:** Start at [`docs/client-canvas-map.md`](docs/client-canvas-map.md) (paint vs hits vs flights vs DOM overlays).

Crate split: `engine` (pure, no I/O) / `cards` (TOML scripts) / `server` (tonic + health Axum) / `schema` (projection DTOs; mapped to/from native proto at the gRPC edge).

**Reference:** [Forge](https://github.com/Card-Forge/forge) — consult its card scripts and rules implementation for tricky interactions.

## Coding standards (project-specific — enforce these)

- **Readability and maintainability are the top priority**, above cleverness or brevity.
- **Guard-return-first (early return) style.** Handle error/edge/invalid cases up front and `return` (or `?` / `continue`) immediately.
- **TDD is the default workflow.** Use the `tdd` skill. The engine is testable via direct API calls with no UI or network.
- **Every bug fix gets a regression test.** When you find a bug, add a test that fails on the broken behavior and passes with the fix — in the same change if you can. Place it at the lowest layer that catches the failure (engine unit test, schema projection test, client mapping test, HTTP integration test).
- **Keep the engine pure.** No I/O, no networking, no wall-clock or randomness that isn't injected.
- **Use Magic terminology and semantics wherever possible.** The ubiquitous language lives in `CONTEXT.md`; keep code and glossary aligned. When rules and simplicity genuinely conflict, name the rule approximated in a `ponytail:` comment.

## Cursor Cloud specific instructions

Cloud Agents use the Dockerfile at [`.cursor/Dockerfile`](.cursor/Dockerfile) via [`.cursor/environment.json`](.cursor/environment.json). Do **not** use interactive dashboard “Set up agent” / snapshot setup for this repo — that mode ignores the Dockerfile. If a saved Cloud environment snapshot exists for the repo, delete it so Dockerfile builds win.

The image already has Rust stable (rustfmt/clippy), Bun 1.3.14, `protoc`, `just`, `cargo-nextest`, and Postgres with `mtgfr` / `mtgfr_web` seeded. `DATABASE_URL` and `WEB_DATABASE_URL` are set. Postgres is started by the environment `start` command.

- Before DB-touching work: `just migrate` (Toasty / `mtgfr`) and/or `just client-migrate` (Drizzle / `mtgfr_web`).
- Prefer `just server-check` / `just client-check` (or `just check`) for verification.
- Put secrets in the Cursor Cloud Agents Secrets UI — do not bake credentials into the image or commit `.env` files.
