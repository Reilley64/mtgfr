# Board: disable native browser text selection

**Status:** Approved (approach 1)  
**Date:** 2026-07-23  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74)

## Problem

Click-dragging on the in-game board selects DOM/canvas content like website text: a green (browser) highlight appears and can stick. That is native `user-select`, not game selection.

## Decision

Add Tailwind `select-none` on the board root (`data-testid="board-mount"`, both connecting and live views). Scope is the table only — lobby, deck builder, and form inputs stay selectable for copy/paste.

No `selectstart` handlers; no `::selection` overrides (not needed if selection cannot start).

## Success

Drag across permanents / chrome does not produce a native text selection highlight; game pointer gestures (select, combat drag, hand drag) unchanged.
