# Card Inspect
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/inspect.ts`, `client/lib/deck-builder/card-hover-preview.ts`, `client/app/board/html/keyboard-mount.ts`, `client/app/board/submodel.ts`

## Problem Statement

Players need to inspect face-up cards and current battlefield modifiers without losing board context or having the preview hidden beneath prompts and system overlays.

## Solution

Alt/Option pins a card into a shared preview `dock` mode. The dock has a full-board backdrop, card art on the left, oracle/approximations and modifier ledger on the right, and it is the topmost board layer while pinned.

## User Stories

- As a player, I can hold Alt over a face-up card to read it.
- As a player, I can inspect hand, stack, and battlefield cards with one behavior.
- As a player, I can see live modifier contributions on a battlefield permanent.
- As a player, I can dismiss inspect with Alt release, Escape, or backdrop click.

## Behavior

- `AltDown` pins the card under the cursor, preferring hand/stack auxiliary hover over battlefield hit.
- `AltUp` dismisses the dock and clears fetched card data.
- Prepared DFC pins default to the back face until catalog data arrives.
- The Flip button appears for cards with a back face.
- `InspectPin` carries name, object/card ids, print, and prepared state.
- `FetchInspectCard` loads catalog data for oracle and faces.
- Battlefield object modifiers render as a grouped modifier ledger by source name.
- Space is blocked while the dock is open through keyboard dismissal priority.
- Inspect is topmost in the board layer stack: above prompts, HUD, pile overlay, concede dialog, result overlay, and portrait gate when present.

## Implementation Decisions

- Board inspect reuses `cardHoverPreviewView` with `mode: "dock"` instead of a separate card-preview component.
- `inspectView` is a thin board wrapper that supplies live modifier extras and board messages.
- Dismissal has no close button in the dock; backdrop/Escape/Alt release are the dismissal paths.
- The overlay is rendered last in `boardOverlays`.

## Testing Decisions

- Scene/unit tests cover Alt pin, dock backdrop, left art, right oracle/extras, DFC flip, and dismissal.
- Layer tests should assert inspect renders above prompt/system overlay DOM.
- Keyboard tests cover Escape dismissing inspect before radial/action cancellation.

## Out of Scope

- Marked damage in the modifier ledger.
- Reflowing the board for portrait screens.
- Inspecting hidden private cards for non-owners.

## Further Notes

- The shared preview also supports cursor-follow mode outside the board; this spec only covers board dock mode.
