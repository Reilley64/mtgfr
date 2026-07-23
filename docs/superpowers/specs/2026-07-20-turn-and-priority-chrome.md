# Turn and Priority Chrome
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/priority-bar.ts`, `client/app/board/html/turn-chrome.ts`, `client/app/board/html/discoverability.ts`, `client/app/board/html/sound-chrome.ts`, `client/app/board/html/keyboard-mount.ts`, `client/app/board/html/mulligan-bar.ts`

## Problem Statement

Players need clear turn, phase, priority, pass, yield, and shortcut controls without hiding playable board information. The chrome must make response windows obvious while using playable-outline language instead of dimming the board.

## Solution

Use `PriorityContextBar` for action controls, `TurnBanner` for active player and phase track, `HintStrip`/`LegendPanel` for discoverability, and a top-left sound toggle. Global keyboard mounts mirror the primary board actions.

## User Stories

- As the priority holder, I know whether to click Next, Resolve card, or confirm combat.
- As the active player, I can arm End Turn instead of repeatedly passing.
- As a non-active player, I can auto-pass Until my turn.
- As a player, I can use Space and Enter for common board actions.
- As a new player, I can discover drag, Alt inspect, Space pass, and badge meanings.
- As a player declaring attackers or blockers, I see a combat coach strip explaining drag-to-stage and Confirm.

## Behavior

- Empty stack with your priority shows the primary Next/combat confirmation button.
- Non-empty stack with your priority shows Resolve card and Resolve stack.
- Helpless non-empty-stack windows do not show a generic Next button.
- Active players see End Turn when the stack is empty and combat staging is not pending.
- Non-active players see the Until my turn rocker.
- Space mirrors the primary/pass action. Enter toggles End Turn or Until my turn.
- While `VisibleState.mulliganing` is true for a seated viewer, `mulliganBarView` replaces the priority bar (Keep / Mulligan). Space and Enter are inert until mulligans finish; Concede stays available.
- After the local seat keeps, the bar stays visible with a waiting status that names undecided living seats (username, or `P{seat}` when empty). Lost seats are omitted. When every living seat has kept, status is “All players kept. Starting game…”.
- `TurnBanner` shows five phase bands: Beginning, Main 1, Combat, Main 2, End, plus step detail when needed.
- `HintStrip` explains drag, activation click, Alt inspect, and Space pass; it auto-hides after 12 seconds and persists dismissal as `mtgfr.hintDismissed`.
- During local declare-attackers / declare-blockers windows, `board-combat-coach` shows drag-to-stage copy (independent of hint dismissal): attack → opponent life orb, block → attacker creature.
- `LegendPanel` explains badges, target/combat outlines, playable border, commander outline, and graveyard/exile outlines.
- Sound toggle sits in the top-left toolbar with legend controls and is visible to all viewers.
- Playability is communicated with playable borders and zone outlines, not with a dim veil over unplayable permanents.

## Implementation Decisions

- `priorityBarView` derives controls from current board model and `VisibleState`; server flags such as `yielded` and `turn_yielded` are authoritative.
- Stack yield is one-shot and disabled while armed until the stack empties.
- End Turn reuses `SetTurnYield`; there is no separate end-turn intent.
- The top-left toolbar has one fixed container for legend and sound controls.
- Global keyboard handling ignores inputs, textareas, selects, and button Space/Enter default activation.

## Testing Decisions

- Chrome tests cover Next, Resolve card, Resolve stack, End Turn, Until my turn, and staged cancel controls.
- Mulligan unit tests cover Keep/Mulligan affordances and waiting status that names undecided seats (including empty-username fallback).
- Keyboard tests cover Space, Enter, Escape, and Alt behavior without stealing text-input focus.
- Discoverability tests cover hint auto-hide, dismissal persistence, legend content, toolbar placement, and combat staging coach copy.
- Playable-chrome tests assert outlines/borders rather than dimming.

## Out of Scope

- Unconditional pass-turn shortcuts.
- Reintroducing board-wide dimming for instant-priority focus.
- Moving priority decisions client-side.

## Further Notes

- Table audio attention cues are fired from board audio data attributes and documented in the table audio spec.
