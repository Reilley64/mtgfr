# AGENTS.md

Project instructions for AI coding agents. Direct user instructions win; otherwise follow this file, then the linked source documents and applicable skills.

## Mission and task routing

Build a browser-based four-player Commander game whose north star is **any card, built faithfully**. This is a direction, not a completeness claim: the engine and `crates/cards/data/` remain partial.

- Grow the engine and DSL from real cards, TDD, smallest increment first.
- **Flag, do not force:** record missing expressiveness in `docs/fidelity/<slug>-increments.md`; never contort a card around the current DSL. Take decks faithful one at a time; `docs/fidelity/` records what was ground and its status. Each deck proves the general system, not its terminal scope.
- Prefer compositions of existing effects (`sequence`, filters, modes, amounts) over card-specific leaves.
- Card work: use `.agents/skills/card-dsl/`; deck intake-to-PR: `.agents/skills/fidelity-grind/`; tricky rules: use `forge` against `.repos/forge` (`just forge` refreshes it).
- Read requirements first in `openspec/specs/`. Use `docs/AGENT_NAVIGATION.md` for engine/CR navigation and `docs/CLIENT_CANVAS_MAP.md` for board paint, hits, flights, and overlays.
- Use `ast-grep` (not `sg`) for AST-shaped Rust/TypeScript searches; use `rg` for text, CR cites, and comments.

## Prime Agent delegation and token budget

**Default to inline work.** A child has its own full prompt/context cost; delegate only independent, context-heavy work whose saved parent context exceeds coordination cost. Do not delegate a single lookup, small edit, sequential dependency, or work that needs the parent's evolving context.

### Model and effort routing

The only optional `rlm()` spawn arguments are `name` and `model`; a child otherwise inherits the parent model. Before overriding it, discover active authenticated models and use an **exact returned selector**:

```python
models = await rlm.find_models("fast economical coding", limit=8)
[(m.selector, m.name) for m in models]  # inspect; search order is not price order
# Then spawn with model="<exact inspected provider/model selector>".
```

Never invent or blindly take the first selector. If discovery is empty, inherit the parent rather than repeatedly searching. Requested models fail closed when unavailable; do not silently substitute another model.

**Effort is not configurable per child:** `effort=` is unsupported and will fail. Children inherit the parent's current `/effort` setting. `/effort` is a user/session control, so recommend changing it only before a homogeneous batch; do not claim mixed-effort concurrent children.

| Work | Model policy | Recommended parent `/effort` before a homogeneous batch |
|---|---|---|
| File inventory, bounded search, mechanical checks, isolated test execution | Choose an inspected economy/fast model from discovery | `low` |
| Focused implementation, ordinary debugging, code review | Inherit parent or choose an inspected proven coding model | `medium` |
| Architecture, ambiguous root cause, Magic rules semantics, security/privacy, final adversarial review | Inherit the strongest appropriate parent or use an inspected reasoning model | `high`; `max` only when justified |

Prefer a cheaper capable model over excessive reasoning for bounded tasks. Spawn with a self-contained prompt, stable name, and selected model, for example:

```python
child = await rlm(
    "Bounded task with paths, constraints, deliverable, verification, and: "
    "reply concisely via agent_message.send(..., receiver_role='parent').",
    name="stable-purpose-name",
    model="<exact inspected provider/model selector>",
)
```

### Delegation discipline

- Give each child one non-overlapping deliverable, exact paths, constraints, verification, and output format. Ask for a concise answer or file artifact, not a narrative transcript.
- Parallelize only truly independent work. Stop when fan-out/merge cost exceeds expected savings.
- Children return answers only through `agent_message` or files; `rlm()` returns admission metadata, never the answer. End the turn instead of polling.
- Use stable unique names. Recover children with `rlm.list_subagents()`, follow up through `agent_message`, and delete completed/unneeded children with `rlm.delete_subagent()`.
- Use subagents for context isolation, not as a substitute for reading their evidence or verifying their result.

## Engineering workflow

1. **Creative or behavior-changing work:** run the Superpowers `brainstorming` workflow first, then planning/execution/TDD skills. Local design and plan files under `docs/superpowers/` are gitignored input, not the living contract.
2. **Implement with TDD:** red → green → review. Every bug fix needs a regression test at the lowest layer that catches it. Use `systematic-debugging` when the cause is unclear.
3. **Document shipped behavior:** update the matching consolidated OpenSpec capability at the end, preferably through an OpenSpec change workflow. Capabilities are `engine`, `card-dsl`, `wire-protocol`, `accounts-and-catalog`, `lobby-and-live-game`, `client-shell`, `deck-builder`, `game-board`, and `production-and-ops`—not one spec per UI surface. Cite the relevant capability rather than inventing requirements. Specs describe current behavior: no TBDs, migration history, or sprint narrative.
4. **Review gate:** every review must check OpenSpec compliance. Missing/conflicting/stale requirements, or shipped behavior documented only in a `*-design.md`, block merge.
5. **Verify before completion:** use `verification-before-completion`; use `.agents/skills/verify/` for live-game/UI work. Exercise changed surfaces through cold load and route entry, not only unit tests.

Project skills live in `.agents/skills/`; discover them there. Superpowers workflow skills come from the Cursor plugin configured in `.cursor/settings.json`; do not vendor them into `.agents/skills/`.

## Coding and test rules

