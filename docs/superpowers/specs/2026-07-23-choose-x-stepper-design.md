# Choose-X stepper restore (Foldkit)

**Date:** 2026-07-23  
**Status:** Approved (autonomous improvement loop)  
**Context:** `#69` shipped Arena-aligned choose-X with Min/−/field/+/Max and a live mana-cost preview, backed by server `min_x` / `max_x` / `x_cost`. The Foldkit migration (`#74`) kept the wire fields and `XPromptState.xCost` but replaced the UI with one button per legal X and dropped `client/lib/xCost.ts`. With `max_payable_x` capped at 255, high-mana X spells can render dozens of buttons and never show what you will pay.

## Goals

1. Restore a clamped stepper for local pre-submit X prompts (cast/activate).
2. Show a live resolved-cost preview using the already-projected `x_cost` + `x_symbols`.
3. Keep submit on the existing cast/activate intent `x` field; engine remains authoritative.
4. Cover the surface with Scene / unit tests that assert outcomes (preview, clamp, confirm), not migration parity.

## Non-goals

- Changing `.proto` / engine affordability (`max_payable_x`).
- Oracle-derived “enters as N/N” hints.
- Denylist of sparse illegal X values (Living Breakthrough) — still out of scope.
- Engine `pending_choice` X formulators (none today); this is the board-local `xPrompt` path only.

## Approaches considered

| Approach | Trade-off |
|----------|-----------|
| **A. Foldkit draft stepper in `BoardModel`** | Matches modal draft pattern; pure update; testable. **Chosen.** |
| B. Cap button list + text preview | Still explodes for large ranges; worse than stepper. |
| C. DOM-local Mount stepper | Fights Foldkit Model/update; harder Scene tests. |

## Design

### State

Extend `XPromptState` with `draftX: number`, initialized to `clampX(maxX, minX, maxX)` when the prompt opens (prefer max affordable, matching `#69`).

### Messages

- `XDraftSet { x: number }` — clamp into `[minX, maxX]` and store on `xPrompt.draftX`.
- Keep `XSubmitted { x }` for confirm (submit uses `draftX` from a Confirm button, or pass the clamped value).

Min / − / + / Max are view convenience clicks that dispatch `XDraftSet` with the computed next value. An optional number input may dispatch `XDraftSet` on change.

### Preview

Pure helpers restored under `client/lib/xCost.ts`:

- `clampX(value, min, max)`
- `costWithChosenX(cost, x)` → WireCost with `generic += x * x_symbols`, `has_x: false`

Preview renders as `Pay ${costText(costWithChosenX(xCost, draftX))}` (brace string). Text is used instead of mana-font pips so generics above mana-font’s 0–20 range (common for high X) never collapse to a false `{0}`.

### View

`boardXPrompt` becomes:

1. Title: `Choose X for {name}`
2. Preview row: Pay + resolved pips (`data-testid="x-prompt-preview"`)
3. Stepper: Min, −, value display/input (`x-prompt-value`), +, Max (`x-prompt-min` / `dec` / `inc` / `max`)
4. Confirm (`x-prompt-confirm`) → `XSubmitted({ x: draftX })`
5. Cancel (existing)

Disabled: − when `draftX <= minX`, + when `draftX >= maxX`.

### Spec updates

Update `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md` so current behavior documents the stepper (remove “button list” / “stepper out of scope”).

## Testing

1. Unit: `clampX` / `costWithChosenX` (Hangarback `{X}{X}`, colored pips preserved).
2. Scene: `x-prompt` shows preview and stepper controls; confirm not a wall of `x-prompt-n` buttons.
3. Update fold: `XDraftSet` clamps; `XSubmitted` still submits intent with chosen `x`.

## Error / edge cases

| Case | Behavior |
|------|----------|
| `maxX < minX` | `clampX` returns `minX`; stepper stuck at min (server bug; client stays safe) |
| Empty range (`maxX === minX`) | Single legal value; −/+ disabled; Confirm works |
| Missing `x_symbols` | Treat `has_x ? 1 : 0` (same as `#69`) |
| Cancel | Clears `xPrompt` (existing `CancelActionClicked`) |
