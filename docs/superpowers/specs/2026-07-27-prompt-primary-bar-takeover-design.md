# Prompt primary-bar takeover — design

**Status:** Design (2026-07-27). Living surface specs remain the source of truth after
implementation:
[`2026-07-20-prompts-and-pending-choices.md`](2026-07-20-prompts-and-pending-choices.md),
[`2026-07-20-turn-and-priority-chrome.md`](2026-07-20-turn-and-priority-chrome.md).

## Problem

When a prompt is open, the priority context bar still shows Next / Resolve /
End Turn / yield beside (or under) docked prompt chrome. Players see two
competing decision surfaces. Simple answers belong in the primary action slot;
rich pickers need a dedicated panel without leftover priority buttons.

## Goal

One decision surface at a time:

- **Simple answers** take over the primary-bar button slot (same place as Next).
- **Rich pickers** use a true center modal and also hide primary-bar actions.
- **Board-targeting coaches** stay bottom-docked above the hand / action area
  (today’s placement) — never a viewport-center modal.

## Non-goals

- New pending-choice wire kinds or answer/intent semantics.
- Hand bar, mana tray, or waiting-banner restyle.
- Keyboard remapping (keep existing prompt Space/Enter rules).
- Changing staged on-board targeting’s Cancel + priority path.

## Approach

**One decision-chrome owner** for the primary-bar slot (reshape
`priorityBarView` / introduce `decisionChromeView`). A thin
`promptPresentation(board, state)` map classifies the active local or engine
prompt as `none` | `simple` | `modal`.

| Board state | Primary-bar slot | Extra chrome |
| --- | --- | --- |
| Idle priority | Today’s Next / Resolve / End Turn / yield | — |
| **Simple** prompt | That prompt’s answer buttons only (companions hidden) | Slim coach: title / count. Board-aim coaches stay **bottom-docked** above the hand bar (today’s `*-aim` vertical placement). Non-aim simple coaches (Yes/No title, etc.) use the same bottom-centered coach strip without action buttons. |
| **Modal** prompt | Empty — priority action row hidden | Viewport-center modal (dimmed backdrop) with title + content + its own actions |
| Staged on-board targeting | Unchanged Cancel + normal priority controls | Existing staged hint |
| Someone else’s pending choice | Unchanged (no interactive prompts) | Existing waiting banner |
| Mulligan undecided | Bar already hidden | Existing mulligan overlay |

Unknown new kinds default to `modal` (hide primary; do not invent bar buttons).

Local answer prompts (X, modal modes, cost picks, play-mode) participate.
Staged on-board targeting sessions that already use Cancel on the bar do **not**
use this takeover.

### Simple vs modal classification

**Simple** (coach + bar actions):

- `may_yes_no`, `dance_exile_more`
- Optional-pay family (`pay_cost`, `pay_or_counter`, …)
- `choose_mode`; local `play-mode` / `modal-mode` row picks
- `choose_trigger_modes`: mode toggles in a slim coach strip; Choose / Cancel in the bar
- Pile A/B, Top/Bottom (and similar short binaries such as Battlefield/Hand)
- On-board aim Confirm / Decline / Assign (target, damage, divide, player-aim, GY/exile aim when answer is commit/decline)

**Modal** (center modal; primary actions hidden):

- Color / mana color
- Library search, creature type, card name
- Card-pick grids, off-board player-pick lists
- Arrange / select-top / distribute / partition lanes, order triggers
- X stepper, join-forces / draw-count number UIs
- Other scrollable / multi-control pickers that are not board-aim commit chrome

### Board-target coach placement

If the prompt requires aiming or clicking something on the board (highlighted
permanents, life orbs, pile cards, hand faces for discard-cost select), the
coach strip stays **fixed above the hand bar** at today’s docked position. It
does not move to a viewport-center modal. Only the answer buttons relocate into
the primary-bar slot (when the kind is `simple`).

### Components & data flow

- `promptPresentation(board, state)` — local session prompts first, then the
  viewer’s `pending_choice`; returns mode + enough context for views.
- `decisionChromeView` — owns the primary-bar slot:
  - `none` → existing priority buttons
  - `simple` → prompt answer buttons (same messages/intents as today)
  - `modal` → no action buttons in the slot
- `promptsView` — content only for `simple` (coach); full modal host for
  `modal` (backdrop + panel + actions).
- Formulators, drafts, and `choiceIntent` stay unchanged — presentation
  ownership only.
- Keyboard: existing `trySubmitReadyPendingDraft` / prompt Enter rules; no
  special remapping for bar takeover.

### Rejected alternatives

1. **Patch both views independently** — two owners of the primary slot; easy to
   show Next under a modal by mistake.
2. **Presentation registry alone without a single slot owner** — useful map,
   but insufficient without consolidating the bar.

## Specs to update at implement time

- [`prompts-and-pending-choices`](2026-07-20-prompts-and-pending-choices.md) —
  coach vs modal presentation; actions in primary bar for simple kinds;
  board-aim coach stays bottom-docked.
- [`turn-and-priority-chrome`](2026-07-20-turn-and-priority-chrome.md) —
  decision chrome ownership; hide priority companions while any classified
  prompt is open.

This file remains design input, not a living surface spec.

## Testing

- Unit: `promptPresentation` covers `simple` / `modal` / `none`, non-decider,
  and staged-target staying off takeover.
- Scene: simple Yes/No → `board-primary` absent; `prompt-yes` / `prompt-no`
  inside `priority-context-bar`; bottom coach present.
- Scene: modal library search → primary actions absent; center modal testids
  present (not bottom `*-aim` dock for that kind).
- Scene: on-board aim → bottom coach + Assign/Confirm/Decline in the bar.
- Chrome regression: idle Next / End Turn when no prompt.

## Out of scope

- Introducing brand-new pending-choice oneof arms.
- Client-side inference of unavailable kinds.
- Redesigning non-decider waiting chrome.
- Unconditional pass-turn shortcuts.
