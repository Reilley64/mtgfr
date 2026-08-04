# AGENTS.md

Project instructions for AI coding agents working in this repository.

## What this is

A browser-based 4-player Commander (MTG) game for playing with friends. The **north star is to support *any* card, built faithfully** — no card is out of scope in principle. This is a design posture, not a completeness claim: the scripts in `crates/cards/data/` cover a fraction of Magic, many with `approximates` / `# ponytail:` gaps, and the engine is not rules-complete.

Grow the engine and DSL *from real cards*, TDD, smallest-increment-first, and **flag-don't-force**: when a card needs something the DSL can't yet express, surface it in that deck's `docs/fidelity/<slug>-increments.md` rather than contort the card. Decks are taken to faithful one at a time — `docs/fidelity/` is the record of which have been ground and where each stands. Each is a proving ground, not the terminal scope.

Card authoring: `.agents/skills/card-dsl/`. Deck-to-faithful pipeline (Archidekt link in, per-deck fidelity report + increments backlog, grind waves, client catch-up, PR out): `.agents/skills/fidelity-grind/`. For tricky rules interactions, use the `forge` skill (vendored sparse tree at `.repos/forge`, refresh with `just forge`) — [Card-Forge/forge](https://github.com/Card-Forge/forge) card/token scripts are the reference. **Prefer composable effects** — reuse and combine existing DSL leaves (`sequence`, filters, shared modes, amounts) rather than minting a one-off effect per card; grow the vocabulary only when a real card cannot be expressed from what already exists.

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
just openspec-check             # openspec validate --all --strict (living specs + active changes)
just forge                      # sync vendored Card-Forge/forge → .repos/forge (commit the diff)
just typecheck                  # client-typecheck
just test                       # server-test + client-test
just migrate                    # apply Toasty migrations (Postgres)
just --group server --list      # server-* recipes only
just --group client --list      # client-* recipes only
just engine-cr-index            # regenerate docs/CR_INDEX.md from engine CR citations
just engine-cr-index-check      # fail if docs/CR_INDEX.md is stale
just client-migrate             # Drizzle migrations for mtgfr_web (WEB_DATABASE_URL)
just dev                        # tmux: bacon server + Foldkit/Vite client
ast-grep --help                 # structural search/rewrite (prefer over text grep for AST-shaped queries)
ast-grep run -l rust -p '<pattern>' crates/engine
ast-grep run -l typescript -p '<pattern>' client/app
```

Prefer **`ast-grep`** (`ast-grep`, not deprecated `sg`) when the question is structural — match/call/import shapes in Rust or TypeScript — and **`rg`** for plain text, CR cites, and comments. See [`docs/AGENT_NAVIGATION.md`](docs/AGENT_NAVIGATION.md).

## Commits & releases

Commits on `main`/`master` follow the [Angular commit message guidelines](https://github.com/angular/angular/blob/main/contributing-docs/commit-message-guidelines.md) (`feat:`, `fix:`, `build:`, `ci:`, `docs:`, `perf:`, `refactor:`, `test`, …; breaking changes via a `BREAKING CHANGE:` footer). [commitlint](https://github.com/conventional-changelog/commitlint) with `@commitlint/config-angular` enforces this on Husky `commit-msg`. In Cursor Cloud, root `npm clean-install` plus `.cursor/scripts/wire-cloud-git-hooks.sh` keep that hook chained through the agent hooks dispatcher. PR CI lint-checks the **PR title** only (squash subject for semantic-release), not every branch commit. [semantic-release](https://semantic-release.org/) is the **only** writer of `v*` tags and GitHub Releases — do not create or push version tags by hand. Repo secret `RELEASE_TOKEN` (PAT with `contents` + `workflow`) is required so that tag push can trigger `docker.yml` (default `GITHUB_TOKEN` cannot cascade workflows). See [production-and-ops](openspec/specs/production-and-ops/spec.md).

**PRs are squash-merged.** The squash commit message on `main` is the **PR title** (plus `(#N)`), not the branch's individual commits. semantic-release analyzes that squash line only — title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge should cut a release; `build:` / `ci:` / `docs:` / `refactor:` / `test:` / `style:` / `perf:` alone will verify green and skip a version bump.

## Architecture commitments (do not relitigate without reason)

- **Engine:** Pure Rust, deterministic, **sequential state machine** — the stack/priority model, *not* a game-loop. Runs authoritatively on the server. No I/O, no networking, no wall-clock or randomness that isn't injected.
- **Event-sourced state:** every intent produces events; events mutate board facts. Priority/pass bookkeeping and pending choices are orchestration state in the submit path — preserve intent-replay determinism.
- **Server:** tonic gRPC (game/auth/decks/catalog/seed) + Axum HTTP health only. Live games are in-memory only; Postgres `mtgfr` holds users, sessions, decks. Pre-game lobby + `table_routes` live on the Foldkit SPA's Nitro BFF on Postgres `mtgfr_web` (Drizzle). BFF routes in-game by table id → pod DNS gRPC; seeds hit Service `edh-api` (newest instance only). API/web Deployments are Argo-owned; rolls drain on SIGTERM. Server-side per-player visibility filtering is a hard rule (hands/libraries are private).
- **Client:** Foldkit SPA on Nitro (Vite; single event-reactor `Model`/`update`/`view` in `client/app/`) — hybrid canvas + Mount bitmap board with thin HTML overlays; same-origin Effect RPC (`/api/rpc`) to the BFF, which dials tonic. **Camera transform** (single source of truth for pan/zoom) and **screen→world hit-testing** are foundations everything downstream assumes. Design tokens live in `design.tokens.json` (DTCG); Tailwind `@theme` and canvas TS outputs are generated — see [`DESIGN.md`](DESIGN.md).
- **Client state is Effect-first Foldkit.** Async work — wire calls, streams, polling — stays in Effect services/streams at runtime boundaries; Foldkit `Model`/`update`/`view` owns UI state and dispatches messages. Keep `effect`, every `@effect/*`, `foldkit`, and `@foldkit/ui` pinned to exact versions that move as one set — `@foldkit/ui` peer-requires a specific `effect` beta, so bump the whole set together or not at all (`client/package.json` holds the current pins). Styled components live in `client/app/domain/ui/`. BFF Drizzle is Effect-native via `drizzle-orm/effect-postgres` + `@effect/sql-pg` (no pg-proxy).
- **Observability:** self-hosted LGTM + Faro + OTEL. Exporters no-op locally unless `OTEL_EXPORTER_OTLP_ENDPOINT` / Faro upstream is set; never put hand/library contents or intent payloads in telemetry.
- **Card pool is data-driven scripts.** `cards` defines the vocabulary (the enums); `engine` implements the rules around it. Let the scripting DSL grow from real cards — resist generalizing it prematurely. Author cards by composing that vocabulary; do not add a bespoke effect leaf for a single card when sequence/conditional/filters/amounts already cover it.
- **Wire types:** `.proto` is the sole contract → prost/tonic (`build.rs` → `OUT_DIR`) + Effect-gRPC clients (`just server-codegen` / `bun run gen` → gitignored `client/app/domain/wire/generated/`). Run codegen after proto changes. See [docs/WIRE_COMPAT.md](docs/WIRE_COMPAT.md).
- **Routing:** Required identifiers belong in **path params** (server: Axum `Path`, client: Foldkit route path segments). **Query params are optional** — filters, paging, redirect targets (`?next=`), and preselection (`?deck=`). Never put a required resource id in a query string.
- **Public crawl posture:** `client/public/robots.txt` disallows all crawlers; do not add sitemaps or marketing SEO without revisiting that choice.

**Crate split:** `cards` (the card DSL — `CardDef` / `Effect` / filters / triggers, the TOML surface, and the scripts in `data/`) / `engine` (pure, no I/O; the rules logic over that vocabulary, which it re-exports) / `server` (tonic + health Axum) / `schema` (projection DTOs; mapped to/from native proto at the gRPC edge). **Client split:** `client/app/` (Foldkit UI), `client/app/domain/` (shared wire/domain helpers), `client/server/` (Nitro BFF routes/plugins), `client/styles/` (Tailwind/design tokens).

**Where to start reading:** [`openspec/specs/`](openspec/specs/) for current system requirements (OpenSpec — source of truth) · [`docs/AGENT_NAVIGATION.md`](docs/AGENT_NAVIGATION.md) for the engine module map and CR lookup (`docs/CR_INDEX.md`, regenerate with `just engine-cr-index`) · [`docs/CLIENT_CANVAS_MAP.md`](docs/CLIENT_CANVAS_MAP.md) for the canvas board (paint vs hits vs flights vs DOM overlays).

## Feature specs (OpenSpec)

Living requirements live under [`openspec/specs/<capability>/spec.md`](openspec/specs/). Capabilities are consolidated (engine, card-dsl, wire-protocol, accounts-and-catalog, lobby-and-live-game, client-shell, deck-builder, game-board, production-and-ops) — **not** one file per UI surface. Document what exists today: no TBD, no migration history, no sprint narrative. Cite the relevant capability instead of inventing requirements.

### New behavior workflow

1. **Brainstorm first** — use the Superpowers `brainstorming` skill for any creative / behavior-changing work. Design input may still be written under local `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` (and plans under `docs/superpowers/plans/`). That whole tree is **gitignored** — design input only, not the living contract.
2. **Implement** — Superpowers `writing-plans` / `executing-plans` / `test-driven-development` (and related process skills) as usual. Do not vendor or fork Cursor `superpowers` plugin skill files into `.agents/skills/`.
3. **Document in OpenSpec at the end** — before claiming done, update the matching OpenSpec capability so main specs describe what shipped. Prefer an OpenSpec change (`/opsx-propose` → apply → `/opsx-archive` or `/opsx-sync`) so deltas merge into `openspec/specs/`. Do **not** leave shipped behavior only in a Superpowers `*-design.md`.

OpenSpec CLI context and per-artifact rules: [`openspec/config.yaml`](openspec/config.yaml). Skills: `.cursor/skills/openspec-*`; commands: `.cursor/commands/opsx-*`.

**Review gate:** Every code review (including autonomous continuous-loop reviews via `requesting-code-review`) must check OpenSpec compliance on the diff. Violations — shipped behavior missing from the matching capability, conflicting requirements across capabilities, requirements that describe abandoned behavior, or a new `*-design.md` without a corresponding OpenSpec update when behavior shipped — are **merge-blocking**.

## Coding standards (project-specific — enforce these)

- **Readability and maintainability are the top priority**, above cleverness or brevity.
- **Guard-return-first (early return) style.** Handle error/edge/invalid cases up front and `return` (or `?` / `continue`) immediately.
- **TDD is the default workflow.** Use the `test-driven-development` skill. Red → green → review. The engine is testable via direct API calls with no UI or network.
- **Every bug fix gets a regression test.** When you find a bug, add a test that fails on the broken behavior and passes with the fix — in the same change if you can. Place it at the lowest layer that catches the failure (engine unit test, schema projection test, client mapping test, HTTP integration test). Use `systematic-debugging` when the cause is unclear.
- **Client UI: every surface gets a Scene test.** Shell routes and board overlays must be covered by `data-testid` Scene assertions in `client/app/shell/surfaces.test.ts` and `client/app/board/html/surfaces.test.ts` (plus focused tests). Do not ship a user-visible panel that only has update/logic tests. When adding a surface, add or extend those suites in the same change.
- **Client interaction: assert outcomes, not only presence.** When changing pointer, keyboard, hover, drag, Mount hosts, lobby/host flow, or BFF env defaults, add or extend a unit/Scene test for the user-visible result (pin set, tile hidden, selected deck matches, art URL swapped, default URL works). Do not frame tests as migration/"parity" checks — name the product behavior. See OpenSpec [`client-shell`](openspec/specs/client-shell/spec.md) (Client Interaction Testing).
- **Interaction / UI PRs.** Check the PR template box when the change touches those surfaces. Before claiming done, run the Interaction checklist in `.agents/skills/verify/SKILL.md` (in addition to `verification-before-completion`).
- **Verify before claiming done.** Use `verification-before-completion` (and the project `verify` skill for live games). Live or Scene checks must exercise the surfaces you changed (cold load, route entry), not only unit green.
- **Use Magic terminology and semantics wherever possible.** The ubiquitous language lives in `CONTEXT.md`; keep code and glossary aligned. When rules and simplicity genuinely conflict, name the rule approximated in a `ponytail:` comment.
- **Client Tailwind: prefer `data-*` + named `group` over JS class ternaries for interactive chrome.** When a tile/button has selected / selectable / pressed / hover-linked styles (raise, ring, hit height, brightness), put stable boolean attrs on the interactive root (`data-selected="true"|"false"`, `data-selectable="true"`, …) and a named group (`group/hand-tile`, `group/pile-card`, …). Encode the look with Tailwind variants — `group-hover/…`, `group-data-[selected=true]/…`, `data-[selected=true]:…` — so JS only sets attributes. Keep playable / zone aura helpers when they are not selection state. Assert the data attrs (and the variant class tokens) in Scene/unit tests rather than reconstructing which ring class a ternary would have emitted. Reference: hand-bar pick chrome in `client/app/board/html/hand.ts` and OpenSpec [`game-board`](openspec/specs/game-board/spec.md).

## Agent skills

Project skills live in `.agents/skills/` — `card-dsl`, `fidelity-grind`, and `forge` for card work, `verify` for the live-game and interaction checklists, plus Foldkit and Effect helpers. Root `skills-lock.json` tracks the GitHub-installed ones. `ls .agents/skills/` for the current set.

Workflow skills (`brainstorming`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `requesting-code-review`, `writing-plans`, `executing-plans`, `using-git-worktrees`, and the rest) come from the Cursor **`superpowers`** plugin, enabled in `.cursor/settings.json`. Do not vendor or fork them into `.agents/skills/`.

## Cursor Cloud specific instructions

Cloud Agents build from [`.cursor/Dockerfile`](.cursor/Dockerfile) via [`.cursor/environment.json`](.cursor/environment.json) — read those for the installed toolchain and the `install` / `start` steps. Do **not** use interactive dashboard "Set up agent" / snapshot setup for this repo; that mode ignores the Dockerfile. If a saved Cloud environment snapshot exists for the repo, delete it so Dockerfile builds win.

- Postgres `mtgfr` / `mtgfr_web` are seeded and `DATABASE_URL` / `WEB_DATABASE_URL` are set. Before DB-touching work: `just migrate` (Toasty / `mtgfr`) and/or `just client-migrate` (Drizzle / `mtgfr_web`).
- Prefer `just server-check` / `just client-check` (or `just check`) for verification.
- Put secrets in the Cursor Cloud Agents Secrets UI — do not bake credentials into the image or commit `.env` files.
- Foldkit DevTools MCP uses Vite relay port `9988`; `foldkit_list_runtimes` only sees a runtime while a browser tab has the app open (`devTools: { Message }` in `client/app/entry.ts`).
