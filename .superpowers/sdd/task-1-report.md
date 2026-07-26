# Wave 3 Task 1 Report: Icons + HTML meta

**Status:** DONE
**Branch:** `cursor/foldkit-submodels-pwa-design-a1bf`
**Base:** `6e1a157`

## Summary

Added install-surface icon assets from `client/public/favicon.svg`, wired `theme-color` plus `apple-touch-icon` in `client/index.html`, added the requested HTML fixture test, and updated the living shell spec to describe the shipped brand asset set.

## Changes

- Created `client/public/pwa-192.png` (192x192), `client/public/pwa-512.png` (512x512), and `client/public/apple-touch-icon.png` (180x180) from the dragon-on-disc SVG.
- Added `<meta name="theme-color" content="#0B1310" />` and `<link rel="apple-touch-icon" href="/apple-touch-icon.png" />` to `client/index.html`.
- Added `client/app/pwa-html.test.ts` to lock the required head tags.
- Updated `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md` brand display section for the new PNG install assets and HTML head wiring.

## Verification

### Red phase

Ran `cd /workspace/client && bun run test app/pwa-html.test.ts`.

Observed the expected failure because `index.html` did not yet contain the `theme-color` meta tag.

### Focused verification

Ran `cd /workspace/client && bun run test app/pwa-html.test.ts app/domain/favicon-assets.test.ts`.

Result: 2 files passed, 5 tests passed.

### Asset verification

Generated and verified raster output sizes with a temporary Sharp install under `/tmp/pwa-icons`:

- `client/public/pwa-192.png` → `192x192`
- `client/public/pwa-512.png` → `512x512`
- `client/public/apple-touch-icon.png` → `180x180`

### Full client verification

Ran `cd /workspace && just client-check`.

Result:

- token freshness check passed
- mana oracle check passed
- codegen + token generation passed
- Biome format passed
- Biome lint passed
- `tsc --noEmit` passed
- Vitest passed: 112 files, 1153 tests

## Concerns

None.
