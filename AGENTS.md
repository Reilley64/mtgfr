# AGENTS.md

Project instructions for AI coding agents working in this repository.

## What this is

A browser-based 4-player Commander (MTG) game for playing with friends. The **north star is to support *any* card, built faithfully** — no card is out of scope in principle. This is a design posture, not a completeness claim: ~493 card scripts exist today (`crates/cards/data/`), many with `approximates` / `# ponytail:` gaps, and the engine is not rules-complete. Grow the engine and DSL *from real cards*, TDD, smallest-increment-first, and **flag-don't-force**: when a card needs something the DSL can't yet express, surface it in that deck's `docs/fidelity/<slug>-increments.md` (via the `fidelity-grind` skill) rather than contort the card. The five Secrets of Strixhaven (`soc`, 2026) Commander decks (~389 unique cards) are the **first faithful decks** — the current proving ground, not the terminal scope. See [`docs/superpowers/specs/`](docs/superpowers/specs/) (especially card-dsl-and-card-pool), `.agents/skills/card-dsl/` for card authoring, and `.agents/skills/fidelity-grind/` for the deck-to-faithful pipeline (Archidekt link in, per-deck fidelity report + increments backlog, grind waves, client catch-up, PR out).

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
just dev                        # tmux: bacon server + Foldkit/Vite client
```

## Commits & releases

Commits on `main`/`master` follow the [Angular commit message guidelines](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md) (`feat:`, `fix:`, `build:`, `ci:`, `docs:`, `perf:`, `refactor:`, `test`, …; breaking changes via a `BREAKING CHANGE:` footer). [commitlint](https://github.com/conventional-changelog/commitlint) with `@commitlint/config-angular` enforces this on `commit-msg` (Husky). [semantic-release](https://semantic-release.org/) is the **only** writer of `v*` tags and GitHub Releases — do not create or push version tags by hand. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) is required so that tag push can trigger `docker.yml` (default `GITHUB_TOKEN` cannot cascade workflows). See [production-topology-and-operations](docs/superpowers/specs/2026-07-20-production-topology-and-operations.md).

**PRs are squash-merged.** The squash commit message on `main` is the **PR title** (plus `(#N)`), not the branch’s individual commits. semantic-release analyzes that squash line only — title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge should cut a release; `build:` / `ci:` / `docs:` / `refactor:` / `test:` / `style:` / `perf:` alone will verify green and skip a version bump.

## Architecture commitments (do not relitigate without reason)

- **Engine:** Pure Rust, deterministic, **sequential state machine** — the stack/priority model, *not* a game-loop. Runs authoritatively on the server.
- **Event-sourced state:** every intent produces events; events mutate board facts. Priority/pass bookkeeping and pending choices are orchestration state in the submit path — preserve intent-replay determinism.
- **Server:** tonic gRPC (game/auth/decks/catalog/seed) + Axum HTTP health only. Live games are in-memory only; Postgres `mtgfr` holds users, sessions, decks. Pre-game lobby + `table_routes` live on the Foldkit SPA's Nitro BFF on Postgres `mtgfr_web` (Drizzle). BFF routes in-game by table id → pod DNS gRPC; seeds hit Service `edh-api` (newest instance only). API/web Deployments are Argo-owned; rolls drain on SIGTERM. Server-side per-player visibility filtering is a hard rule (hands/libraries are private). See [wire](docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md), [lobby/live-game](docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md), [accounts/decks](docs/superpowers/specs/2026-07-20-accounts-decks-and-catalog.md).
- **Client:** Foldkit SPA on Nitro (Vite; single event-reactor `Model`/`update`/`view` in `client/app/`) — hybrid canvas + Mount bitmap board with thin HTML overlays; same-origin Effect RPC (`/api/rpc`) to the BFF, which dials tonic. **Camera transform** (single source of truth for pan/zoom) and **screen→world hit-testing** are foundations everything downstream assumes. Design tokens live in [`DESIGN.md`](DESIGN.md) / `client/styles/global.css`. See [client board](docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md), [client shell](docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md), and [Foldkit migration design](docs/superpowers/specs/2026-07-21-foldkit-client-migration-design.md).
- **Client state is Effect-first Foldkit.** Async work — wire calls, streams, polling — stays in Effect services/streams at runtime boundaries; Foldkit `Model`/`update`/`view` owns UI state and dispatches messages. Keep `effect` and `@effect/*` packages pinned to the same exact beta. BFF Drizzle uses `@effect/sql-pg` (pg-proxy).
- **Observability:** self-hosted LGTM + Faro + OTEL. Exporters no-op locally unless `OTEL_EXPORTER_OTLP_ENDPOINT` / Faro upstream is set; never put hand/library contents or intent payloads in telemetry. See [production topology](docs/superpowers/specs/2026-07-20-production-topology-and-operations.md).
- **Card pool is data-driven scripts.** Let the scripting DSL grow from real cards — resist generalizing it prematurely. See [card-dsl-and-card-pool](docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md).
- **Wire types:** `.proto` is the sole contract → prost/tonic (`build.rs` → `OUT_DIR`) + Effect-gRPC clients (`just server-codegen` / `bun run gen` → gitignored `client/lib/wire/generated/`). Run codegen after proto changes. See [docs/WIRE_COMPAT.md](docs/WIRE_COMPAT.md) and [wire-protocol-and-visibility](docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md).
- **Routing:** Required identifiers belong in **path params** (server: Axum `Path`, client: Foldkit route path segments). **Query params are optional** — filters, paging, redirect targets (`?next=`), and preselection (`?deck=`). Never put a required resource id in a query string.
- **Public crawl posture:** `client/public/robots.txt` disallows all crawlers; do not add sitemaps or marketing SEO without revisiting that choice.
- **Engine CR lookup:** Start at [`docs/agent-navigation.md`](docs/agent-navigation.md) (module map, `docs/CR_INDEX.md`, regenerate with `just engine-cr-index` / agent hooks).
- **Feature specs:** Start at [`docs/superpowers/specs/`](docs/superpowers/specs/) for module behavior (source of truth).
- **Client canvas board:** Start at [`docs/client-canvas-map.md`](docs/client-canvas-map.md) (paint vs hits vs flights vs DOM overlays).

Crate split: `engine` (pure, no I/O) / `cards` (TOML scripts) / `server` (tonic + health Axum) / `schema` (projection DTOs; mapped to/from native proto at the gRPC edge). Client split: `client/app/` (Foldkit UI), `client/lib/` (shared wire/domain helpers), `client/server/` (Nitro BFF routes/plugins), `client/styles/` (Tailwind/design tokens).

**Reference:** [Forge](https://github.com/Card-Forge/forge) — consult its card scripts and rules implementation for tricky interactions.

## Coding standards (project-specific — enforce these)

- **Readability and maintainability are the top priority**, above cleverness or brevity.
- **Guard-return-first (early return) style.** Handle error/edge/invalid cases up front and `return` (or `?` / `continue`) immediately.
- **TDD is the default workflow.** Use the `test-driven-development` skill (obra/superpowers). Red → green → review. The engine is testable via direct API calls with no UI or network.
- **Every bug fix gets a regression test.** When you find a bug, add a test that fails on the broken behavior and passes with the fix — in the same change if you can. Place it at the lowest layer that catches the failure (engine unit test, schema projection test, client mapping test, HTTP integration test). Use `systematic-debugging` when the cause is unclear.
- **Verify before claiming done.** Use `verification-before-completion` (and the project `verify` skill for live games).
- **Keep the engine pure.** No I/O, no networking, no wall-clock or randomness that isn't injected.
- **Use Magic terminology and semantics wherever possible.** The ubiquitous language lives in `CONTEXT.md`; keep code and glossary aligned. When rules and simplicity genuinely conflict, name the rule approximated in a `ponytail:` comment.

## Agent skills

Project skills: `card-dsl`, `fidelity-grind`, `verify`, `effect-ts`.

Workflow skills from **obra/superpowers** (see `skills-lock.json`): `brainstorming`,
`test-driven-development`, `systematic-debugging`, `verification-before-completion`,
`requesting-code-review`, `writing-plans`, `executing-plans`, `subagent-driven-development`,
`dispatching-parallel-agents`, `using-git-worktrees`, `finishing-a-development-branch`,
`receiving-code-review`, `using-superpowers`, `writing-skills`.

## Cursor Cloud specific instructions

Cloud Agents use the Dockerfile at [`.cursor/Dockerfile`](.cursor/Dockerfile) via [`.cursor/environment.json`](.cursor/environment.json). Do **not** use interactive dashboard “Set up agent” / snapshot setup for this repo — that mode ignores the Dockerfile. If a saved Cloud environment snapshot exists for the repo, delete it so Dockerfile builds win.

The image already has Rust stable (rustfmt/clippy), Bun 1.3.14, `protoc`, `just`, `cargo-nextest`, and Postgres with `mtgfr` / `mtgfr_web` seeded. `DATABASE_URL` and `WEB_DATABASE_URL` are set. Postgres is started by the environment `start` command.

- Before DB-touching work: `just migrate` (Toasty / `mtgfr`) and/or `just client-migrate` (Drizzle / `mtgfr_web`).
- Prefer `just server-check` / `just client-check` (or `just check`) for verification.
- Put secrets in the Cursor Cloud Agents Secrets UI — do not bake credentials into the image or commit `.env` files.
