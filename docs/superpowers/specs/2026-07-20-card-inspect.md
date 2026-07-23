# Card Inspect
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/inspect.ts`, `client/lib/deck-builder/card-hover-preview.ts`, `client/lib/inspect.ts`, `client/app/board/html/keyboard-mount.ts`, `client/app/board/submodel.ts`

## Problem Statement

Players need to inspect face-up cards and current battlefield modifiers without losing board context or having the preview hidden beneath prompts and system overlays. Life orbs show only max commander damage (`Cmd N`); players also need the per-source 21-damage breakdown without cluttering the battlefield.

## Solution

Alt/Option pins a card into a shared preview `dock` mode. The dock has a full-board backdrop, card art on the left, oracle/approximations and modifier ledger on the right, and it is the topmost board layer while pinned. Alt over a life orb pins that seat into a text-only player dock with life and per-commander damage rows.

## User Stories

- As a player, I can hold Alt over a face-up card to read it.
- As a player, I can inspect hand, stack, and battlefield cards with one behavior.
- As a player, I can see live modifier contributions on a battlefield permanent.
- As a player, I can see marked damage on a damaged battlefield permanent.
- As a player, I can hold Alt over a life orb to see that seat’s life and per-commander damage breakdown.
- As a player, I can dismiss inspect with Alt release, Escape, or backdrop click.

## Behavior

- `AltDown` pins the card under the cursor, preferring hand/stack auxiliary hover over battlefield hit; when no card hit, a life-orb avatar hit pins that seat.
- `AltUp` dismisses the dock and clears fetched card data.
- Prepared DFC pins default to the back face until catalog data arrives.
- The Flip button appears for cards with a back face.
- `InspectPin` carries name, object/card ids, print, prepared state, and optional `playerSeat` for life-orb pins.
- `FetchInspectCard` loads catalog data for oracle and faces (card pins only).
- Battlefield object modifiers render as a grouped modifier ledger by source name.
- When the pinned live object has `marked_damage > 0`, the dock shows a `Marked damage: N` line (`inspect-marked-damage`) above the modifier ledger.
- Player pins render a text-only dock (`inspect-overlay`) with `Life: N` (`inspect-player-life`) and, when `commander_damage` has rows, a `Commander damage` panel (`inspect-commander-damage`) listing each source as `Owner[: — Commander]: amount / 21` (`inspect-commander-damage-{seat}`). Orb paint stays max-only `Cmd N`.
- Space is blocked while the dock is open through keyboard dismissal priority.
- Inspect is topmost in the board layer stack: above prompts, HUD, pile overlay, concede dialog, result overlay, and portrait gate when present.

## Implementation Decisions

- Board inspect reuses `cardHoverPreviewView` with `mode: "dock"` for card pins; player pins use a matching backdrop/content shell without BindCardArt.
- `inspectView` is a thin board wrapper that supplies live modifier extras, player commander-damage extras, and board messages.
- `commanderDamageBreakdown` in `client/lib/inspect.ts` labels sources by owner username (fallback `P{seat}`) and appends a visible `is_commander` object name when present.
- Dismissal has no close button in the dock; backdrop/Escape/Alt release are the dismissal paths.
- The overlay is rendered last in `boardOverlays`.

## Testing Decisions

- Scene/unit tests cover Alt pin (card and life orb), dock backdrop, left art, right oracle/extras, marked damage, commander-damage rows (present/absent), DFC flip, and dismissal.
- Layer tests should assert inspect renders above prompt/system overlay DOM.
- Keyboard tests cover Escape dismissing inspect before radial/action cancellation.

## Out of Scope

- Reflowing the board for portrait screens.
- Inspecting hidden private cards for non-owners.
- HTML chips or multi-source labels on the life orb itself (orb stays max-only).

## Further Notes

- The shared preview also supports cursor-follow mode outside the board; this spec only covers board dock mode.
- Sibling: [`2026-07-20-battlefield.md`](2026-07-20-battlefield.md) for orb `Cmd N` paint.
