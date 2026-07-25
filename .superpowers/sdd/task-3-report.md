# Task 3 Report: Activation menu view + Scene tests

## Status

**DONE_WITH_CONCERNS**

## Summary

- Replaced the selected-permanent SVG radial overlay with a card-anchored activation menu in `client/app/board/html/activation-menu.ts`.
- Moved `selectedRadialOptions` into the new menu file and rewired `overlays.ts` and `submodel.ts`.
- Replaced the three radial Scene assertions in `client/app/board/scene.test.ts` with activation-menu assertions.
- Deleted `client/app/board/html/activation-radial.ts`.

## TDD Evidence

### RED

Updated the three Scene tests in `client/app/board/scene.test.ts` before production changes:

- `selected permanent with tap-for-mana shows activation menu row`
- `selected tapped mana source keeps its disabled tap-for-mana row visible`
- `activation menu is placed beside the selected card screen center`

Ran:

```bash
cd /workspace/client
bunx vitest run app/board/scene.test.ts -t "activation menu|tap-for-mana shows|disabled tap-for-mana"
```

Result: **FAIL**
- Exit code: `1`
- Failure signal matched expectation: `activation-menu` / `activation-menu-panel` were absent while the old radial still rendered.
- I adjusted the disabled-row assertion from a nonexistent Foldkit `toHaveAttribute(...)` matcher to an equivalent selector assertion:
  - `Scene.selector('[data-testid="activation-menu-row-tap_for_mana"][aria-disabled="true"]')`
- Re-ran the same command and kept the suite red for the correct reason: missing activation-menu DOM.

### GREEN

Implemented the new menu view and wiring, then ran:

```bash
cd /workspace/client
bunx vitest run app/board/scene.test.ts app/board/geometry/radial.test.ts
```

Result: **PASS**
- Exit code: `0`
- `2` files passed
- `105` tests passed

## Implementation Notes

- `activation-menu.ts`
  - Reused the old radial pointer/keyboard messages: `RadialWedgeArmed`, `RadialWedgeReleased`, `RadialWedgeHovered`, `RadialOptionPicked`
  - Kept `data-wedge` on each row
  - Used `activationMenuPlacement(...)` and `activationMenuEstimatedHeight(...)`
  - Rendered tap and mana cost chips via `manaFontClass("T")` and `costPips(...)`
- `overlays.ts`
  - Swapped `activationRadialView(...)` for `activationMenuView(...)`
- `submodel.ts`
  - Pointed `selectedRadialOptions` import at `./html/activation-menu`

## Verification

### Focused lint

Ran:

```bash
cd /workspace/client
bunx biome check --formatter-enabled=false app/board/html/activation-menu.ts app/board/html/overlays.ts app/board/scene.test.ts app/board/submodel.ts
```

Result: **PASS**
- Exit code: `0`

### Typecheck

Ran:

```bash
cd /workspace/client
bun run typecheck
```

Result: **PASS**
- Exit code: `0`
- `tsc --noEmit` passed
- The script regenerates wire/tokens first; I reverted the incidental generated-file churn afterward so the task diff stayed scoped.

### Focused tests

Ran:

```bash
cd /workspace/client
bunx vitest run app/board/scene.test.ts app/board/geometry/radial.test.ts
```

Result: **PASS**
- Exit code: `0`
- `105` tests passed

### Full client-check attempt

Ran:

```bash
just client-check
```

Result: **FAIL**, but not because of Task 3 behavior
- `client/app/board/html/activation-menu.ts` initially needed import organization; fixed
- Remaining blockers were repo/environment-level:
  - `biome.json` schema version mismatch (`2.5.3` config vs `2.5.5` CLI)
  - pre-existing `lint/style/noNonNullAssertion` warning/error in `client/app/board/geometry/radial.ts` line 122

## Self-review

- Scope stayed within Task 3: no wedge retirement, no spec rewrite.
- Existing `RadialOptionPicked` outcome tests were left intact and still pass.
- Unrelated formatter/generated-file churn from verification commands was reverted.

## Files Changed

- `client/app/board/html/activation-menu.ts`
- `client/app/board/html/overlays.ts`
- `client/app/board/submodel.ts`
- `client/app/board/scene.test.ts`
- `client/app/board/html/activation-radial.ts` (deleted)

## Concerns

- `just client-check` is still red due pre-existing repo/tooling issues outside Task 3.
