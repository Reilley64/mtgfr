# OpenSpec ← Superpowers migration summary

Migration completed: Superpowers timestamped living specs → consolidated OpenSpec capabilities under `openspec/specs/`.

## Ongoing workflow (post-migration)

1. **Superpowers brainstorming** for new behavior (optional local gitignored `docs/superpowers/specs/*-design.md`).
2. **Implement** via Superpowers plans / TDD.
3. **Document in OpenSpec** before done (`/opsx-propose` → apply → archive/sync into `openspec/specs/`).

Design docs are local scratch; OpenSpec is the living contract. The former `docs/superpowers/` tree was removed from git after migration.

## Documents created

| Path | Requirements | Source consolidation |
|------|--------------|----------------------|
| `openspec/specs/engine/spec.md` | 25 | engine-core, turn-priority-and-stack, choices-actions-and-resolution, combat-and-commander-rules |
| `openspec/specs/card-dsl/spec.md` | 12 | card-dsl-and-card-pool (+ shipped schema/fidelity bits) |
| `openspec/specs/wire-protocol/spec.md` | 14 | wire-protocol-and-visibility (+ WIRE_COMPAT / buf breaking / mana_only) |
| `openspec/specs/accounts-and-catalog/spec.md` | 9 | accounts-decks-and-catalog |
| `openspec/specs/lobby-and-live-game/spec.md` | 13 | lobby-table-routing-and-live-game (+ shipped BFF route split / Effect WebDb) |
| `openspec/specs/client-shell/spec.md` | 11 | shell-routes-and-auth, coverage-by-set, ui-component-layer, foldkit-devtools, interaction-test-policy |
| `openspec/specs/deck-builder/spec.md` | 5 | deck-list-and-builder |
| `openspec/specs/game-board/spec.md` | 16 | ~15 board/lobby-entry living surfaces |
| `openspec/specs/production-and-ops/spec.md` | 13 | production-topology, observability-ops, ci-and-release (+ CDN / OTEL / CI image / sharding as shipped) |
| `openspec/specs/README.md` | — | capability index |
| `openspec/config.yaml` | — | project context + artifact rules |

## Documents updated (repointed to OpenSpec)

- `AGENTS.md` — Feature specs section now OpenSpec-first
- `README.md`, `docs/README.md`, `docs/AGENT_NAVIGATION.md`, `docs/WIRE_COMPAT.md`
- Removed `docs/superpowers/` from git (tree remains gitignored for local brainstorming scratch)
- `.agents/skills/card-dsl/SKILL.md`, `.agents/skills/foldkit/SKILL.md`
- `.github/PULL_REQUEST_TEMPLATE.md` — OpenSpec checklist item

## Duplicate specs consolidated

Fine-grained Superpowers surface specs were **not** 1:1 migrated. Notable merges:

- Engine rules (4 living specs) → `engine`
- CI + topology + observability (3) → `production-and-ops`
- Shell + coverage + UI components + DevTools + test policy → `client-shell`
- All board chrome/paint/interaction living specs → `game-board`
- Lobby server routing kept separate from lobby entry UI (`lobby-and-live-game` vs `game-board`)

## Outdated requirements removed / corrected from code

- Stale “~780 cards / nine precons” → current pool size left qualitative; **ten** precon fixtures `-1`..`-10`
- Nextest-sharding design’s “Postgres on every shard” → shipped CI isolates Postgres to migrate job only
- Commander damage keyed by `(ObjectId, …)` in old combat prose → implementation uses `(PlayerId owner, amount)`
- Poison SBA (10 counters) present in code but missing from old engine living specs → absorbed
- `mana_only` legal-action flag absorbed (was design-doc-first)
- Art URL “no Scryfall host” conflict between accounts draft and `buildImageUrl` → art rules owned solely by `deck-builder` + CDN Worker in `production-and-ops`

## Design-only Superpowers docs not promoted as requirements

Unfinished or process-only `*-design.md` programs remain historical (examples): engine-refactor leftover waves, soc-fidelity mega-PR process, session-resilience UX copy beyond `UnknownTable`, card-image-cdn `.jpg` cutover (code remains `.webp`), effect-deepening optional host swaps, elo-leaderboard design extras beyond shipped page, tablet/ergonomics already absorbed where shipped.

## Inconsistencies requiring manual review

1. **Mulligan stream events:** schema/engine still have mulligan event kinds; `stream.proto` omits them; clients use snapshot fields — OpenSpec documents snapshot-sourced UI as current; deciding whether to add proto arms is a product choice.
2. **Prompt kind matrix:** `game-board` collapses dozens of pending-choice presentations — expand OpenSpec scenarios if a kind regresses often.
3. **Elo formula / K-factor:** unlock-tail persist is specified; exact rating math in `crates/server/src/elo.rs` is lightly documented.
4. **Modifier/Permanent field collapse:** continuous-effect registry is specified; many ad-hoc `Permanent`/`Game` fields remain in code alongside it.
5. **Land-subtype filter blind spot** (Clifftop Retreat–style): known DSL/engine limitation; not elevated to a positive requirement.
6. **Worktrees** under `.claude/worktrees/` still carry old Superpowers README wording — ignored for this migration (not mainline).

## Possibly undocumented in OpenSpec (code present)

- Per-RPC cookie set/clear matrix on `/api/rpc` beyond session semantics
- Full enumeration of ~130 `VisibleEvent` oneof arms (intentionally omitted; coverage gated by client registry check)
- Canvas layer-stack detail → still `docs/CLIENT_CANVAS_MAP.md` (companion, not OpenSpec)
- `card-dsl` / `card-schema` Cargo feature split

## OpenSpec requirements not fully reflected in implementation (known gaps, intentional)

- Full CR 613/614 completeness — explicitly out of scope beyond registry coverage
- Partner commanders — out of scope
- Planeswalker-as-attack-target — partial
- Live game persistence / replay from event log — out of scope by architecture
- Offline PWA / app-shell caching — network-only SW by design

## Validation

`openspec validate --specs --strict` — all 9 capabilities pass (118 requirements total).