- Optimize for readability and maintainability; use guard-return-first style (`return`, `?`, or `continue`).
- Use Magic terminology from `CONTEXT.md`. Mark genuine rules simplifications with a `ponytail:` comment naming the approximation.
- Every user-visible client surface needs `data-testid` Scene coverage in the corresponding suite—shell routes in `client/app/shell/surfaces.test.ts`, board overlays in `client/app/board/html/surfaces.test.ts`—plus focused tests, in the same change.
- Interaction changes (pointer, keyboard, hover, drag, Mount hosts, lobby/host flow, BFF defaults) must assert user-visible outcomes, not presence or “parity.” Check the UI PR box and run `.agents/skills/verify/SKILL.md`'s Interaction checklist.
- For interactive Tailwind chrome, expose stable boolean `data-*` attributes and named groups; style with `data-`/`group-data-` variants instead of JS class ternaries. Keep non-selection playable/zone aura helpers. Assert attributes and variant tokens in tests. See `client/app/board/html/hand.ts` and the `game-board` spec.

## Architecture invariants

Do not relitigate these without evidence:

- **Engine:** pure deterministic Rust sequential state machine implementing stack/priority—not a game loop. No I/O, networking, wall clock, or uninjected randomness.
- **State:** intents produce events; events mutate board facts. Priority/pass and pending choices are submit-path orchestration; preserve replay determinism.
- **Boundaries:** `cards` owns DSL vocabulary/data; `engine` implements/re-exports rules; `server` owns tonic game/auth/decks/catalog/seed gRPC plus Axum health; `schema` owns projection DTOs and proto-edge mapping. Client code splits into `client/app/` UI, `client/app/domain/` shared wire/domain helpers, `client/server/` BFF, and `client/styles/` tokens.
- **Server/data:** server-authoritative games are in memory. Postgres `mtgfr` stores users/sessions/decks; Nitro BFF and `table_routes` use `mtgfr_web`. BFF routes table ID to pod DNS gRPC; seeds use newest `edh-api`. Argo owns deployments; SIGTERM drains rolls.
- **Privacy/telemetry:** server-side per-player visibility filtering is mandatory. Never emit hands, libraries, or intent payloads to LGTM/Faro/OTEL. Exporters no-op locally unless their upstream environment is configured.
- **Client:** Foldkit SPA on Nitro/Vite; one `Model`/`update`/`view` reactor; Effect owns async boundaries. Same-origin Effect RPC `/api/rpc` reaches the Nitro BFF, which dials tonic. Board is canvas/Mount with thin HTML overlays. Camera transform is the pan/zoom source of truth; hit testing converts screen→world.
- **Client packages:** pin exact versions of `effect`, all `@effect/*`, `foldkit`, and `@foldkit/ui`; move the set together. Styled components live in `client/app/domain/ui/`; BFF Drizzle uses `drizzle-orm/effect-postgres` + `@effect/sql-pg`, never pg-proxy.
- **Wire:** `.proto` is the sole contract. Regenerate both server and gitignored Effect-gRPC clients after changes; see `docs/WIRE_COMPAT.md`.
- **Routing:** required IDs are path params; query params are optional filters, paging, redirects, or preselection.
- **Design/crawl:** `design.tokens.json` is DTCG source for generated Tailwind/canvas tokens. `robots.txt` disallows all crawlers; no sitemap/marketing SEO without revisiting that posture.

## Commands

```text
just check                     format + lint + typecheck + tests, both sides
just server-check              server-only check
just client-check              client-only check
just test | just lint          both-side focused gates
just openspec-check            strict living-spec validation
just proto-check               buf STANDARD + wire break check vs origin/main
just engine-cr-index[-check]   regenerate/check CR citations
just migrate                   Toasty migrations for mtgfr
just client-migrate            Drizzle migrations for mtgfr_web
just forge                     refresh vendored Forge scripts; commit diff
just dev                       tmux bacon server + Foldkit/Vite client
cargo nextest run --profile ci [filter] [--nocapture]
cargo clippy --all-targets -- -D warnings
cargo fmt
just --group server --list | just --group client --list
```

Run generated code after proto changes with `just server-codegen` / `bun run gen`. Format before committing.

## Commits and releases

- Use Angular conventional commits (`feat:`, `fix:`, `docs:`, etc.; breaking changes use a `BREAKING CHANGE:` footer). Commitlint enforces Husky `commit-msg`.
- PRs squash-merge; PR CI lint-checks the **PR title only**, not branch commits, and that title becomes the analyzed `main` subject. Use `feat:`/`fix:` (or breaking footer) for a release; `build:`/`ci:`/`docs:`/`refactor:`/`test:`/`style:`/`perf:` alone skip versioning.
- Semantic-release alone writes `v*` tags/releases—never create or push them manually. Cascading `docker.yml` requires `RELEASE_TOKEN` with `contents` + `workflow`; default `GITHUB_TOKEN` cannot trigger it. See `production-and-ops`.
- Cursor Cloud hook setup requires root `npm clean-install` plus `.cursor/scripts/wire-cloud-git-hooks.sh`.

## Cursor Cloud

- Build through `.cursor/Dockerfile` + `.cursor/environment.json`; read them for `install`/`start` steps. Never use dashboard snapshot setup. Delete saved snapshots so the Dockerfile wins.
- Databases and URLs are seeded. Before DB work run `just migrate` and/or `just client-migrate`.
- Keep secrets in the Cloud Agents Secrets UI; never commit `.env` or bake credentials into images.
- Foldkit DevTools uses Vite relay `9988`; `foldkit_list_runtimes` needs an open app tab with `devTools: { Message }`.
