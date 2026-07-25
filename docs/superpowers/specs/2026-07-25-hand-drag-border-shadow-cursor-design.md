# Hand drag border, flight shadow, and cursors

**Status:** approved design  
**Module:** `client/app/board/html/hand.ts`, `client/app/board/html/hand-drag-mount.ts`, `client/app/board/bitmap/paint-flights.ts`, `design.tokens.json` (`drop-shadow.drag`), `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`, `docs/superpowers/specs/2026-07-20-flights.md`

## Goal

While dragging from the action bar, the playable border follows the ghost (not the faded source). Drag and canvas flights share a stronger Arena-like lift shadow. Cursors communicate playability and grab state.

## Non-goals

- Painting playable rings on Mount/canvas flights after release
- Changing drag play threshold, fan geometry, or hover bring-to-front stacking
- Priority chrome or mulligan overlay cursors
- Engine/wire changes

## Behavior

### Playable border (hand-drag ghost only)

1. When `handDrag` is active for a bar tile, the **source** face keeps `opacity-25` but loses playable/zone-playable aura (`ring-playable-border` and layered playable outlines).
2. The **ghost** uses `barZoneAura(zone, true)` for the zone being dragged (hand mint; command/gy/exile keep layered playable + zone outlines).
3. On drag end or cancel, the source returns to normal aura rules.
4. After release, canvas flights do **not** carry the playable ring.

### Shadows

1. Deepen `--drop-shadow-drag` in `design.tokens.json` (regenerate tokens) so the ghost lift reads more Arena-like; ghost continues to use `drop-shadow-drag`.
2. `paintFlightCard` raises canvas shadow blur / offset / alpha to match that lift family (bitmap cannot consume the CSS token directly; pick concrete numbers that visually match).
3. Resting hand tiles stay on `--shadow-hand`.

### Cursors

On the tile hit strip:

| State | Cursor |
|-------|--------|
| Unplayable (idle) | `cursor-not-allowed` |
| Playable (idle) | `cursor-grab` |
| Any active `handDrag` | `cursor-grabbing` on `document.documentElement` (or `body`), cleared on drag end / cancel / pointercancel |

Discard-select and Alt-inspect behavior are unchanged aside from the unplayable cursor on idle hits.

## Implementation notes

- Prefer view/CSS + flight paint constants (no new board-model cursor enum).
- Ghost must receive the correct **zone** for `barZoneAura` (not hard-coded `"hand"` when dragging command/gy/exile).
- Grabbing cursor: set in `hand-drag-mount` teardown pair or from a small view effect when `handDrag != null`; must clear even if the view unmounts mid-drag.
- Difficulty of shadow polish: low (token + `paint-flights.ts` constants).

## Spec truth

Update `hand-and-zone-bar` for drag border move, cursors, and ghost shadow. Update `flights` for stronger in-flight shadow. Cross-link this design.

## Testing

- `hand.test.ts`: with `handDrag`, source face has no `ring-playable-border`; ghost has playable aura; hit classes include `cursor-not-allowed` / `cursor-grab` as appropriate.
- Assert grabbing cursor is applied while drag is live and cleared after end (mount unit or Scene with a `data-testid` / document style probe — prefer mount teardown unit if Scene cannot see `documentElement` style).
- Flight paint unit: assert new shadow blur/offset/alpha constants (or draw contract) so a silent regress to the old soft shadow fails.
- Token test: update expected `--drop-shadow-drag` string when the token changes.
