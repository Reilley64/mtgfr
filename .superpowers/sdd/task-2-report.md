# Task 2 Report: Grabbing cursor during hand drag

## Status

**Complete**

## Changes

### `client/app/board/html/hand-drag-mount.ts`

- Added exported `setHandDragGrabbingCursor(active: boolean)` helper that sets `document.documentElement.style.cursor` to `"grabbing"` or `""`, with a `typeof document === "undefined"` guard.
- Wired `setHandDragGrabbingCursor(true)` immediately after a successful `HandDragStarted` offer in `onPointerDown` (only when payload is non-null).
- Wired `setHandDragGrabbingCursor(false)` inside `teardown` (pointerup, pointercancel, and re-start paths).
- Wired `setHandDragGrabbingCursor(false)` in the acquireRelease cleanup after `handle.teardown()`.

### `client/app/board/html/hand-drag-mount.test.ts`

- Extended existing Task 1 zone tests with a focused unit test for `setHandDragGrabbingCursor` (sets `"grabbing"` when active, clears to `""` when inactive).

## TDD

1. **RED:** Added failing test — `setHandDragGrabbingCursor is not a function`.
2. **GREEN:** Implemented helper + mount wiring; all tests pass.

## Verification

```
cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/hand-drag.test.ts
```

Result: **2 files, 12 tests passed**

## Commit

```
feat(client): use grabbing cursor during hand drag
```

## Concerns

- None. Cursor is not set when payload is null after pointerdown. Teardown and mount unmount both clear the cursor (teardown runs first in cleanup, so the explicit post-teardown clear is redundant but harmless per plan).

## Review follow-up

- Extracted `armHandDragGrabbingCursor()` so the mount arms the global grabbing cursor only after a successful drag start and tears it down through a captured disposer.
- Updated `MountHandBarDrag` teardown to call the session disposer before resetting it to the default no-op clear path.
- Extended the focused helper tests to cover arming, disposer clearing, and double-dispose safety while keeping the existing `setHandDragGrabbingCursor` coverage intact.
- Verification for this follow-up: `cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/hand-drag.test.ts` → `2 files, 15 tests passed`.
- Follow-up commit: `test(client): gate hand drag grabbing cursor arm/disarm`.
