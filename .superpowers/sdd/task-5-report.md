Status: complete; optional `proxyArtUrl` now flows through board bitmap paint/preload/flights/exit-fx plus hand, stack, prompts, pile, mulligan, and inspect surfaces.
Commit: `feat(client): thread proxy art through board paint and overlays`
Specs: updated battlefield, hand-and-zone-bar, stack, and card-inspect for the shared proxy-art contract.
Tests: `bun test app/board/bitmap/paint-cards.test.ts app/board/bitmap/mount.test.ts app/board/html/hand.test.ts app/board/html/stack.test.ts app/board/html/surfaces.test.ts app/board/html/chrome.test.ts app/board/html/prompts.test.ts app/board/inspect-pile-concede.test.ts` → 255 pass.
Tests: `just client-typecheck` → pass.
Tests: `just client-migrate` then `just client-check` → 118 files / 1216 tests pass.
Concerns: no known product concerns; local `client-check` needed Drizzle migrations because the `lobbies` table was absent in this VM before `just client-migrate`.
Report path: `/workspace/.superpowers/sdd/task-5-report.md`

## Review fix: stale proxy art on flight retarget

### Red

Command:

```bash
bun run test -- app/board/exit-fx-sync.test.ts
```

Observed failure before the fix:

```text
FAIL  app/board/exit-fx-sync.test.ts > syncBoardWithGame exit FX > retargeting a flight to a non-proxy card clears stale proxy art
AssertionError: expected 'https://example.com/proxy-bear.png' to be undefined
```

### Green

Same command after the fix:

```bash
bun run test -- app/board/exit-fx-sync.test.ts
```

Observed pass after the fix:

```text
Test Files  1 passed (1)
Tests  5 passed (5)
```

### Living spec updates

- `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md`
- `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md`
- `docs/superpowers/specs/2026-07-20-system-overlays.md`
- `docs/superpowers/specs/2026-07-20-flights.md`

### Focused verification

```bash
bun run test -- app/board/bitmap/paint-cards.test.ts app/board/bitmap/mount.test.ts app/board/bitmap/paint-flights.test.ts app/board/bitmap/paint-exit-fx.test.ts app/board/bitmap/flight-frame.test.ts app/board/motion/exit-fx.test.ts app/board/exit-fx-sync.test.ts app/board/story.test.ts app/board/html/chrome.test.ts app/board/html/prompts.test.ts app/board/inspect-pile-concede.test.ts
just client-typecheck
```

Observed results:

- 11 test files passed
- 175 tests passed
- `tsc --noEmit` passed via `just client-typecheck`
