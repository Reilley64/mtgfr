Status: complete; optional `proxyArtUrl` now flows through board bitmap paint/preload/flights/exit-fx plus hand, stack, prompts, pile, mulligan, and inspect surfaces.
Commit: `feat(client): thread proxy art through board paint and overlays`
Specs: updated battlefield, hand-and-zone-bar, stack, and card-inspect for the shared proxy-art contract.
Tests: `bun test app/board/bitmap/paint-cards.test.ts app/board/bitmap/mount.test.ts app/board/html/hand.test.ts app/board/html/stack.test.ts app/board/html/surfaces.test.ts app/board/html/chrome.test.ts app/board/html/prompts.test.ts app/board/inspect-pile-concede.test.ts` → 255 pass.
Tests: `just client-typecheck` → pass.
Tests: `just client-migrate` then `just client-check` → 118 files / 1216 tests pass.
Concerns: no known product concerns; local `client-check` needed Drizzle migrations because the `lobbies` table was absent in this VM before `just client-migrate`.
Report path: `/workspace/.superpowers/sdd/task-5-report.md`
