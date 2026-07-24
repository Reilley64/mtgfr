# Task 3 Report: Spec + disabled Mulligan + verification

## Status

**DONE**

## Summary

- Added missing Scene coverage for the disabled `mulligan-take` control in `client/app/board/html/chrome.test.ts`.
- Updated `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` to describe the shipped mulligan overlay, post-keep waiting banner, overlay/waiting test coverage, and the `2026-07-24-mulligan-pregame-overlay-design.md` cross-link.
- Kept the existing production behavior intact: `client/app/board/html/mulligan-overlay.ts` already bound `h.Disabled(!chrome.canMulligan)`, and focused verification confirmed that wiring.

## TDD Evidence

### RED

Added this Scene:

```ts
test("mulligan take is disabled when can_mulligan is false", () => {
  // undecided, can_mulligan: false, mulligans_taken: 6
  Scene.expect(Scene.testId("mulligan-overlay")).toExist();
  Scene.expect(Scene.testId("mulligan-take")).toBeDisabled();
});
```

Ran:

```bash
cd /workspace/client
bunx vitest run app/board/html/chrome.test.ts
```

Result: **FAIL**
- Exit code: `1`
- Failure cause: the new undecided-mulligan Scene used `resolveBoardOverlayMounts()`, which expected a hand-bar drag mount that is intentionally absent while the overlay hard-lock is active.
- This was a test harness/setup failure, not a product-behavior failure.

### GREEN

Adjusted the new Scene to use the same undecided-overlay mount resolution as the existing mulligan overlay test:

- `Scene.Mount.resolveAll([MountPriorityWatch(), PriorityElapsed({ seconds: 0 })], [BindCardArt, ArtLoaded()])`

Re-ran:

```bash
cd /workspace/client
bunx vitest run app/board/html/chrome.test.ts
```

Result: **PASS**
- Exit code: `0`
- `1` file passed
- `11` tests passed
- The new assertion proved the existing `h.Disabled(!chrome.canMulligan)` binding works.

## Focused Verification

Ran from `client/`:

```bash
bunx vitest run app/board/html/chrome.test.ts app/board/inspect-pile-concede.test.ts
bunx vitest run lib/mulligan.test.ts
bunx tsc --noEmit -p tsconfig.json
bunx biome check --write app/board/html/mulligan-overlay.ts app/board/html/overlays.ts app/board/html/concede.ts app/board/html/chrome.test.ts
```

Result: **PASS**
- Exit code: `0`
- `app/board/html/chrome.test.ts` + `app/board/inspect-pile-concede.test.ts`: `40` tests passed
- `lib/mulligan.test.ts`: `7` tests passed
- TypeScript typecheck passed
- Biome checked `4` files and rewrote `1` file (`client/app/board/html/mulligan-overlay.ts`) for formatting only

## Post-format Re-check

Re-ran after Biome formatting:

```bash
cd /workspace/client
bunx vitest run app/board/html/chrome.test.ts app/board/inspect-pile-concede.test.ts
bunx vitest run lib/mulligan.test.ts
bunx tsc --noEmit -p tsconfig.json
```

Result: **PASS**
- Exit code: `0`
- `47` focused tests passed total
- TypeScript typecheck passed

## Files Changed

- `client/app/board/html/chrome.test.ts`
- `client/app/board/html/mulligan-overlay.ts` (Biome formatting only)
- `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md`

## Notes

- No engine, wire, or London mulligan behavior changed in this task.
- The spec now matches the shipped overlay/waiting flow from Tasks 1–2.
