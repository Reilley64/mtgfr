# Mulligan pre-game overlay

**Status:** approved design  
**Module:** `client/app/board/html/mulligan-bar.ts` (replace/retire bar chrome), new mulligan overlay view, `client/app/board/html/overlays.ts`, `client/lib/mulligan.ts`, turn/priority chrome spec

## Goal

Make opening-hand mulligans feel like Arena: a dedicated pre-game overlay owns the decision, not a strip above the action bar.

## Decisions (from brainstorming)

1. **Full pre-game overlay** while undecided — dimmed board, large hand faces in the overlay, Keep / Mulligan there; hide the normal hand bar for that phase.
2. **After Keep** — dismiss the overlay; return to the normal board + hand bar; waiting is a light banner until everyone keeps.
3. **Hard lock** while undecided — no pan/zoom/priority/hand-bar interaction under the overlay; only overlay chrome + Concede.

## Non-goals

- London mulligan (draw 7 then put N on bottom) or Vancouver scry.
- Mulligan timers, host force-keep, disconnect auto-keep.
- Spectator-specific mulligan chrome (spectators stay without Keep/Mulligan).
- Inspect / Alt-inspect from overlay faces (hard lock; faces are display-only).
- Wire/engine changes — snapshot fields (`mulliganing`, `hand_kept`, `can_mulligan`, `mulligans_taken`) remain the source of truth.

## Behavior

### Undecided local seat (`mulliganing && !local.hand_kept`)

- Render full-viewport overlay (`data-testid="mulligan-overlay"`):
  - Dimmed backdrop that captures pointer events (hard lock on board chrome underneath).
  - Title / status from existing `mulliganChrome` (e.g. “Opening hand”, “Keep this hand or take a mulligan.”).
  - Large faces for the viewer’s hand-zone objects (same art path as hand tiles / `cardArt`).
  - Keep (`mulligan-keep`) and Mulligan (`mulligan-take`, disabled when `!can_mulligan`) with current labels, including `Mulligan (N taken)` after the first.
- Hide `hand-bar` and do not show priority bar / prompts / activation radial (already suppressed while `mulliganing`).
- Concede remains available (existing concede control).
- Space / Enter remain inert while `mulliganing` (existing rule).

### On Mulligan

- Overlay stays open; faces refresh from the new hand snapshot after the redraw intent applies.
- Labels update via `mulliganChrome` (`mulligans_taken`).

### After local Keep (`hand_kept` while still `mulliganing`)

- Overlay is not shown.
- Normal board + `hand-bar` return.
- Light waiting banner (`data-testid` e.g. `mulligan-waiting`) shows the existing waiting copy from `mulliganChrome.status` (names undecided living seats; “All players kept. Starting game…” when none remain).
- No Keep / Mulligan buttons on the banner.

### When `mulliganing` becomes false

- Waiting banner clears; priority chrome returns as today.

## Architecture

- Keep pure `mulliganChrome` in `client/lib/mulligan.ts` as the copy/enablement source.
- Prefer a dedicated overlay module (e.g. `mulligan-overlay.ts`) over stretching `mulligan-bar.ts` into two unrelated layouts; retire or thin the bar so it is not the undecided surface.
- Composition in `overlays.ts`:
  - Undecided → overlay, no hand bar.
  - Kept + still mulliganing → hand bar + waiting banner.
  - Not mulliganing → hand bar + priority (unchanged).
- Hard lock is overlay-level pointer capture + omitting interactive layers (hand/priority/prompts), not a new camera lock subsystem unless board pointer mounts still steal events — if they do, gate those mounts while undecided.

## Visual

- Forest/HUD tokens consistent with docked prompt aims (`bg-forest-*`, vine border, snow text) — not a second visual language.
- Hand faces large enough to read as the hero of the phase (not peek tiles); wrap/scroll if seven wide faces overflow small viewports.
- Dim backdrop reads as “pre-game,” not an error modal.

## Testing

- Scene: undecided → `mulligan-overlay` + Keep, no `hand-bar` / `board-primary`.
- Scene: after Keep → no overlay, `mulligan-waiting` with named seats, `hand-bar` present.
- Scene / unit: Mulligan control disabled when `can_mulligan` is false; Space inert while mulliganing (existing).
- Update `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` in the same implementation change so behavior truth matches the overlay (not the old bar).

## Spec touch-ups (implementation)

- `2026-07-20-turn-and-priority-chrome.md`: replace “mulligan bar replaces priority bar” with overlay + post-keep waiting banner.
- Cross-link this design from that module spec’s behavior section if useful; do not duplicate engine mulligan rules already in `2026-07-21-mulligan-and-beacon-seed-design.md`.
