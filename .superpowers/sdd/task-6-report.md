# Task 6 Report: Living docs — DESIGN.md + surface specs

**Status:** Complete  
**Commit:** `docs: record shellFrame fonts and CSS landscape rotate`

## Summary

Updated living documentation to match Wave 1 shipped behavior: Manrope/Space Grotesk shell fonts via `shellFrame`, CSS portrait landscape rotate (`.landscape-rotate-root`), and removal of the portrait rotate dialog from specs.

## Files changed

| File | Changes |
|------|---------|
| `DESIGN.md` | Landscape Rule → CSS rotate, no dialog/reflow; Typography → `font-shell`/`font-display` + board `font-sans` |
| `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md` | User story, `shellFrame` section, landscape rotate subscription/class, typography, module list, safe-area note |
| `docs/superpowers/specs/2026-07-20-system-overlays.md` | Removed portrait gate from problem/behavior; out-of-scope points at app-root rotate |
| `docs/superpowers/specs/2026-07-20-board-composition.md` | Landscape Rule → CSS rotate at app root |
| `docs/superpowers/specs/2026-07-20-card-inspect.md` | Dropped portrait-gate from inspect z-order stack |
| `PRODUCT.md` | Portrait sentence: rotate prompt → CSS landscape rotate (no dialog) |

## Shipped behavior documented

- **`shellFrame`** (`client/app/shell/frame/shell-frame.ts`): shared full-bleed shell for auth, decks, lobby, leaderboard, coverage; `font-shell` body, `font-display` titles, atmosphere variants, header/stage/badge chrome.
- **Fonts:** Manrope + Space Grotesk loaded in `global.css` via `@fontsource`; tokens `--font-shell` / `--font-display` in `tokens.generated.css`; board HUD/canvas stays `font-sans` / `system-ui`.
- **Landscape rotate:** `matchMedia("(orientation: portrait) and (max-width: 900px)")` → `landscapeRotate.active` on app model; `landscape-rotate-root` class on `data-testid="landscape-root"`; CSS width/height swap + 90° rotate in `global.css`. No `#portrait-gate` dialog.

## Not updated (out of brief scope)

Cross-links in `README.md`, `deck-list-and-builder.md`, `lobby-entry-ui.md`, and design sidecars still mention "portrait gate" in index/link text — behavior is correct in the five surface specs + DESIGN.md; index cleanup can be a follow-up.

## Verification

Docs-only change; no code or test run required. Content aligned with `client/app/view.ts`, `client/app/subscriptions.ts`, `client/app/shell/frame/shell-frame.ts`, `client/styles/global.css`, and existing Scene tests (`#portrait-gate` absent, `landscape-rotate-root` present).
