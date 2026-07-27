# Hand play-mode chooser — design

**Status:** Implemented (2026-07-26). Surface specs are the source of truth:
[`2026-07-20-hand-and-zone-bar.md`](2026-07-20-hand-and-zone-bar.md),
[`2026-07-20-prompts-and-pending-choices.md`](2026-07-20-prompts-and-pending-choices.md)
(local session chrome; cross-link flights/stack staging as needed)

## Problem

A hand card with more than one legal action is rendered as multiple hand-bar
tiles. `handExtras` emits a second (or third) tile for every non-primary
`ActionView` on the same object. Valley Rannet’s mountaincycling and
forestcycling therefore appear as two cards, both captioned “Discard”. The same
duplication hits cast+cycle and cast+hand-ability whenever more than one mode is
legal.

## Goal

One hand tile per card. Playing the card chooses among its currently legal
modes. If exactly one mode is legal at that timing, that mode runs with no
chooser. Drag-to-play parks the card in the stack slot while the chooser is
open; Cancel returns the card to hand without submitting an intent.

## Non-goals

- Engine, proto, or schema changes (legal actions stay one `ActionView` each).
- Merging typecycling into a single ability or changing landcycling rules text.
- Collapsing multi-action tiles in graveyard / exile / command sections.
- Changing modal *spell* mode picking (`modalCast`) after a cast mode is chosen.

## Approach

**Client-only play-mode session** (rejected: schema composite `play_modes`
action; rejected: engine pending “choose hand mode”). The engine already lists
what is legal; the hand bar should not duplicate faces for alternate actions.

### Resting hand tile

- Render **one** tile per hand card. Stop appending `handExtras` as extra hand
  slots.
- Playable border when the object has one or more legal hand-section actions.
- Cost pips always show the card’s printed / normal cast cost (`ObjectView`
  mana cost), never a cycle or hand-ability cost.
- Caption:
  - Multi-mode (`legalModes.length > 1`): no Cycle/Discard caption.
  - Sole legal mode is `cycle` or `activate_hand_ability`: keep today’s
    Cycle / Discard caption.
  - Sole cast / play_land: no ability caption (unchanged).

### Play gesture

On click or drag-above-threshold, resolve
`legalModes = hand-section ActionViews for that object`:

| Count | Behavior |
| --- | --- |
| 0 | No-op (unplayable). |
| 1 | Run the existing `planRunAction` / cost / target / `modalCast` pipeline for that `ActionView`. |
| 2+ | Enter local `playModePick` session: hide the hand tile, park the card in the stack slot (same staged-ghost / flight ownership pattern as an in-progress cast), open docked play-mode aim. |

Click and drag use the same mode resolution. Drag does **not** auto-pick a
primary mode when multiple modes are legal.

### Play-mode chooser

- Local `BoardModel` state (sibling to `modalCast` / `staged`), not an engine
  `pending_choice`.
- Docked aim chrome in the same family as `pending-mode-aim` /
  `modal-mode-aim` (e.g. `play-mode-aim`, buttons `play-mode-{i}`).
- Button order: `cast` / `play_land`, then `cycle`, then
  `activate_hand_ability` ordered by `ability_index`.
- Each button uses that action’s existing `label` (and cost presentation the
  action already exposes). Ability costs appear only on chooser buttons, not on
  the resting tile.
- Choosing a mode clears `playModePick` and feeds the chosen `ActionView` into
  the existing payment / targeting / modal-spell pipeline (card may remain
  staged if that pipeline needs it).
- Cancel (Escape / Cancel control / existing session-cancel rules): clear
  `playModePick`, clear staging / flight hide, return the card to hand. No
  intent submitted.

### Stale legality

While `playModePick` is open, snapshot updates may change the mode set:

- Drop buttons for actions no longer present.
- If exactly one mode remains, auto-run that mode (same as a one-mode play).
- If zero remain, cancel the session (card returns to hand; no intent).

If a mode is chosen but disappears before submit, follow the same stale-staged
cast path (clear session; no spurious intent).

### Components (implementation sketch)

- `client/app/board/html/actions.ts` — `modesForObject` (or equivalent); hand
  bar stops using `handExtras` for tiles (helper may remain for other callers or
  be removed if unused).
- `client/app/board/html/hand.ts` — one slot per card; playable/caption rules
  above.
- `planHandDrop` / `HandActionActivated` — branch on mode count before binding a
  single action.
- Prompts / overlays — docked play-mode aim + Cancel.
- Specs — update hand-and-zone-bar and prompts surface specs when implementing.

## Testing

- Unit: mode aggregation / ordering; auto path when `length === 1`; hand bar no
  longer emits extras tiles for multi-action objects.
- Scene: multi-action fixture (cast + two hand abilities, Valley Rannet shape)
  → one hand tile; play → `play-mode-aim` + stack park; pick a hand ability →
  continues that action; Cancel → card back in hand.
- Scene: only cycle legal → no chooser; Cycle caption; activate still works.
- Scene: cast + cycle both legal → one tile; chooser offers both.
- Assert outcomes (tile count, park/hide, chooser presence, cancel restore), not
  only testid presence ([interaction test policy](2026-07-22-client-interaction-test-policy-design.md)).

## Out of scope (recap)

Engine/proto/schema, GY/exile/command multi-action collapsing, changing
typecycling card scripts, modal spell mode UX beyond “cast is one play mode.”
