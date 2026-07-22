# Foldkit cutover — merge cleanup

**Date:** 2026-07-22  
**Status:** Approved for planning  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74) (`cursor/foldkit-migration-design-1ef0`)  
**Context:** SolidStart/Vinxi is already removed; Foldkit + Nitro is the live client. CI is green. This work is **merge readiness only** — present-tense docs, light code hygiene, small behavior/build nits, and PR metadata for squash-merge + semantic-release.

## Decisions (locked)

| Question | Choice |
|---|---|
| Scope | **Full merge polish** (docs + comments + behavior + Nitro test exclusion + Biome warnings + PR title/body) |
| Packaging | **One commit** on the existing PR branch |
| Docs posture | Foldkit is **present-tense truth**; migration design/plan stay as **archive**, not deleted |
| Behavior fixes | Only known small nits (`start_error: AlreadyStarted` on successful start; Biome non-nulls; Nitro not shipping tests) |
| Out of scope | Features, board parity work, engine/server/proto, upstream Foldkit Image/sprites, history rewrite beyond this one commit |

## Goals

1. Humans and agents reading the repo after merge see **Foldkit + Nitro**, not SolidStart/`client/src`.
2. No migration-era wording in active code comments that implies SolidStart is still present.
3. Lobby start success does not surface as a start error.
4. Client lint is **warning-clean**; production Nitro output contains **no** `*.test.*` artifacts.
5. Squash-merge line is release-correct: `feat:` subject + `BREAKING CHANGE:` in the PR body (no `feat!:`).

## Non-goals

- Second architecture narrative or re-running the migration plan.
- Broader lobby/board UX polish.
- Changing commitlint config or CI jobs.
- Deleting the migration design/plan files.

## Docs

Update only surfaces that still describe SolidStart/Vinxi/`client/src` as current.

| Doc | Change |
|---|---|
| `README.md` | Wire / BFF / client rows → Foldkit SPA + Nitro BFF + Effect RPC; `just dev` → Foldkit/Vite (not Vinxi). |
| `DESIGN.md` | Token path → `client/styles/global.css`; canvas paint pointers → `client/app/board/**` (not `Board.tsx`). |
| `docs/superpowers/specs/2026-07-20-client-shell-deck-builder-and-observability.md` | **Module** line and architecture blurb → `client/app/shell/**`, `client/lib/**`, `client/server/**`; test inventory paths; keep **Further Notes** → migration design. |
| `docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md` | Same for `client/app/board/**` (canvas / bitmap / html / geometry); keep Further Notes. |
| `docs/superpowers/specs/2026-07-21-foldkit-client-migration-design.md` | Status → **Done (cutover on branch; pending merge)**; one-line note that live architecture is AGENTS.md + shell/board specs; this file is historical. |
| `docs/superpowers/plans/2026-07-21-foldkit-client-migration.md` | Top banner: **historical plan — executed**; no task-by-task rewrite. |
| `docs/agent-navigation.md`, `docs/client-canvas-map.md`, production/lobby/wire specs | Touch **only** if they still claim SolidStart/`client/src` as current. |

**Rule:** Prefer path/stack updates over retelling the migration. Do not delete archive docs.

## Code comments

Rewrite or drop file headers that name SolidStart/Vinxi/`client/src` as if still present (e.g. Faro, `rpcServer`, `api/[...path]`, “ported from Solid” one-liners). Prefer neutral present-tense (“BFF session cookie…”) over history.

## Lobby `start_error` after start

**Bug:** `toLobbyView` always sets `start_error` from `startError(snap, userId)`. When `snap.startedAt` is set, that returns `AlreadyStarted`, so a successful start (and every later poll) yields `started: true` **and** `start_error: "AlreadyStarted"`. The lobby UI shows “The game already started.” and disables Start for the wrong reason.

**Fix:** When `snap.startedAt != null`, `toLobbyView` sets `start_error: null`. `started: true` is the sole “already started” signal. Keep `startError()` for **pre-start** gates (host, seated, ready, player count).

**POST `/api/tables/start/v1`:** If the lobby is already started, return the normal started view (`started: true`, `start_error: null`) — idempotent success, not an error path.

**Test:** Unit coverage that a started lobby snapshot projects to `started: true` and `start_error: null` (extend existing lobby-store / lobby entry tests).

## Biome `noNonNullAssertion`

Clear the existing warnings in:

- `client/app/board/geometry/layout.ts`
- `client/app/board/geometry/layout.test.ts`
- `client/app/board/geometry/density.test.ts`

Use small guards / `expect(…).toBeDefined()` patterns. Do **not** disable the rule. Goal: `bun run lint` reports **0 warnings**.

## Nitro must not ship tests

Production `bun run build` currently emits `method_gate.test.mjs` under `client/.output/server/`.

**Fix:** Exclude `**/*.test.ts` (and equivalent) from the Nitro server scan via `client/nitro.config.ts` and/or server layout so Vitest owns tests only.

**Verify:** After `bun run build`, no path under `client/.output` matches `*.test.*`.

## PR metadata (squash + semantic-release)

Per AGENTS.md, the squash commit message is the **PR title**. semantic-release reads that line only.

| Field | Value |
|---|---|
| Title | `feat: replace solidstart client with foldkit and nitro` (no `!`) |
| Body | Short summary + test plan, and footer: `BREAKING CHANGE: client is Foldkit SPA on Nitro; SolidStart/Vinxi removed.` |

One commit on the branch for this entire cleanup (approach: single merge-readiness commit).

## Verification

Before claiming done:

1. `just client-check` (format, lint warning-clean, typecheck, tests)
2. `bun run build` in `client/` — no `*.test.*` under `.output`
3. Commitlint accepts the cleanup commit
4. PR title/body updated as above

## Packaging

Single commit on `cursor/foldkit-migration-design-1ef0`, e.g.:

```text
docs: foldkit merge cleanup for cutover PR

Present-tense docs, lobby start_error fix, biome non-nulls,
Nitro test exclusion, and PR-ready breaking-change metadata.
```

(Exact subject may adjust to stay ≤72 chars and lower-case subject per commitlint.)
