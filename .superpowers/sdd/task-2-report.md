# Task 2 Report: Post-keep waiting banner

## Status

**DONE**

## Summary

After a seated player keeps their opening hand during mulligans, the decision overlay now stays hidden, the `hand-bar` returns, and a centered `mulligan-waiting` banner renders the existing `mulliganChrome` waiting status copy.

## TDD Evidence

### RED

Updated `client/app/board/html/chrome.test.ts`:
- renamed `mulliganing kept seat does not show decision overlay`
- added assertions for `mulligan-waiting` existence and exact waiting copy

Ran:

```bash
cd client && bunx vitest run app/board/html/chrome.test.ts -t "waiting banner|mulligan kept"
```

Result: **FAIL**
- `Expected element matching testId "mulligan-waiting" to exist but it does not.`

### GREEN

Implemented the minimal change:

| File | Change |
|------|--------|
| `client/app/board/html/mulligan-overlay.ts` | Added `mulliganWaitingView(state): Html | null` using `mulliganChrome` and `data-testid="mulligan-waiting"` |
| `client/app/board/html/overlays.ts` | Wired `mulliganWaitingView(state)` for seated viewers immediately after `mulliganOverlayView(state)` |
| `client/app/board/html/chrome.test.ts` | Replaced kept-seat expectations with waiting-banner assertions |

Re-ran:

```bash
cd client && bunx vitest run app/board/html/chrome.test.ts -t "mulligan"
```

Result: **PASS** — `2 passed`

## Verification

Focused suite:

```bash
cd client && bunx vitest run app/board/html/chrome.test.ts
```

Result: **PASS** — `10 passed`

## Self-Review

- `mulliganWaitingView` reuses `mulliganChrome.status`, so Task 2 keeps the copy source of truth in `client/lib/mulligan.ts`.
- Guard-return-first shape stays intact: hidden unless `chrome.show && !chrome.showControls`.
- Overlay composition remains ordered: undecided seats still get `mulligan-overlay`; kept seats get only `mulligan-waiting`.
- `hand-bar` visibility remains driven by `undecidedMulligan`, so it returns immediately after keep without extra state.
- No wire, engine, or docs changes were introduced.

## Concerns

None.
