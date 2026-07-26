# Task 9 report — Witherbloom client catch-up, smoke, sync

## Status: COMPLETE WITH ONE EXTERNAL-TOOL BLOCKER

The Witherbloom client catch-up landed: `may_return_from_graveyard` now carries `mandatory`
through DTO → proto → gRPC map → browser wire, optional returns show an explicit decline, and
mandatory returns hide decline while keeping `Return` disabled until exactly one graveyard card is
picked.

## Commits

- `d92152c` `fix(client): gate mandatory graveyard return prompts`
- `9a636d2` merge `origin/main` into `cursor/soc-fidelity-program-8537`

## What landed

- Projected `mandatory: bool` on `PendingChoiceView::MayReturnFromGraveyard` in
  `crates/schema/src/dto.rs`, `crates/schema/src/projection/choice.rs`,
  `proto/mtgfr/v1/stream.proto`, `crates/server/src/grpc/map/stream.rs`, and
  `client/lib/wire/types.ts`.
- Added browser-wire regression coverage in `client/lib/wire/protoMap.test.ts`.
- Tightened client prompt semantics in `client/lib/choice.ts`:
  optional graveyard returns decline via explicit empty `choose_sacrifices`; both optional and
  mandatory `Return` submits now require a picked card; mandatory returns cannot decline.
- Added client regression tests in `client/lib/choice.test.ts`,
  `client/app/board/html/prompts.test.ts`, and `client/app/board/html/surfaces.test.ts`.
- Updated living prompt spec in
  `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md`.

## Verification

- Focused TDD red→green:
  `bun test client/lib/choice.test.ts client/lib/wire/protoMap.test.ts client/app/board/html/prompts.test.ts client/app/board/html/surfaces.test.ts`
  → **197 passed**.
- Full client verification:
  `just client-check` → **green** (`vitest: 1095 passed`).
- Precon legality:
  `cargo nextest run --profile ci -p server -- witherbloom_pestilence_is_a_legal_commander_deck`
  → **1 passed**.

## Live smoke (`-3`)

- Started local stack with `cargo run -p server -- serve` and `cd client && bun run dev`
  (Vite on `localhost:3000`, server health on `localhost:8080`).
- 4-seat BFF smoke table `EC3HQV` reached real gameplay after mulligans and exercised live
  `play_land`, `cast`, `activate`, `cycle`, `declare_attackers`, `declare_blockers`, plus pending
  `choose_target` and `discard`.
- The terminal driver did **not** reach game over; it eventually hit repeated
  `reject.not_your_priority` sync errors after 104 intent steps. This is honest partial coverage,
  not a claimed full-clear.

## PR / sync notes

- PR body update for `#224` is blocked here: the requested `ManagePullRequest` tool is unavailable
  in this harness, and `gh` is read-only.
- Sync merge completed cleanly against `origin/main`; post-merge `just client-check` stayed green and
  `witherbloom_pestilence_is_a_legal_commander_deck` still passed.
- Pre-existing unstaged branch worktree changes outside this task were left untouched.

## Report path

`/workspace/.superpowers/sdd/task-9-report.md`
