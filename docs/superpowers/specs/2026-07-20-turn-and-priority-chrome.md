# Turn and Priority Chrome
**Status:** Current (as of 2026-07-27)
**Module:** `client/app/board/html/priority-bar.ts`, `client/app/board/html/prompt-bar-actions.ts`, `client/app/board/html/turn-chrome.ts`, `client/app/board/html/discoverability.ts`, `client/app/domain/combatCoach.ts`, `client/app/board/html/sound-chrome.ts`, `client/app/board/html/keyboard-mount.ts`, `client/app/board/html/mulligan-overlay.ts`

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
- As a player declaring attackers or blockers, I see a combat coach strip explaining drag-to-stage, click-to-cancel, and Confirm.

## Behavior

- Empty stack with your priority shows the primary Next/combat confirmation button.
- Non-empty stack with your priority shows Resolve card and Resolve stack.
- Helpless non-empty-stack windows do not show a generic Next button.
- Active players see End Turn when the stack is empty, combat staging is not pending, and the engine does not require any attackers (`required_attacks` empty — goad / must-attack). While End Turn is already armed, the control stays visible so the seat can cancel.
- Non-active players see the Until my turn rocker.
- Any classified prompt (`promptPresentation(...).mode !== "none"`) suppresses idle priority-bar actions (`Next`, `Resolve card`, `Resolve stack`, `End Turn`, `Until my turn`, and the bar-level `Cancel`) so prompt chrome can take over the slot; `board-reject` remains visible.
- Simple prompt takeovers reuse that slot directly: `priority-context-bar` renders the answer buttons for `may_yes_no` / `dance_exile_more`, optional-pay prompts, `choose_mode`, `choose_trigger_modes` submit/cancel, pile picks, revealed/countered destination picks, board-aim Confirm / Decline / Assign / Cancel actions, and local `playModePick` / both phases of `modalCast`, while the matching bottom coach stays informational-only.
- Board-aim simple prompts keep their existing bottom-docked `*-aim` coach placement above the hand bar; takeover changes only the primary-action slot, not coach placement.
- Rich prompt presentations (`promptPresentation(...).mode === "modal"`) leave `priority-context-bar` with no prompt buttons and instead use centered `*-modal` shells with a dimmed backdrop; no idle priority companions remain visible under the modal.
- Pure on-board staged targeting without a staged picker keeps the existing bar-level `Cancel` control; classified prompt flows such as `playModePick` and staged `preferPick` target pickers hand off cancellation to prompt chrome.
- Space mirrors the primary/pass action. Enter toggles End Turn or Until my turn.
- While `VisibleState.mulliganing` is true and the local seated viewer has not kept (`!hand_kept`), `mulliganOverlayView` shows full-viewport `mulligan-overlay` (dimmed hard-lock backdrop, large opening-hand faces via shared `cardArt` / `BindCardArt`, Keep / Mulligan). Opening-hand prints resolve through `CardArtTick` passthrough on the board submodel boundary (not `GotBoardMessage`). Status copy from `mulliganChrome` explains the friendly first mulligan (free redraw to 7, no London bottom) and, after that, the next hand size. The normal `hand-bar` and priority bar are hidden. Space and Enter stay inert; Concede remains available above the overlay.
- After the local seat keeps while others are still deciding, the overlay dismisses, `hand-bar` returns, and `mulligan-waiting` shows waiting copy that names undecided living seats (username, or `P{seat}` when empty). Lost seats are omitted. When every living seat has kept, status is “All players kept. Starting game…”.
- `TurnBanner` shows five phase bands: Beginning, Main 1, Combat, Main 2, End, plus step detail when needed.
- `HintStrip` explains drag, activation click, Alt inspect, and Space pass; it auto-hides after 12 seconds and persists dismissal as `mtgfr.hintDismissed`.
- During local declare-attackers / declare-blockers windows, `board-combat-coach` shows drag-to-stage and click-to-cancel copy (independent of hint dismissal): attack → opponent life orb (click attacker to un-stage), block → attacker creature (click blocker to un-stage).
- `LegendPanel` explains badges, target/combat outlines, playable border, commander outline, and graveyard/exile outlines.
- Sound toggle sits in the top-left toolbar with legend controls and is visible to all viewers.
- Playability is communicated with playable borders and zone outlines, not with a dim veil over unplayable permanents.

## Implementation Decisions

- `priorityBarView` derives controls from current board model and `VisibleState`; server flags such as `yielded` and `turn_yielded` are authoritative.
- `promptPresentation(board, state)` is the single decision-chrome classifier for the priority bar: it checks local prompt sessions before the viewer-owned `pending_choice`, returns `none | simple | modal`, classifies staged pickers from `stagedPickTargets(...)` as local modals, and leaves only pure staged on-board arrow aim on the legacy Cancel path.
- Unknown or uncategorized pending kinds fall back to `modal`, so the bar hides idle controls instead of guessing new simple-button layouts.
- Stack yield is one-shot and disabled while armed until the stack empties.
- End Turn reuses `SetTurnYield`; there is no separate end-turn intent.
- The top-left toolbar has one fixed container for legend and sound controls.
- Global keyboard handling ignores inputs, textareas, selects, and button Space/Enter default activation.

## Testing Decisions

- Chrome tests cover Next, Resolve card, Resolve stack, End Turn, Until my turn, and staged / parked play-mode cancel controls.
- Surface Scene tests cover simple and modal prompt takeovers hiding idle priority actions, including yes/no, pay-cost, mode, pile, destination, board-aim commit/decline/cancel actions, play-mode, modal-mode actions moving into `priority-context-bar`, and centered rich-prompt shells such as library search / color / creature-type / card-name while the matching coach prompt stays button-free.
- `promptPresentation.test.ts` covers the chrome classifier itself, including `simple` vs `modal`, board-aim tagging, pure staged-target `none`, staged `preferPick` modal classification, and modal fallback for uncategorized/off-board prompt cases.
- Chrome Scene tests cover the undecided `mulligan-overlay`, disabled `mulligan-take`, and the post-keep `mulligan-waiting` banner with the restored `hand-bar`.
- A live board Scene test resolves mulligan `BindCardArt` with `CardArtTick` through the app `toParentMessage` boundary so art ticks are not wrapped as `GotBoardMessage`.
- Mulligan unit tests cover Keep/Mulligan copy, enablement, and waiting status that names undecided seats (including empty-username fallback).
- Keyboard tests cover Space, Enter, Escape, and Alt behavior without stealing text-input focus.
- Discoverability tests cover hint auto-hide, dismissal persistence, legend content, toolbar placement, and combat staging coach copy.
- Playable-chrome tests assert outlines/borders rather than dimming.

## Out of Scope

- Unconditional pass-turn shortcuts.
- Reintroducing board-wide dimming for instant-priority focus.
- Moving priority decisions client-side.

## Further Notes

- Table audio attention cues are fired from board audio data attributes and documented in the table audio spec.
- Prompt takeover rationale and classification matrix are documented in [prompt-primary-bar-takeover-design](2026-07-27-prompt-primary-bar-takeover-design.md).
