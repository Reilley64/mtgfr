# Task 1 report: Playable border follows ghost + idle cursors

## Scope

Implemented Task 1 on branch `cursor/hand-drag-chrome-b23c` from base `ee3094e8`.

Included:

- `HandDragStarted.zone` message payload support
- `HandDragState.zone` submodel storage
- hand-bar drag payload zone inference from `data-bar-zone` / action section
- playable border moved off the drag source and onto the hand-drag ghost
- idle hit-strip cursors changed to `cursor-grab` for playable and `cursor-not-allowed` for unplayable

Explicitly not included:

- Task 2 grabbing cursor behavior
- Task 3 shadow work

## Files changed

- `client/app/board/messages.ts`
- `client/app/board/submodel.ts`
- `client/app/board/html/hand-drag-mount.ts`
- `client/app/board/html/hand.ts`
- `client/app/board/html/hand.test.ts`

## TDD evidence

### Red

Ran:

```bash
cd client && bunx vitest run app/board/html/hand.test.ts -t "drag chrome|not-allowed"
```

Observed failures:

- drag source still contained `ring-playable-border`
- unplayable hit strip still rendered `cursor-default`

### Green

Ran:

```bash
cd client && bunx vitest run app/board/html/hand.test.ts app/board/hand-drag.test.ts app/board/html/surfaces.test.ts
```

Result: 3 files passed, 100 tests passed.

## Notes

- The drag source now keeps only the fade while the ghost owns the playable border.
- The ghost aura follows the bar zone carried on the drag payload, defaulting to `"hand"` when no bar zone is present.
- No Task 2 cursor-grabbing or Task 3 shadow changes were made.

## Concerns

None.
# Task 1 Report: Slot hover z (TDD)

**Branch:** `cursor/hand-hover-stack-b23c`  
**Base:** `ef43e2c8`  
**Commit:** `13330f81` — `feat(client): bring hovered hand bar tile to front`  
**Status:** DONE_WITH_CONCERNS

## Summary

Implemented hand-bar hover bring-to-front by moving z-elevation from the face wrapper (`group-hover/hand-tile:z-30`) to the tile root (`hover:z-50`). Added `data-testid="hand-tile-{id}"` on the tile root for test targeting. Discard-selected tiles still raise via transform only; no selection-time z bump.

## TDD cycle

### Step 1 — RED (failing tests added)

Added `describe("handView hover stacking", …)` with two tests per the task brief in `client/app/board/html/hand.test.ts`.

```bash
cd client && bunx vitest run app/board/html/hand.test.ts -t "hover stacking"
```

**Result:** 2 failed (expected) — `hand-tile-42` missing, no `hover:z-50` on root.

### Step 2 — GREEN (implementation)

Changes in `client/app/board/html/hand.ts` `tile()`:

1. **Tile root class** — appended `hover:z-50`:
   ```ts
   h.Class(
     "group/hand-tile pointer-events-none relative shrink-0 origin-bottom overflow-visible hover:z-50",
   ),
   ```

2. **Tile root test id** — when `objectId != null`:
   ```ts
   ...(objectId != null ? [h.DataAttribute("testid", `hand-tile-${objectId}`)] : []),
   ```

3. **Face class** — raise only, removed face z-elevate:
   ```ts
   const faceClass = [
     "pointer-events-none absolute top-0 right-0 transition-transform duration-[120ms] ease-state",
     discardSelected
       ? "[transform:translateY(var(--raise-y))]"
       : "group-hover/hand-tile:[transform:translateY(var(--raise-y))]",
   ].join(" ");
   ```

Resting inline `"z-index": String(index + 1)` unchanged. No elevated z for `discardSelected`.

### Step 3 — verify PASS

```bash
cd client && bunx vitest run app/board/html/hand.test.ts
```

**Result:** 10 passed (10)

## Test correction (concern)

The brief’s second test used:

```ts
expect(className(root)).not.toContain("z-50"); // bare z-50 without hover: prefix
```

This fails when the root correctly has `hover:z-50`, because `className()` joins tokens into a string that contains the substring `z-50`. Adjusted to token-based check (consistent with the same test’s `z-30` assertion):

```ts
expect(treeHasClass(root, "z-50")).toBe(false); // bare z-50 without hover: prefix
```

Intent preserved; substring false positive removed.

## Files changed

| File | Change |
|------|--------|
| `client/app/board/html/hand.ts` | Root `hover:z-50`, `hand-tile-{id}` testid, face raise without z-30 |
| `client/app/board/html/hand.test.ts` | New hover stacking describe block (+ one assertion fix) |

## Verification

- `bunx vitest run app/board/html/hand.test.ts` — **10/10 PASS**
- No new linter issues on touched files

## Out of scope (Task 2)

- Scene/surface integration tests in `surfaces.test.ts`
- Live browser hover verification

---

## Follow-up fix for whole-branch review findings

### Findings addressed

- **Critical:** tile root hover stacking was non-functional because inline `"z-index"` overrode the Tailwind hover utility.
- **Important:** hover-stacking tests only checked class strings and would not fail if inline stacking still blocked hover.

### TDD evidence

#### RED

```bash
cd client && bunx vitest run app/board/html/hand.test.ts -t "hover stacking"
```

Result: **2 failed / 8 skipped**. The new assertions failed because the tile root did not use `[z-index:var(--hand-z)]` / `hover:[z-index:50]` and still relied on inline `"z-index"`.

#### GREEN

Changed `client/app/board/html/hand.ts` so the tile root now uses:

- class: `[z-index:var(--hand-z)] hover:[z-index:50]`
- style: `--hand-z: String(index + 1)`

This preserves:

- discard-selected tiles raising by translate only
- whole-bar hover elevation from the root
- `hand-tile-{id}` test ids

Re-ran:

```bash
cd client && bunx vitest run app/board/html/hand.test.ts -t "hover stacking"
```

Result: **2 passed / 8 skipped**.

### Final verification

```bash
cd client && bunx vitest run app/board/html/hand.test.ts app/board/hand-drag.test.ts && bunx tsc --noEmit -p tsconfig.json && bunx biome check --write app/board/html/hand.ts app/board/html/hand.test.ts
```

Result:

- `vitest`: **18 passed**
- `tsc --noEmit`: **passed**
- `biome check --write`: **passed, no fixes applied**
