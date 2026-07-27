# AGENTS.md

Project instructions for AI coding agents working in this repository.

## What this is

A browser-based 4-player Commander (MTG) game for playing with friends. The **north star is to support *any* card, built faithfully** — no card is out of scope in principle. This is a design posture, not a completeness claim: ~665 deckable card scripts exist today (`crates/cards/data/`), many with `approximates` / `# ponytail:` gaps, and the engine is not rules-complete. Grow the engine and DSL *from real cards*, TDD, smallest-increment-first, and **flag-don't-force**: when a card needs something the DSL can't yet express, surface it in that deck's `docs/fidelity/<slug>-increments.md` (via the `fidelity-grind` skill) rather than contort the card. The five Secrets of Strixhaven (`soc`, 2026) Commander decks (~389 unique cards) are the **first faithful decks** — the current proving ground, not the terminal scope. See [`docs/superpowers/specs/`](docs/superpowers/specs/) (especially card-dsl-and-card-pool), `.agents/skills/card-dsl/` for card authoring, and `.agents/skills/fidelity-grind/` for the deck-to-faithful pipeline (Archidekt link in, per-deck fidelity report + increments backlog, grind waves, client catch-up, PR out).

## Commands

```
cargo build
cargo nextest run --profile ci           # all tests (via `just server-test`)
cargo nextest run --profile ci <name>    # tests whose name matches <name>
cargo nextest run --profile ci --nocapture  # show println! output from tests
cargo clippy --all-targets -- -D warnings  # lint — treat warnings as failures (`just server-lint`)
cargo fmt                       # format before committing
just check                      # format + lint + typecheck + test (both sides)
just format                     # server-format + client-format
just lint                       # server-lint + client-lint
just proto-check                # buf STANDARD lint + WIRE breaking vs origin/main
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

Commits on `main`/`master` follow the [Angular commit message guidelines](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md) (`feat:`, `fix:`, `build:`, `ci:`, `docs:`, `perf:`, `refactor:`, `test`, …; breaking changes via a `BREAKING CHANGE:` footer). [commitlint](https://github.com/conventional-changelog/commitlint) with `@commitlint/config-angular` enforces this on Husky `commit-msg`. In Cursor Cloud, root `npm clean-install` plus `.cursor/scripts/wire-cloud-git-hooks.sh` keep that hook chained through the agent hooks dispatcher. PR CI lint-checks the **PR title** only (squash subject for semantic-release), not every branch commit. [semantic-release](https://semantic-release.org/) is the **only** writer of `v*` tags and GitHub Releases — do not create or push version tags by hand. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) is required so that tag push can trigger `docker.yml` (default `GITHUB_TOKEN` cannot cascade workflows). See [ci-and-release](docs/superpowers/specs/2026-07-20-ci-and-release.md) and [production-topology-and-operations](docs/superpowers/specs/2026-07-20-production-topology-and-operations.md).

**PRs are squash-merged.** The squash commit message on `main` is the **PR title** (plus `(#N)`), not the branch’s individual commits. semantic-release analyzes that squash line only — title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge should cut a release; `build:` / `ci:` / `docs:` / `refactor:` / `test:` / `style:` / `perf:` alone will verify green and skip a version bump.

## Architecture commitments (do not relitigate without reason)

- **Engine:** Pure Rust, deterministic, **sequential state machine** — the stack/priority model, *not* a game-loop. Runs authoritatively on the server.
- **Event-sourced state:** every intent produces events; events mutate board facts. Priority/pass bookkeeping and pending choices are orchestration state in the submit path — preserve intent-replay determinism.
- **Server:** tonic gRPC (game/auth/decks/catalog/seed) + Axum HTTP health only. Live games are in-memory only; Postgres `mtgfr` holds users, sessions, decks. Pre-game lobby + `table_routes` live on the Foldkit SPA's Nitro BFF on Postgres `mtgfr_web` (Drizzle). BFF routes in-game by table id → pod DNS gRPC; seeds hit Service `edh-api` (newest instance only). API/web Deployments are Argo-owned; rolls drain on SIGTERM. Server-side per-player visibility filtering is a hard rule (hands/libraries are private). See [wire](docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md), [lobby/live-game](docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md), [accounts/decks](docs/superpowers/specs/2026-07-20-accounts-decks-and-catalog.md).
- **Client:** Foldkit SPA on Nitro (Vite; single event-reactor `Model`/`update`/`view` in `client/app/`) — hybrid canvas + Mount bitmap board with thin HTML overlays; same-origin Effect RPC (`/api/rpc`) to the BFF, which dials tonic. **Camera transform** (single source of truth for pan/zoom) and **screen→world hit-testing** are foundations everything downstream assumes. Design tokens live in `design.tokens.json` (DTCG); Tailwind `@theme` and canvas TS outputs are generated — see [`DESIGN.md`](DESIGN.md) prose and [shell-routes-and-auth](docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md) / [deck-list-and-builder](docs/superpowers/specs/2026-07-20-deck-list-and-builder.md) / [lobby-entry-ui](docs/superpowers/specs/2026-07-20-lobby-entry-ui.md). See the [spec index](docs/superpowers/specs/README.md), especially [board-composition](docs/superpowers/specs/2026-07-20-board-composition.md), [board-camera-and-layout](docs/superpowers/specs/2026-07-20-board-camera-and-layout.md), and [battlefield](docs/superpowers/specs/2026-07-20-battlefield.md).
- **Client state is Effect-first Foldkit.** Async work — wire calls, streams, polling — stays in Effect services/streams at runtime boundaries; Foldkit `Model`/`update`/`view` owns UI state and dispatches messages. Keep `effect` and `@effect/*` packages pinned to the same exact beta. BFF Drizzle is Effect-native via `drizzle-orm/effect-postgres` + `@effect/sql-pg` (no pg-proxy).
- **Observability:** self-hosted LGTM + Faro + OTEL. Exporters no-op locally unless `OTEL_EXPORTER_OTLP_ENDPOINT` / Faro upstream is set; never put hand/library contents or intent payloads in telemetry. See [observability-ops](docs/superpowers/specs/2026-07-20-observability-ops.md) and [production topology](docs/superpowers/specs/2026-07-20-production-topology-and-operations.md).
- **Card pool is data-driven scripts.** `cards` defines the vocabulary (the enums); `engine` implements the rules around it. Let the scripting DSL grow from real cards — resist generalizing it prematurely. See [card-dsl-and-card-pool](docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md).
- **Wire types:** `.proto` is the sole contract → prost/tonic (`build.rs` → `OUT_DIR`) + Effect-gRPC clients (`just server-codegen` / `bun run gen` → gitignored `client/app/domain/wire/generated/`). Run codegen after proto changes. See [docs/WIRE_COMPAT.md](docs/WIRE_COMPAT.md) and [wire-protocol-and-visibility](docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md).
- **Routing:** Required identifiers belong in **path params** (server: Axum `Path`, client: Foldkit route path segments). **Query params are optional** — filters, paging, redirect targets (`?next=`), and preselection (`?deck=`). Never put a required resource id in a query string.
- **Public crawl posture:** `client/public/robots.txt` disallows all crawlers; do not add sitemaps or marketing SEO without revisiting that choice.
- **Engine CR lookup:** Start at [`docs/agent-navigation.md`](docs/agent-navigation.md) (module map, `docs/CR_INDEX.md`, regenerate with `just engine-cr-index` / agent hooks).
- **Feature specs:** Start at [`docs/superpowers/specs/`](docs/superpowers/specs/) for module behavior (source of truth).
- **Client canvas board:** Start at [`docs/client-canvas-map.md`](docs/client-canvas-map.md) (paint vs hits vs flights vs DOM overlays).

## Feature specs

Specs live in [`docs/superpowers/specs/`](docs/superpowers/specs/). Keep **one spec per code target / feature surface**, not per topic, PR, or wave. Document what exists today: no TBD, no Solid/migration history, and no historical client narrative. When a change splits, merges, or renames a target, update the relevant specs in the same change. Board specs are intentionally fine-grained (composition, battlefield, hand, stack, radial, prompts, inspect, and more); use the [README index](docs/superpowers/specs/README.md). Cite the relevant spec instead of inventing requirements.

**Superpowers workflows (Cursor `superpowers` plugin — do not vendor or fork those skill files into `.agents/skills/`):** When brainstorming, writing plans, or implementing a change to a surface that already has a living module spec (e.g. `hand-and-zone-bar`, `flights`, `turn-and-priority-chrome`), **update that existing spec in the same change** so Behavior / Implementation / Testing still describe what ships. A new `*-design.md` (or a local plan under `docs/superpowers/plans/`, gitignored) is optional design input; it does **not** replace updating the surface spec. Prefer cross-linking the design from the module spec over leaving the module spec stale. Only create a new indexed feature spec when the work introduces a **new** code target / surface that has no home yet.

**Review gate:** Every code review (including autonomous continuous-loop reviews via `requesting-code-review`) must check Feature specs compliance on the diff. Violations — PR/wave-scoped design sidecars *without* the corresponding surface-spec update, migration/history prose in feature specs, shipped behavior missing from the surface spec, or wrong section template — are **merge-blocking**. Implementation plans may be written under `docs/superpowers/plans/` (gitignored; do not commit) and must not be indexed as feature specs.

Crate split: `cards` (the card DSL — `CardDef` / `Effect` / filters / triggers, the TOML surface, and the scripts in `data/`) / `engine` (pure, no I/O; the rules logic over that vocabulary, which it re-exports) / `server` (tonic + health Axum) / `schema` (projection DTOs; mapped to/from native proto at the gRPC edge). Client split: `client/app/` (Foldkit UI), `client/app/domain/` (shared wire/domain helpers), `client/server/` (Nitro BFF routes/plugins), `client/styles/` (Tailwind/design tokens).

**Reference:** [Forge](https://github.com/Card-Forge/forge) — consult its card scripts and rules implementation for tricky interactions.

## Coding standards (project-specific — enforce these)

- **Readability and maintainability are the top priority**, above cleverness or brevity.
- **Guard-return-first (early return) style.** Handle error/edge/invalid cases up front and `return` (or `?` / `continue`) immediately.
- **TDD is the default workflow.** Use the `test-driven-development` skill (superpowers plugin). Red → green → review. The engine is testable via direct API calls with no UI or network.
- **Every bug fix gets a regression test.** When you find a bug, add a test that fails on the broken behavior and passes with the fix — in the same change if you can. Place it at the lowest layer that catches the failure (engine unit test, schema projection test, client mapping test, HTTP integration test). Use `systematic-debugging` when the cause is unclear.
- **Client UI: every surface gets a Scene test.** Shell routes and board overlays must be covered by `data-testid` Scene assertions in `client/app/shell/surfaces.test.ts` and `client/app/board/html/surfaces.test.ts` (plus focused tests). Do not ship a user-visible panel that only has update/logic tests. When adding a surface, add or extend those suites in the same change.
- **Client interaction: assert outcomes, not only presence.** When changing pointer, keyboard, hover, drag, Mount hosts, lobby/host flow, or BFF env defaults, add or extend a unit/Scene test for the user-visible result (pin set, tile hidden, selected deck matches, art URL swapped, default URL works). Do not frame tests as migration/"parity" checks — name the product behavior. See [`docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md`](docs/superpowers/specs/2026-07-22-client-interaction-test-policy-design.md).
- **Interaction / UI PRs.** Check the PR template box when the change touches those surfaces. Before claiming done, run the Interaction checklist in `.agents/skills/verify/SKILL.md` (in addition to `verification-before-completion`).
- **Verify before claiming done.** Use `verification-before-completion` (and the project `verify` skill for live games). Live or Scene checks must exercise the surfaces you changed (cold load, route entry), not only unit green.
- **Keep the engine pure.** No I/O, no networking, no wall-clock or randomness that isn't injected.
- **Use Magic terminology and semantics wherever possible.** The ubiquitous language lives in `CONTEXT.md`; keep code and glossary aligned. When rules and simplicity genuinely conflict, name the rule approximated in a `ponytail:` comment.
- **Client Tailwind: prefer `data-*` + named `group` over JS class ternaries for interactive chrome.** When a tile/button has selected / selectable / pressed / hover-linked styles (raise, ring, hit height, brightness), put stable boolean attrs on the interactive root (`data-selected="true"|"false"`, `data-selectable="true"`, …) and a named group (`group/hand-tile`, `group/pile-card`, …). Encode the look with Tailwind variants — `group-hover/…`, `group-data-[selected=true]/…`, `data-[selected=true]:…` — so JS only sets attributes. Keep playable / zone aura helpers when they are not selection state. Assert the data attrs (and the variant class tokens) in Scene/unit tests rather than reconstructing which ring class a ternary would have emitted. Reference: hand-bar pick chrome in `client/app/board/html/hand.ts` and [`hand-and-zone-bar`](docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md).

## Agent skills

Project / installed skills under `.agents/skills/`: `card-dsl`, `fidelity-grind`, `verify`,
`effect-ts`, `find-skills`, plus Foldkit skills (`foldkit`, `generate-program`, `audit-program`).
`skills-lock.json` tracks only the GitHub-installed entries that remain there (`effect-ts`,
`find-skills`).

Workflow skills from the Cursor **`superpowers`** plugin (enabled in `.cursor/settings.json`):
`brainstorming`, `test-driven-development`, `systematic-debugging`,
`verification-before-completion`, `requesting-code-review`, `writing-plans`, `executing-plans`,
`subagent-driven-development`, `dispatching-parallel-agents`, `using-git-worktrees`,
`finishing-a-development-branch`, `receiving-code-review`, `using-superpowers`, `writing-skills`.

## Cursor Cloud specific instructions

Cloud Agents use the Dockerfile at [`.cursor/Dockerfile`](.cursor/Dockerfile) via [`.cursor/environment.json`](.cursor/environment.json). Do **not** use interactive dashboard “Set up agent” / snapshot setup for this repo — that mode ignores the Dockerfile. If a saved Cloud environment snapshot exists for the repo, delete it so Dockerfile builds win.

The image already has Rust stable (rustfmt/clippy), Bun 1.3.14, Node/npm (`.node-version`), `protoc`, `just`, `cargo-nextest`, and Postgres with `mtgfr` / `mtgfr_web` seeded. `DATABASE_URL` and `WEB_DATABASE_URL` are set. Postgres is started by the environment `start` command. Environment `install` runs root `npm clean-install` (husky + commitlint), wires Cloud git hooks via `.cursor/scripts/wire-cloud-git-hooks.sh`, then client `bun install` and `cargo fetch`.

- Before DB-touching work: `just migrate` (Toasty / `mtgfr`) and/or `just client-migrate` (Drizzle / `mtgfr_web`).
- Prefer `just server-check` / `just client-check` (or `just check`) for verification.
- Put secrets in the Cursor Cloud Agents Secrets UI — do not bake credentials into the image or commit `.env` files.
- Foldkit DevTools MCP uses Vite relay port `9988`; `foldkit_list_runtimes` only sees a runtime while a browser tab has the app open (`devTools: { Message }` in `client/app/entry.ts`).
